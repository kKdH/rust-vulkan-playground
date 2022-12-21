use proc_macro::TokenStream;
use std::fs::File;
use std::io::Write;
use std::os::linux::raw::ino_t;

use proc_macro2::{Ident, Literal};
use quote::quote;
use syn::{LitStr, Token};
use syn::parse::ParseStream;


struct ShaderSource {
    kind: ::shaderc::ShaderKind,
    source: String,
}

#[proc_macro]
pub fn shader(input: TokenStream) -> TokenStream {

    let shader = ::syn::parse::<ShaderSource>(input).unwrap();

    let binary = compile_shader(&shader);
    let binary_len = Literal::usize_unsuffixed(binary.len());

    let shader_fn = quote_shader_fn(&shader);

    let tokens = quote! {
        const bytes: [u8; #binary_len] = [#(#binary),*];
        #shader_fn
    };

    tokens.into()
}

fn quote_shader_fn(shader: &ShaderSource) -> ::proc_macro2::TokenStream {
    match shader.kind {
        ::shaderc::ShaderKind::Vertex => {
            quote! {
                pub fn shader() -> skyshard::graphics::vulkan::shaders::VertexShaderBinary {
                    <skyshard::graphics::vulkan::shaders::VertexShaderBinary>::new(&bytes)
                }
            }
        }
        ::shaderc::ShaderKind::Fragment => {
            quote! {
                pub fn shader() -> skyshard::graphics::vulkan::shaders::FragmentShaderBinary {
                    <skyshard::graphics::vulkan::shaders::FragmentShaderBinary>::new(&bytes)
                }
            }
        }
        ::shaderc::ShaderKind::Geometry => {
            quote! {
                pub fn shader() -> skyshard::graphics::vulkan::shaders::GeometryShaderBinary {
                    <skyshard::graphics::vulkan::shaders::GeometryShaderBinary>::new(&bytes)
                }
            }
        }
        _ => panic!("unsupported shader type")
    }
}

impl ::syn::parse::Parse for ShaderSource {

    fn parse(input: ParseStream) -> syn::Result<Self> {

        input.parse::<Ident>()?;
        input.parse::<Token![:]>()?;
        let kind = input
            .parse::<LitStr>()
            .map(|lit| lit.value())?;

        input.parse::<Token![,]>()?;
        input.parse::<Ident>()?;
        input.parse::<Token![:]>()?;
        let source = input
            .parse::<LitStr>()
            .map(|lit| lit.value())?;

        Ok(ShaderSource {
            kind: resolve_shader_kind(kind.as_str()),
            source
        })
    }
}

fn compile_shader(shader: &ShaderSource) -> Vec<u8> {

    let compiler = ::shaderc::Compiler::new()
        .expect("should initialize a shader compiler to compile source code into spir-v");
    let options = ::shaderc::CompileOptions::new()
        .expect("should initialize a compiler options object to compile source code into spir-v");
    let binary_result = compiler
        .compile_into_spirv(
            shader.source.as_str(),
            shader.kind,
            "shader.glsl",
            "main",
            Some(&options),
        )
        .expect("should compile shader source code into spir-v");

    Vec::from(binary_result.as_binary_u8())
}

fn resolve_shader_kind(kind: &str) -> ::shaderc::ShaderKind {
    match kind {
        "Vertex" => ::shaderc::ShaderKind::Vertex,
        "Fragment" => ::shaderc::ShaderKind::Fragment,
        "Geometry" => ::shaderc::ShaderKind::Geometry,
        "TessControl" => ::shaderc::ShaderKind::TessControl,
        "TessEvaluation" => ::shaderc::ShaderKind::TessEvaluation,
        _ => panic!("Unknown shader kind") // TODO: Don't panic
    }
}
