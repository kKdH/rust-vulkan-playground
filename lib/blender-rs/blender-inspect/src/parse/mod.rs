use std::collections::HashMap;
use std::fmt::{Display, Formatter};
use std::num::NonZeroUsize;

use thiserror::Error;

use crate::parse::input::Input;
use crate::parse::parsers::parse_blend;

mod parsers;
mod input;

pub type Location = usize;

#[derive(Debug)]
pub struct BlendFile {
    pub header: FileHeader,
    pub blocks: Vec<FileBlock>,
    pub dna: Dna,
    address_table: AddressTable,
}

impl BlendFile {

    pub fn look_up<T>(&self, address: T) -> Option<&FileBlock>
    where T: AddressLike {
        self.address_table
            .get(&address.address())
            .map(|index| self.blocks.get(*index))
            .flatten()
    }
}

pub type Address = NonZeroUsize;
pub type AddressTable = HashMap<Address, usize>;

pub trait AddressLike {
    fn address(&self) -> Address;
}

impl AddressLike for NonZeroUsize {
    fn address(&self) -> Address {
        *self
    }
}

impl AddressLike for &NonZeroUsize {
    fn address(&self) -> Address {
        **self
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

#[derive(Debug)]
pub struct Dna {
    pub field_names: Vec<String>,
    pub types: Vec<DnaType>,
    pub structs: Vec<DnaStruct>,
    pub pointer_size: usize,
}

impl Dna {

    pub fn find_field_name_of(&self, field: &DnaField) -> Option<&String> {
        self.field_names.get(field.name_index)
    }

    pub fn find_type_of<A>(&self, typed: A) -> Option<&DnaType>
    where A: HasDnaTypeIndex {
        self.types.get(typed.type_index(self))
    }

    pub fn find_struct_of(&self, block: &FileBlock) -> Option<&DnaStruct> {
        self.structs.get(block.sdna)
    }

    pub fn find_struct_by_name(&self, name: &str) -> Option<&DnaStruct> {
        self.structs.iter().find(|dna_struct| {
            self.find_type_of(*dna_struct)
                .map(|dna_type| name == dna_type.name)
                .unwrap_or(false)
        })
    }
}

#[derive(Debug)]
pub struct DnaType {
    pub name: String,
    pub size: usize,
}

impl DnaType {

    pub fn new(name: &'static str, size: usize) -> DnaType {
        DnaType {
            name: String::from(name),
            size
        }
    }
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

pub trait HasDnaTypeIndex {
    fn type_index(&self, dna: &Dna) -> usize;
}

impl HasDnaTypeIndex for &DnaField {
    fn type_index(&self, _: &Dna) -> usize {
        self.type_index
    }
}

impl HasDnaTypeIndex for &DnaStruct {
    fn type_index(&self, _: &Dna) -> usize {
        self.type_index
    }
}

impl HasDnaTypeIndex for &FileBlock {
    fn type_index(&self, dna: &Dna) -> usize {
        dna.find_struct_of(self)
           .expect("Could not determine struct of FileBlock!")
           .type_index
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
    pub major: char,
    pub minor: char,
    pub patch: char,
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
    pub sdna: usize,
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

pub fn parse(blend: &[u8]) -> Result<BlendFile, BlendParseError> {
    let input = Input::new(blend, None, None);
    parse_blend(input).map(|(header, blocks, dna)| {
        let address_table: AddressTable = blocks.iter()
            .enumerate()
            .filter_map(|(index, block)| match block.address {
                None => None,
                Some(address) => Some((address, index))
            })
            .collect();
        BlendFile {
            header,
            blocks,
            dna,
            address_table
        }
    })
}
