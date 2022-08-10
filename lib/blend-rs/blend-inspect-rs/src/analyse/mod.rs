mod analysers;

use std::collections::HashMap;

use std::slice::Iter;

use thiserror::Error;

use crate::parse::{BlendFile, Endianness};

pub type Result<A> = ::core::result::Result<A, AnalyseError>;

#[derive(Debug)]
pub struct Structure {
    endianness: Endianness,
    structs: Vec<Struct>,
    struct_names: HashMap<String, usize>,
}

impl Structure {

    fn new(endianness: Endianness, structs: Vec<Struct>) -> Structure {
        let struct_names = structs.iter()
            .enumerate()
            .map(|(index, strct)| (String::from(&strct.name), index))
            .collect();
        Structure {
            endianness,
            structs,
            struct_names,
        }
    }

    pub fn structs(&self) -> Iter<'_, Struct> {
        self.structs.iter()
    }

    pub fn find_struct_by_name(&self, name: &str) -> Option<&Struct> {
        self.struct_names.get(name)
            .map(|index| self.structs.get(*index))
            .flatten()
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum Type {
    Char,
    UChar,
    Short,
    UShort,
    Int,
    Int8,
    Int64,
    UInt64,
    Long,
    ULong,
    Float,
    Double,
    Void,
    Struct { name: String, size: usize },
    Pointer { base_type: Box<Type>, size: usize },
    Array { base_type: Box<Type>, length: usize },
    Function { size: usize },

    /// This [`Type`] designates types which do not have a backing struct.
    /// If a type is recognized as a struct but its size is zero, then it
    /// becomes a [`Type::Special`].
    Special { name: String },
}

impl Type {

    const TYPE_NAME_CHAR: &'static str = "char";
    const TYPE_NAME_UCHAR: &'static str = "uchar";
    const TYPE_NAME_SHORT: &'static str = "short";
    const TYPE_NAME_USHORT: &'static str = "ushort";
    const TYPE_NAME_INT: &'static str = "int";
    const TYPE_NAME_INT8: &'static str = "int8_t";
    const TYPE_NAME_INT64: &'static str = "int64_t";
    const TYPE_NAME_UINT64: &'static str = "uint64_t";
    const TYPE_NAME_LONG: &'static str = "long";
    const TYPE_NAME_ULONG: &'static str = "ulong";
    const TYPE_NAME_FLOAT: &'static str = "float";
    const TYPE_NAME_DOUBLE: &'static str = "double";
    const TYPE_NAME_VOID: &'static str = "void";

    pub const PRIMITIVES: [Type; 12] = [
        Type::Char,
        Type::UChar,
        Type::Short,
        Type::UShort,
        Type::Int,
        Type::Int8,
        Type::Int64,
        Type::UInt64,
        Type::Long,
        Type::ULong,
        Type::Float,
        Type::Double,
    ];

    pub fn name(&self) -> Option<&'static str> {
        match self {
            Type::Char => Some(Type::TYPE_NAME_CHAR),
            Type::UChar => Some(Type::TYPE_NAME_UCHAR),
            Type::Short => Some(Type::TYPE_NAME_SHORT),
            Type::UShort => Some(Type::TYPE_NAME_USHORT),
            Type::Int => Some(Type::TYPE_NAME_INT),
            Type::Int8 => Some(Type::TYPE_NAME_INT8),
            Type::Int64 => Some(Type::TYPE_NAME_INT64),
            Type::UInt64 => Some(Type::TYPE_NAME_UINT64),
            Type::Long => Some(Type::TYPE_NAME_LONG),
            Type::ULong => Some(Type::TYPE_NAME_ULONG),
            Type::Float => Some(Type::TYPE_NAME_FLOAT),
            Type::Double => Some(Type::TYPE_NAME_DOUBLE),
            Type::Void => Some(Type::TYPE_NAME_VOID),
            Type::Struct { .. } => None,
            Type::Pointer { .. } => None,
            Type::Array { .. } => None,
            Type::Function { .. } => None,
            Type::Special { .. } => None,
        }
    }

    pub fn base_type(&self) -> &Type {
        match self {
            Type::Char => &Type::Char,
            Type::UChar => &Type::UChar,
            Type::Short => &Type::Short,
            Type::UShort => &Type::UShort,
            Type::Int => &Type::Int,
            Type::Int8 => &Type::Int8,
            Type::Int64 => &Type::Int64,
            Type::UInt64 => &Type::UInt64,
            Type::Long => &Type::Long,
            Type::ULong => &Type::ULong,
            Type::Float => &Type::Float,
            Type::Double => &Type::Double,
            Type::Void => &Type::Void,
            Type::Struct { .. } => &self,
            Type::Pointer { base_type, size: _size } => base_type.base_type(),
            Type::Array { base_type, length: _length } => base_type.base_type(),
            Type::Function { .. } => &self,
            Type::Special { .. } => &self
        }
    }

    pub fn size(&self) -> usize {
        Type::compute_size(self)
    }

    fn compute_size(data_type: &Self) -> usize {
        match data_type {
            Type::Char => 1,
            Type::UChar => 1,
            Type::Short => 2,
            Type::UShort => 2,
            Type::Int => 4,
            Type::Int8 => 1,
            Type::Int64 => 8,
            Type::UInt64 => 8,
            Type::Long => 4,
            Type::ULong => 4,
            Type::Float => 4,
            Type::Double => 8,
            Type::Void => 0,
            Type::Struct { name: _, size } => *size,
            Type::Pointer { base_type: _, size } => *size,
            Type::Array { base_type, length } => length * Type::compute_size(base_type),
            Type::Function { size } => *size,
            Type::Special { .. } => 0,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Field {
    name: String,
    data_type: Type,
    offset: usize, //TODO: Remove offset. Not used anymore.
}

impl Field {

    pub fn name(&self) -> &str {
        self.name.as_str()
    }

    pub fn data_type(&self) -> &Type {
        &self.data_type
    }

    pub fn size(&self) -> usize {
        self.data_type.size()
    }

    pub fn offset(&self) -> usize {
        self.offset
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Struct {
    struct_index: usize,
    type_index: usize,
    name: String,
    fields: Vec<Field>,
    fields_by_name: HashMap<String, usize>,
    size: usize,
}

impl Struct {

    fn new(struct_index: usize, type_index: usize, name: String, fields: Vec<Field>, size: usize) -> Struct {
        let fields_by_name = fields.iter()
            .enumerate()
            .map(|(index, field)| (Clone::clone(&field.name), index))
            .collect::<HashMap<String, usize>>();
        Struct {
            struct_index,
            type_index,
            name,
            fields,
            fields_by_name,
            size,
        }
    }

    pub fn struct_index(&self) -> usize {
        self.struct_index
    }

    pub fn struct_type_index(&self) -> usize {
        self.type_index
    }

    pub fn name(&self) -> &str {
        self.name.as_str()
    }

    pub fn fields(&self) -> Iter<'_, Field> {
        self.fields.iter()
    }

    pub fn size(&self) -> usize {
        self.size
    }

    pub fn find_field_by_name(&self, name: &str) -> Option<&Field> {
        self.fields_by_name
            .get(name)
            .and_then(|index| self.fields.get(*index))
    }
}

pub enum Mode {
    /// Designates a mode in which all types of a [`Dna`] will be analysed.
    All,
    /// Designates a mode in which only used types and their transitive types will be analysed.
    RequiredOnly,
}

#[derive(Error, Debug, PartialEq)]
pub enum AnalyseError {

    #[error("The value '{value}' could not be parsed as field name!")]
    MalformedFieldName { value: String },

    #[error("A field name with '{index}' does not exist!")]
    InvalidFieldNameIndex { index: usize },

    #[error("A type with '{index}' does not exist!")]
    InvalidTypeIndex { index: usize },

    #[error("A struct with '{index}' does not exist!")]
    InvalidStructIndex { index: usize },

    #[error("Unknown type '{value}'!")]
    UnknownType { value: String },

    #[error("Unknown struct '{value}'!")]
    UnknownStruct { value: String },
}

pub fn analyse(blend_file: &BlendFile, mode: Mode) -> Result<Structure> {
    crate::analyse::analysers::analyse(
        &blend_file.header,
        &blend_file.blocks,
        &blend_file.dna,
        mode
    )
}
