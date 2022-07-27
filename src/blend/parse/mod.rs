use std::collections::HashMap;
use std::error::Error;
use std::fmt::{Display, Formatter};
use std::num::NonZeroUsize;
use nom::error::ErrorKind;

use thiserror::Error;

use crate::blend::parse::input::Input;
use crate::blend::parse::parsers::parse_blend;

mod parsers;
mod input;

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

    pub fn dna(&self) -> &Dna {
        &self.dna
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

impl Dna {

    pub fn field_name_of(&self, field: &DnaField) -> &String {
        &self.field_names[field.name_index]
    }

    pub fn type_of<A>(&self, typed: A) -> Option<&DnaType>
    where A: HasDnaTypeIndex {
        self.types.get(typed.type_index())
    }

    pub fn struct_of(&self, block: &FileBlock) -> Option<&DnaStruct> {
        self.structs.get(block.dna)
    }
}

pub trait HasDnaTypeIndex {
    fn type_index(&self) -> usize;
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

impl HasDnaTypeIndex for &DnaStruct {
    fn type_index(&self) -> usize {
        self.type_index
    }
}

#[derive(Debug)]
pub struct DnaField {
    pub name_index: usize,
    pub type_index: usize,
}

impl HasDnaTypeIndex for &DnaField {
    fn type_index(&self) -> usize {
        self.type_index
    }
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
    /// Identifier for the end of the file header of a blend file.
    REND,
    TEST,
    GLOB,
    DATA,
    WM,
    IM,
    SN,
    WS,
    BR,
    /// Identifier for a [`FileBlock`] containing scene information.
    SC,
    PL,
    /// Identifier for a [`FileBlock`] containing object information.
    OB,
    GR,
    CA,
    LA,
    /// Identifier for a [`FileBlock`] containing mesh data.
    ME,
    WO,
    LS,
    /// Identifier for a [`FileBlock`] containing material data.
    MA,
    /// Identifier for the DNA block.
    DNA,
    /// Identifier for the end of a blend file.
    ENDB,
}

impl Display for Identifier {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        let text = match self {
            Identifier::Unknown { .. } => "Unknown",
            Identifier::REND => "REND",
            Identifier::TEST => "Test",
            Identifier::GLOB => "Glob",
            Identifier::DATA => "Data",
            Identifier::WM => "WM",
            Identifier::IM => "IM",
            Identifier::SN => "SN",
            Identifier::WS => "WS",
            Identifier::BR => "BR",
            Identifier::SC => "Scene",
            Identifier::PL => "PL",
            Identifier::OB => "Object",
            Identifier::GR => "BR",
            Identifier::CA => "CA",
            Identifier::LA => "LA",
            Identifier::ME => "Mesh",
            Identifier::WO => "WO",
            Identifier::LS => "LS",
            Identifier::MA => "Material",
            Identifier::DNA => "DNA",
            Identifier::ENDB => "ENDB",
        };
        write!(formatter, "{}", text)
    }
}

#[derive(Error, Debug)]
pub enum BlendParseError {

    #[error("Failed to parse header!")]
    ParseHeaderError,

    #[error("An error of kind '{kind}' occurred while parsing the dna!")]
    ParseDnaError { kind: String },

    #[error("Failed to parse dna, due to missing input!")]
    IncompleteDnaError,

    #[error("An error occurred parsing blend file!")]
    ParseError,
}

pub fn parse(blend: Data) -> Result<Blend, BlendParseError> {
    let input = Input::new(blend, None, None);
    parse_blend(input)
}