use std::fs::File;
use std::io::Write;
use std::ops::Deref;

use proc_macro2::{Literal, TokenStream};
use quote::{format_ident, quote};

use blend_inspect_rs::{Blend, Endianness, inspect, Struct, Type};
use itertools::Itertools;


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

    let quoted_structs: Vec<TokenStream> = blend.structs()
        .sorted_by(|a, b| Ord::cmp(a.name(), b.name()))
        .map(|structure| {
            let name = format_ident!("{}", structure.name());
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
                    let pointer_size = Literal::usize_unsuffixed(blend.pointer_size());
                    quote! {
                        impl DoubleLinked<Pointer<#name, #pointer_size>, #pointer_size> for #name {
                            fn next(&self) -> &Pointer<Self, #pointer_size> {
                                &self.next
                            }
                            fn prev(&self) -> &Pointer<Self, #pointer_size> {
                                &self.prev
                            }
                        }
                    }
                }
                else {
                    quote!()
                }
            };
            let major_version = Literal::character(blend.version().major);
            let minor_version = Literal::character(blend.version().minor);
            let patch_version = Literal::character(blend.version().patch);
            let pointer_size = Literal::usize_unsuffixed(blend.pointer_size());
            let endianness = match blend.endianness() {
                Endianness::Little => quote!(Endianness::Little),
                Endianness::Big => quote!(Endianness::Big),
            };
            let struct_name = Literal::string(structure.name());
            let struct_index = Literal::usize_unsuffixed(structure.struct_index());
            let struct_type_index = Literal::usize_unsuffixed(structure.struct_type_index());
            quote! {
                #[repr(C, packed(4))]
                pub struct #name {
                    #(#fields),*
                }
                impl GeneratedBlendStruct for #name {
                    const BLEND_VERSION: Version = Version::new(#major_version, #minor_version, #patch_version);
                    const BLEND_POINTER_SIZE: usize = #pointer_size;
                    const BLEND_ENDIANNESS: Endianness = #endianness;
                    const STRUCT_NAME: &'static str = #struct_name;
                    const STRUCT_INDEX: usize = #struct_index;
                    const STRUCT_TYPE_INDEX: usize = #struct_type_index;
                }
                #double_linked
            }
        })
        .collect();

    let primitive_types_verifications = quote_primitive_types_verifications();
    let struct_verifications = quote_struct_verifications(&blend);

    let code = quote! {
        pub mod #module_name_ident {
            #![allow(non_snake_case)]
            #![allow(dead_code)]

            use crate::blend::{Function, GeneratedBlendStruct, Pointer, Version, Endianness, Void};
            use crate::blend::traverse::{DoubleLinked};

            #(#quoted_structs)*

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

fn quote_type(ty: &Type) -> TokenStream {
    match ty {
        Type::Char => {
            let ident = format_ident!("{}", "i8");
            quote!(#ident)
        },
        Type::UChar => {
            let ident = format_ident!("{}", "u8");
            quote! {
                #ident
            }
        },
        Type::Short => {
            let ident = format_ident!("{}", "i16");
            quote! {
                #ident
            }
        }
        Type::UShort => {
            let ident = format_ident!("{}", "u16");
            quote!(#ident)
        }
        Type::Int => {
            let ident = format_ident!("{}", "i32");
            quote!(#ident)
        }
        Type::Int8 => {
            let ident = format_ident!("{}", "i8");
            quote!(#ident)
        }
        Type::Int64 => {
            let ident = format_ident!("{}", "i64");
            quote!(#ident)
        }
        Type::UInt64 => {
            let ident = format_ident!("{}", "u64");
            quote!(#ident)
        }
        Type::Long => {
            let ident = format_ident!("{}", "i32");
            quote!(#ident)
        }
        Type::ULong => {
            let ident = format_ident!("{}", "u32");
            quote!(#ident)
        }
        Type::Float => {
            let ident = format_ident!("{}", "f32");
            quote!(#ident)
        }
        Type::Double => {
            let ident = format_ident!("{}", "f64");
            quote!(#ident)
        }
        Type::Void => quote!(Void),
        Type::Struct { name, size: _size } => {
            let name = format_ident!("{}", name);
            quote!(#name)
        },
        Type::Pointer { base_type, size } => {
            let size = Literal::usize_unsuffixed(*size);
            let ty = quote_type(base_type);
            quote! {
                Pointer<#ty, #size>
            }
        }
        Type::Array { base_type, length } => {
            let size = Literal::usize_unsuffixed(*length);
            let ty = quote_type(base_type);
            quote! {
                [#ty; #size]
            }
        }
        Type::Function { size } => {
            let size = Literal::usize_unsuffixed(*size );
            quote! {
                Function<#size>
            }
        },
        Type::Special { .. } => quote!(()),
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
