
use core::result::Result;
use std::slice::Iter;

use thiserror::Error;

mod analyse;
mod parse;
mod reader;

use crate::parse::{Dna, FileBlock, FileHeader, PointerSize, Version};


pub use crate::analyse::{Struct, Structure, Type, Mode};
pub use crate::analyse::analyse;
pub use crate::parse::parse;


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

    pub fn structs(&self) -> Iter<'_, Struct> {
        self.structure.structs()
    }

    pub fn pointer_size(&self) -> usize {
        match self.header.pointer_size {
            PointerSize::Pointer4Bytes => 4,
            PointerSize::Pointer8Bytes => 8
        }
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

pub fn inspect<A>(source: A) -> Result<Blend, BlendError>
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
