
use core::result::Result;
use std::slice::Iter;

use thiserror::Error;

mod analyse;
mod parse;

pub use crate::analyse::{Struct, Structure, Type, Mode};
pub use crate::analyse::analyse;
pub use crate::parse::{BlendFile, Dna, DnaType, DnaStruct, DnaField, FileBlock, FileHeader, Identifier, PointerSize, Version, Endianness, Address, AddressLike, AddressTable, HasDnaTypeIndex};
pub use crate::parse::parse;


pub type Data<'a> = &'a[u8];

#[derive(Debug)]
pub struct Blend {
    blend_file: BlendFile,
    structure: Structure,
}

impl Blend {

    pub fn blocks(&self) -> Iter<'_, FileBlock> {
        self.blend_file.blocks.iter()
    }

    pub fn structs(&self) -> Iter<'_, Struct> {
        self.structure.structs()
    }

    pub fn version(&self) -> &Version {
        &self.blend_file.header.version
    }

    pub fn pointer_size(&self) -> usize {
        match self.blend_file.header.pointer_size {
            PointerSize::Pointer4Bytes => 4,
            PointerSize::Pointer8Bytes => 8
        }
    }

    pub fn endianness(&self) -> &Endianness {
        &self.blend_file.header.endianness
    }

    pub fn find_struct_by_name(&self, name: &str) -> Option<&Struct> {
        self.structure.find_struct_by_name(name)
    }
}

pub trait BlendSource<'a> {
    fn data(&self) -> Data<'a>;
}

impl <'a> BlendSource<'a> for &'a[u8] {
    fn data(&self) -> Data<'a> {
        self
    }
}

impl <'a> BlendSource<'a> for &'a Vec<u8> {
    fn data(&self) -> Data<'a> {
        &self[..]
    }
}

#[derive(Error, Debug)]
#[error("Failed to read blender data! {message}")]
pub struct BlendError {

    message: String,

    #[source]
    cause: Box<dyn std::error::Error>,
}

pub fn inspect<'a, A>(source: A) -> Result<Blend, BlendError>
where A: BlendSource<'a> {
    let blend_file = parse(source)
        .map_err(|cause| {
            BlendError {
                message: String::from("Could not parse header, blocks and dna!"),
                cause: Box::new(cause)
            }
        })?;
    let structure = analyse(&blend_file, Mode::All)
        .map_err(|cause| {
            BlendError {
                message: String::from("Could not analyse the structure of the blender data!"),
                cause: Box::new(cause)
            }
        })?;
    Ok(Blend {
        blend_file,
        structure
    })
}
