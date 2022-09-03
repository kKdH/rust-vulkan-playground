use std::fs::File;
use std::io::Write;
use std::ops::Deref;
use std::env;
use proc_macro2::{Literal, TokenStream};
use quote::{format_ident, quote};

use blend_inspect_rs::{Blend, Endianness, inspect, Struct, Type};
use itertools::Itertools;

#[proc_macro]
pub fn blend_use(item: proc_macro::TokenStream) -> proc_macro::TokenStream {


    let out_dir = env::var("OUT_DIR").unwrap();
    let include_lit = Literal::string(format!("{}/blender3_3.rs", out_dir).as_str());

    blend_bindgen_rs::generate("/home/elmar/Projects/rust-vulkan-playground/lib/blend-rs/blend-rs/gen/blender3_3.blend", &out_dir);

    let output: TokenStream = quote! {
        include!(#include_lit);
    };

    proc_macro::TokenStream::from(output)
}
