use std::env;
use std::fs::File;
use std::io::Write;

use proc_macro2::{Literal, TokenStream};
use quote::{format_ident, quote};

use blender_inspect::{inspect, Type};
use itertools::Itertools;
use syn::Lit;


pub fn generate(source_file: &str, target_dir: &str) {

    let data = std::fs::read(source_file).unwrap();
    let blend = inspect(data).ok().unwrap();

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

    let mut code = quote! {
        #![allow(non_snake_case)]
        use crate::blender::*;

        #(#quoted_structs)*
    };

    let file_name = format!("{}blender{}_{}.rs", target_dir, blend.version().major, blend.version().minor);
    let mut generated_file = File::create(&file_name).expect(&file_name);

    write!(&mut generated_file, "{:#}", code).expect("Unable to write generated.ts");
}

fn quote_type(ty: &Type) -> TokenStream {
    match ty {
        Type::Char => {
            let ident = format_ident!("{}", "char");
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
            let ident = format_ident!("{}", "i64");
            quote!(#ident)
        }
        Type::ULong => {
            let ident = format_ident!("{}", "u64");
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
