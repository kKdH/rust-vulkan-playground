use std::fs::File;
use std::io::Write;
use std::ops::Deref;

use proc_macro2::{Ident, Literal, TokenStream};
use quote::{format_ident, quote};

use blend_inspect_rs::{Blend, Endianness, inspect, Struct, Type};
use itertools::Itertools;
use syn::__private::str;


pub fn generate(source_file: &str, target_dir: &str) -> String {

    let data = std::fs::read(source_file).unwrap();
    let blend = inspect(&data).ok().unwrap();

    let module_name = {
        let major = blend.version().major;
        let minor = blend.version().minor.to_digit(10).unwrap() * 10 + blend.version().patch.to_digit(10).unwrap();
        match blend.pointer_size() {
            4 => format!("blender{}_{}x86", major, minor),
            8 => format!("blender{}_{}", major, minor),
            _ => panic!("Illegal pointer size '{}'! Possible values are 4 and 8.", blend.pointer_size())
        }
    };

    let module_name_ident = format_ident!("{}", module_name);
    let file_name = format!("{}/{}.rs", target_dir, module_name);

    let blend_structs: Vec<TokenStream> = blend.structs()
        .sorted_by(|a, b| Ord::cmp(a.name(), b.name()))
        .map(|structure| {
            let name = format_ident!("{}", structure.name());
            let pointer_size = Literal::usize_unsuffixed(blend.pointer_size());
            let fields: Vec<TokenStream> = structure.fields()
                .map(|field| {
                    let name = match field.name() {
                        "type" => "type_",
                        "macro" => "macro_",
                        "match" => "match_",
                        "ref" => "ref_",
                        name => name,
                    };
                    let name = format_ident!("{}", name);
                    let ty = quote_type(field.data_type());
                    quote! {
                        pub #name: #ty
                    }
                })
                .collect();
            let double_linked = {
                if is_struct_double_linked(structure) {
                    quote! {
                        impl DoubleLinked<Pointer<#name>> for #name {
                            fn next(&self) -> &Pointer<Self> {
                                &self.next
                            }
                            fn prev(&self) -> &Pointer<Self> {
                                &self.prev
                            }
                        }
                    }
                }
                else {
                    quote!()
                }
            };
            let named = {
                if is_struct_named(structure) {
                    quote! {
                        impl Named for #name {
                            fn get_name(&self) -> &str {
                                self.name.to_name_str_unchecked()
                            }
                        }
                    }
                }
                else {
                    quote!()
                }
            };
            let generated_impl = quote_generated_impl(
                &blend,
                structure.name(),
                structure.struct_index(),
                structure.struct_type_index(),
                Vec::new(),
                false
            );
            let pointer_target_impl = quote_pointer_target_impl(structure.name());
            quote! {
                #[repr(C, packed(4))]
                pub struct #name {
                    #(#fields),*
                }
                #generated_impl
                #pointer_target_impl
                #double_linked
                #named
            }
        })
        .collect();

    let primitive_types_verifications = quote_primitive_types_verifications();
    let struct_verifications = quote_struct_verifications(&blend);
    let nothing_struct = quote_nothing_struct(&blend);
    let void_struct = quote_void_struct(&blend);
    let pointer_struct = quote_pointer_struct(&blend);
    let function_struct = quote_function_struct(&blend);

    let code = quote! {
        pub mod #module_name_ident {
            #![allow(non_snake_case)]
            #![allow(dead_code)]

            use crate::blend::{GeneratedBlendStruct, Version, Endianness, PointerLike, PointerTarget, NameLike};
            use crate::blend::traverse::DoubleLinked;
            use crate::blend::traverse::Named;
            use blend_inspect_rs::Address;
            use std::marker::PhantomData;

            #nothing_struct
            #void_struct
            #pointer_struct
            #function_struct

            #(#blend_structs)*

            #[cfg(test)]
            mod verifications {
                use super::*;
                #primitive_types_verifications
                #struct_verifications
            }
        }
    };

    let mut generated_file = File::create(&file_name).expect(&file_name);
    write!(&mut generated_file, "{:#}", code).expect("Unable to write generated.ts");

    file_name
}

fn quote_nothing_struct(blend: &Blend) -> TokenStream {
    let generated_impl = quote_generated_impl(
        blend,
        "Nothing",
        usize::MAX,
        usize::MAX,
        Vec::new(),
        true
    );
    quote! {
        #[derive(Debug, Copy, Clone)]
        pub struct Nothing;
        impl PointerTarget<Nothing> for Nothing {}
        #generated_impl
    }
}

fn quote_void_struct(blend: &Blend) -> TokenStream {
    let generated_impl = quote_generated_impl(
        blend,
        "Void",
        usize::MAX - 1,
        usize::MAX - 1,
        Vec::new(),
        true
    );
    quote! {
        #[derive(Debug, Copy, Clone)]
        pub struct Void;
        impl PointerTarget<Void> for Void {}
        #generated_impl
    }
}

fn quote_pointer_struct(blend: &Blend) -> TokenStream {
    let pointer_size = Literal::usize_unsuffixed(blend.pointer_size());
    let generated_impl_fields = quote_generated_impl_fields(
        blend,
        "Pointer",
        usize::MAX - 2,
        usize::MAX - 2,
        true
    );
    quote! {
        #[derive(Debug, Clone)]
        pub struct Pointer<T>
        where T: PointerTarget<T> {
            pub value: [u8; #pointer_size],
            phantom: PhantomData<T>
        }
        impl <T> Pointer<T>
        where T: PointerTarget<T> {
            pub fn new(value: [u8; #pointer_size]) -> Self {
                Pointer {
                    value,
                    phantom: Default::default()
                }
            }
        }
        impl <T> PointerLike<T> for Pointer<T>
        where T: PointerTarget<T> {
            type Pointer<A: PointerTarget<A>> = Pointer<A>;
            fn as_instance_of<B: PointerTarget<B>>(&self) -> Self::Pointer<B> {
                Pointer::new(self.value)
            }
            fn address(&self) -> Option<Address> {
                let result = self.value.iter().enumerate().fold(0usize, |result, (index, value)| {
                    result + ((*value as usize) << (8 * index))
                });
                Address::new(result)
            }
            fn is_valid(&self) -> bool {
                self.value.iter().sum::<u8>() > 0
            }
        }
        impl <T> PointerTarget<Pointer<T>> for Pointer<T>
        where T: PointerTarget<T> {

        }
        impl <T> GeneratedBlendStruct for Pointer<T>
        where T: PointerTarget<T> {
            #generated_impl_fields
        }
    }
}

fn quote_function_struct(blend: &Blend) -> TokenStream {
    let pointer_size = Literal::usize_unsuffixed(blend.pointer_size());
    let generated_impl = quote_generated_impl(
        blend,
        "Function",
        usize::MAX - 3,
        usize::MAX - 3,
        Vec::new(),
        true
    );
    quote! {
        #[derive(Debug, Copy, Clone)]
        pub struct Function {
            pub value: [u8; #pointer_size]
        }
        #generated_impl
    }
}

fn quote_generated_impl(blend: &Blend, struct_name: &str, struct_index: usize, type_index: usize, type_parameters: Vec<(&str, Option<&str>)>, is_synthetic: bool) -> TokenStream {
    let type_parameters_idents = type_parameters.iter()
        .map(|(name, _)| format_ident!("{}", name))
        .collect::<Vec<Ident>>();
    let name = {
        let name = format_ident!("{}", struct_name);
        if type_parameters_idents.is_empty() {
            quote!(#name)
        }
        else {
            quote!(#name<#(#type_parameters_idents),*>)
        }
    };
    let type_parameters = {
        if type_parameters.is_empty() {
            quote!()
        }
        else {
            let type_parameters = type_parameters.iter()
                .map(|(name, bounds)| {
                    let name = format_ident!("{}", name);
                    let bounds = bounds
                        .map(|bounds| {
                            let bounds = format_ident!("{}", bounds);
                            quote!{: #bounds}
                        })
                        .unwrap_or(quote!());
                    quote! {#name #bounds}
                })
                .collect::<Vec<TokenStream>>();
            quote!(<#(#type_parameters),*>)
        }
    };
    let fields = quote_generated_impl_fields(blend, struct_name, struct_index, type_index, is_synthetic);
    quote! {
        impl #type_parameters GeneratedBlendStruct for #name {
            #fields
        }
    }
}

fn quote_generated_impl_fields(blend: &Blend, struct_name: &str, struct_index: usize, type_index: usize, is_synthetic: bool) -> TokenStream {
    let major_version = Literal::character(blend.version().major);
    let minor_version = Literal::character(blend.version().minor);
    let patch_version = Literal::character(blend.version().patch);
    let struct_name_literal = Literal::string(struct_name);
    let struct_index_literal = Literal::usize_unsuffixed(struct_index);
    let type_index_literal = Literal::usize_unsuffixed(type_index);
    let endianness = quote_endianness(blend.endianness());
    let pointer_size_literal = Literal::usize_unsuffixed(blend.pointer_size());
    let is_synthetic_ident = format_ident!("{}", is_synthetic);
    quote! {
        const BLEND_VERSION: Version = Version::new(#major_version, #minor_version, #patch_version);
        const BLEND_POINTER_SIZE: usize = #pointer_size_literal;
        const BLEND_ENDIANNESS: Endianness = #endianness;
        const STRUCT_NAME: &'static str = #struct_name_literal;
        const STRUCT_INDEX: usize = #struct_index_literal;
        const STRUCT_TYPE_INDEX: usize = #type_index_literal;
        const IS_SYNTHETIC: bool = #is_synthetic_ident;
    }
}


fn quote_pointer_target_impl(struct_name: &str) -> TokenStream {
    let name = format_ident!("{}", struct_name);
    quote! {
        impl PointerTarget<#name> for #name {}
    }
}

fn quote_type(ty: &Type) -> TokenStream {
    match ty {
        Type::Char => {
            let ident = primitive_type_ident(ty);
            quote!(#ident)
        },
        Type::UChar => {
            let ident = primitive_type_ident(ty);
            quote!(#ident)
        },
        Type::Short => {
            let ident = primitive_type_ident(ty);
            quote!(#ident)
        }
        Type::UShort => {
            let ident = primitive_type_ident(ty);
            quote!(#ident)
        }
        Type::Int => {
            let ident = primitive_type_ident(ty);
            quote!(#ident)
        }
        Type::Int8 => {
            let ident = primitive_type_ident(ty);
            quote!(#ident)
        }
        Type::Int64 => {
            let ident = primitive_type_ident(ty);
            quote!(#ident)
        }
        Type::UInt64 => {
            let ident = primitive_type_ident(ty);
            quote!(#ident)
        }
        Type::Long => {
            let ident = primitive_type_ident(ty);
            quote!(#ident)
        }
        Type::ULong => {
            let ident = primitive_type_ident(ty);
            quote!(#ident)
        }
        Type::Float => {
            let ident = primitive_type_ident(ty);
            quote!(#ident)
        }
        Type::Double => {
            let ident = primitive_type_ident(ty);
            quote!(#ident)
        }
        Type::Void => quote!(Void),
        Type::Struct { name, size: _size } => {
            let name = format_ident!("{}", name);
            quote!(#name)
        },
        Type::Pointer { base_type, size: _size } => {
            let ty = quote_type(base_type);
            quote! {
                Pointer<#ty>
            }
        }
        Type::Array { base_type, length } => {
            let size = Literal::usize_unsuffixed(*length);
            let ty = quote_type(base_type);
            quote! {
                [#ty; #size]
            }
        }
        Type::Function { size: _size } => {
            quote! {
                Function
            }
        },
        Type::Special { .. } => quote!(Nothing),
    }
}

fn quote_endianness(endianness: &Endianness) -> TokenStream {
    match endianness {
        Endianness::Little => quote!(Endianness::Little),
        Endianness::Big => quote!(Endianness::Big),
    }
}

fn quote_primitive_types_verifications() -> TokenStream {
    let assertions: Vec<TokenStream> = Type::PRIMITIVES.iter()
        .map(|ty| {
            let expectation_literal = Literal::usize_unsuffixed(ty.size());
            let type_ident = quote_type(ty);
            quote! {
                assert_eq!(std::mem::size_of::<#type_ident>(), #expectation_literal);
            }
        })
        .collect();
    quote! {
        #[test]
        fn verify_primitive_types_size() {
            #(#assertions);*
        }
    }
}

fn quote_struct_verifications(blend: &Blend) -> TokenStream {
    let verifications = blend.structs()
        .map(|structure| {
            let function_name = format_ident!("verify_{}_size", structure.name());
            let type_ident = format_ident!("{}", structure.name());
            let expected_size = Literal::usize_unsuffixed(structure.size());
            quote! {
                #[test]
                fn #function_name() {
                    assert_eq!(std::mem::size_of::<#type_ident>(), #expected_size);
                }
            }
        });
    quote! {
        #(#verifications)*
    }
}

fn primitive_type_ident(ty: &Type) -> Ident {
    match ty {
        Type::Char => format_ident!("{}", "i8"),
        Type::UChar => format_ident!("{}", "u8"),
        Type::Short => format_ident!("{}", "i16"),
        Type::UShort => format_ident!("{}", "u16"),
        Type::Int => format_ident!("{}", "i32"),
        Type::Int8 => format_ident!("{}", "i8"),
        Type::Int64 => format_ident!("{}", "i64"),
        Type::UInt64 => format_ident!("{}", "u64"),
        Type::Long => format_ident!("{}", "i32"),
        Type::ULong => format_ident!("{}", "u32"),
        Type::Float => format_ident!("{}", "f32"),
        Type::Double => format_ident!("{}", "f64"),
        _ => panic!("not a primitive type")
    }
}

fn is_struct_double_linked(structure: &Struct) -> bool {
    let (has_next, has_prev) = structure.fields().fold((false, false), |(has_next, has_prev), field| {
        match field.data_type() {
            Type::Pointer { base_type, size: _pointer_size } => {
                match base_type.deref() {
                    Type::Struct { name, size: _struct_size } => {
                        if name == structure.name() {
                            match field.name() {
                                "next" => (true, has_prev),
                                "prev" => (has_next, true),
                                _ => (has_next, has_prev)
                            }
                        } else {
                            (has_next, has_prev)
                        }
                    }
                    _ => (has_next, has_prev)
                }
            }
            _ => (has_next, has_prev)
        }
    });
    has_next && has_prev
}

fn is_struct_named(structure: &Struct) -> bool {
    structure.fields().any(|field| {
        field.name() == "name" && match field.data_type() {
            Type::Array { base_type, length: _length } => {
                match base_type.deref() {
                    Type::Char => true,
                    _ => false,
                }
            }
            _ => false,
        }
    })
}
