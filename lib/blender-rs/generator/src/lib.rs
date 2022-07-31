
use std::fs::File;
use std::io::Write;


use proc_macro2::{Literal, TokenStream};
use quote::{format_ident, quote};

use blender_inspect::{Blend, inspect, Type};
use itertools::Itertools;



pub fn generate(source_file: &str, target_dir: &str) {

    let data = std::fs::read(source_file).unwrap();
    let blend = inspect(data).ok().unwrap();

    let module_name = format!("blender{}_{}", blend.version().major, blend.version().minor);
    let module_name_ident = format_ident!("{}", module_name);
    let file_name = format!("{}{}.rs", target_dir, module_name);

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
            quote! {
                #[repr(C)]
                pub struct #name {
                    #(#fields),*
                }
            }
        })
        .collect();

    let primitive_types_verifications = quote_primitive_types_verifications();
    let struct_verifications = quote_struct_verifications(&blend);

    let code = quote! {
        #![allow(non_snake_case)]
        use crate::blender::*;

        #(#quoted_structs)*

        #[cfg(test)]
        mod verifications {
            use crate::#module_name_ident::*;
            #primitive_types_verifications
            #struct_verifications
        }
    };

    let mut generated_file = File::create(&file_name).expect(&file_name);
    write!(&mut generated_file, "{:#}", code).expect("Unable to write generated.ts");
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
