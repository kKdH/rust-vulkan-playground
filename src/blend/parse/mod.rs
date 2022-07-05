mod parsers;
mod input;

use std::collections::HashMap;
use std::num::NonZeroUsize;
use thiserror::Error;
use crate::blend::parse::input::Input;
use crate::blend::parse::parsers::parse_blend;

pub type Data<'a> = &'a [u8];
pub type Location = usize;

#[derive(Debug)]
pub struct Blend {
    header: FileHeader,
    blocks: Vec<FileBlock>,
    blocks_by_identifier: HashMap<Identifier, Vec<FileBlock>>,
    blocks_by_address: HashMap<NonZeroUsize, Vec<FileBlock>>,
    dna: Dna,
}

impl Blend {

    pub fn header(&self) -> &FileHeader {
        &self.header
    }

    pub fn blocks_by_address(&self, address: NonZeroUsize) -> Option<&Vec<FileBlock>> {
        self.blocks_by_address.get(&address)
    }

    pub fn blocks_by_identifier(&self, identifier: Identifier) -> Option<&Vec<FileBlock>> {
        self.blocks_by_identifier.get(&identifier)
    }
}

#[derive(Debug)]
pub struct Dna {
    pub field_names: Vec<String>,
    pub types: Vec<DnaType>,
    pub structs: Vec<DnaStruct>,
}

#[derive(Debug)]
pub struct DnaType {
    pub name: String,
    pub size: usize,
}

#[derive(Debug)]
pub struct DnaStruct {
    pub type_index: usize,
    pub fields: Vec<DnaField>,
}

#[derive(Debug)]
pub struct DnaField {
    pub name_index: usize,
    pub type_index: usize,
}

#[derive(Debug, Copy, Clone, PartialEq)]
pub struct FileHeader {
    pub pointer_size: PointerSize,
    pub endianness: Endianness,
    pub version: Version,
}

#[derive(Debug, Copy, Clone, PartialEq)]
pub enum PointerSize {
    Pointer4Bytes,
    Pointer8Bytes
}

impl PointerSize {
    fn size(&self) -> usize {
        match self {
            PointerSize::Pointer4Bytes => 4,
            PointerSize::Pointer8Bytes => 8,
        }
    }
}

#[derive(Debug, Copy, Clone, PartialEq)]
pub enum Endianness {
    Little,
    Big
}

#[derive(Debug, Copy, Clone, PartialEq)]
pub struct Version {
    major: char,
    minor: char,
    patch: char,
}

impl Version {
    fn new(major: char, minor: char, patch: char) -> Version {
        Version { major, minor, patch }
    }
}

#[derive(Debug, Copy, Clone)]
pub struct FileBlock {
    pub identifier: Identifier,
    pub length: usize,
    pub address: Option<NonZeroUsize>,
    pub dna: usize,
    pub count: usize,
    block_location: Location,
    data_location: Location,
}

impl FileBlock {

    pub fn block_location(&self) -> Location {
        self.block_location
    }

    pub fn data_location(&self) -> Location {
        self.data_location
    }
}

#[derive(Debug, Hash, Copy, Clone, Eq, PartialEq)]
pub enum Identifier {
    Unknown { code: [u8; 4] },
    REND,
    TEST,
    GLOB,
    DATA,
    WM,
    IM,
    SN,
    WS,
    BR,
    SC,
    PL,
    OB,
    GR,
    CA,
    LA,
    ME,
    WO,
    LS,
    MA,
    DNA,
    ENDB,
}

#[derive(Error, Debug)]
pub enum BlendParseError {

    #[error("Failed to parse header!")]
    ParseHeaderError,

    #[error("Failed to parse dna!")]
    ParseDnaError,

    #[error("An error occurred parsing blend file!")]
    ParseError,
}

pub fn parse(blend: Data) -> Result<Blend, BlendParseError> {
    let input = Input::new(blend, None, None);
    parse_blend(input)
}
