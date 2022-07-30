use core::result::Result;

use thiserror::Error;

use analyse::analyse;
use parse::parse;

use crate::blend::analyse::{Mode, Structure};
use crate::blend::parse::{Dna, FileBlock, FileHeader, Version};

mod analyse;
mod parse;
mod reader;
mod generated;

pub type Data<'a> = &'a[u8];

#[derive(Debug)]
pub struct Blend {
    header: FileHeader,
    blocks: Vec<FileBlock>,
    dna: Dna,
    structure: Structure,
}

impl Blend {
    pub fn version(&self) -> &Version {
        &self.header.version
    }
}

pub trait BlendSource {
    fn data(&self) -> Data;
}

impl BlendSource for &[u8] {
    fn data(&self) -> Data {
        self
    }
}

impl BlendSource for Vec<u8> {
    fn data(&self) -> Data {
        self.as_slice()
    }
}

#[derive(Error, Debug)]
#[error("Failed to read blender data! {message}")]
pub struct BlendError {

    message: String,

    #[source]
    cause: Box<dyn std::error::Error>,
}

pub fn read<A>(source: A) -> Result<Blend, BlendError>
where A: BlendSource {
    let (header, blocks, dna) = parse(source.data())
        .map_err(|cause| {
            BlendError {
                message: String::from("Could not parse header, blocks and dna!"),
                cause: Box::new(cause)
            }
        })?;
    let structure = analyse(&header, &blocks, &dna, Mode::All)
        .map_err(|cause| {
            BlendError {
                message: String::from("Could not analyse the structure of the blender data!"),
                cause: Box::new(cause)
            }
        })?;
    Ok(Blend { header, blocks, dna, structure })
}

#[cfg(test)]
mod test {
    use std::fs::File;
    use std::io::Write;
    use hamcrest2::{assert_that, equal_to, HamcrestMatcher, is};
    use itertools::Itertools;
    use quote::__private::{Ident, Literal, Span, TokenStream};
    use quote::{format_ident, quote};
    use crate::blend;

    use crate::blend::parse::{DnaStruct, DnaType, FileBlock, Identifier, PointerSize, Version};
    use crate::blend::{Blend, generated, read};
    use crate::blend::analyse::{Field, Type};
    use crate::blend::reader::Reader;

    #[test]
    fn test_read() {

        let data = std::fs::read("test/resources/cube.blend").unwrap();
        let blend = read(data).ok().unwrap();

        assert_that!(blend.version(), is(equal_to(&Version { major: '3', minor: '0', patch: '2' })));

        let objects = blend.blocks.iter()
            .filter(|block| block.identifier == Identifier::OB)
            .map(|block| blend.dna.find_type_of(block).unwrap())
            .collect::<Vec<&DnaType>>();

        println!("Objects: {:?}", objects);

        let quoted_structs: Vec<TokenStream> = blend.structure.structs
            .values()
            .sorted_by(|a, b| Ord::cmp(a.name(), b.name()))
            .map(|structure| {
                let name = format_ident!("{}", structure.name());
                let fields: Vec<TokenStream> = structure.fields()
                    .filter(|field| {
                        match field.data_type() {
                            Type::FunctionPointer => false,
                            _ => true,
                        }
                    })
                    .map(|field| {
                        let name = match field.name() {
                            "type" => "type_",
                            "macro" => "macro_",
                            "match" => "match_",
                            name => name,
                        };
                        let x: ();
                        let name = format_ident!("{}", name);
                        let ty = match field.data_type() {
                            Type::Char => {
                                let ident = format_ident!("{}", "char");
                                quote! {
                                    #ident
                                }
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
                                quote! {
                                    #ident
                                }
                            }
                            Type::Int => {
                                let ident = format_ident!("{}", "i32");
                                quote! {
                                    #ident
                                }
                            }
                            Type::Int8 => {
                                let ident = format_ident!("{}", "i8");
                                quote! {
                                    #ident
                                }
                            }
                            Type::Int64 => {
                                let ident = format_ident!("{}", "i64");
                                quote! {
                                    #ident
                                }
                            }
                            Type::UInt64 => {
                                let ident = format_ident!("{}", "u64");
                                quote! {
                                    #ident
                                }
                            }
                            Type::Long => {
                                let ident = format_ident!("{}", "i64");
                                quote! {
                                    #ident
                                }
                            }
                            Type::ULong => {
                                let ident = format_ident!("{}", "u64");
                                quote! {
                                    #ident
                                }
                            }
                            Type::Float => {
                                let ident = format_ident!("{}", "f32");
                                quote! {
                                    #ident
                                }
                            }
                            Type::Double => {
                                let ident = format_ident!("{}", "f64");
                                quote! {
                                    #ident
                                }
                            }
                            Type::Void => quote!(Void),
                            Type::Struct { name, size: _size } => {
                                let name = format_ident!("{}", name);
                                quote! {
                                    #name
                                }
                            },
                            Type::Pointer { base_type, size } => {
                                let size = Literal::usize_suffixed(*size);
                                quote! {
                                    Pointer<usize, #size>
                                }
                            }
                            Type::Array { base_type, length } => {
                                let ident = format_ident!("{}", "usize");
                                quote! {
                                    #ident
                                }
                            }
                            Type::FunctionPointer => {
                                let ident = format_ident!("{}", "usize");
                                quote! {
                                    #ident
                                }
                            }
                            Type::Special { .. } => {
                                let ident = format_ident!("{}", "usize");
                                quote! {
                                    #ident
                                }
                            }
                        };
                        quote! {
                            #name: #ty
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
            use crate::blend::generated::*;

            #(#quoted_structs)*
        };

        let file_name = format!("src/blend/generated/blender{}_{}.rs", blend.version().major, blend.version().minor);
        let mut generated_file = File::create(&file_name).expect(&file_name);

        write!(&mut generated_file, "{:#}", code).expect("Unable to write generated.ts");

        // let reader = Reader::builder()
            // .structures("Mesh")
            // .build();

        // reader.read(blend, data)
    }

    fn fubar() {

        // let x = generated::blender3_0::Link {
        //     next: (),
        //     prev: ()
        // }
    }
}
