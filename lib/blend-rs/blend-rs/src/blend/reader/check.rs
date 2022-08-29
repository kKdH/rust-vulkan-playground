use blend_inspect_rs::{BlendFile, Endianness, HasDnaTypeIndex, Version};
use crate::blend::{GeneratedBlendStruct, ReadError};

type PointerSize = usize;

pub fn check_blend<A>(blend: &BlendFile) -> Result<(), ReadError>
where A: GeneratedBlendStruct {
    check_version(blend.header.version, A::BLEND_VERSION)?;
    check_pointer_size(blend.header.pointer_size.size(), A::BLEND_POINTER_SIZE)?;
    check_endianness(blend.header.endianness, A::BLEND_ENDIANNESS)
}

pub fn check_same_type<A, B>(blend: &BlendFile, expected: A, actual: B) -> Result<(), ReadError>
where A: HasDnaTypeIndex,
      B: HasDnaTypeIndex {

    let expected_type = blend.dna.find_type_of(expected)
        .expect("Failed to resolve DnaType!");
    let actual_type = blend.dna.find_type_of(actual)
        .expect("Failed to resolve DnaType!");
    if expected_type == actual_type {
        Ok(())
    }
    else {
        Err(ReadError::InvalidPointerTypeError {
            expected: Clone::clone(&expected_type.name),
            actual: Clone::clone(&actual_type.name)
        })
    }
}

fn check_version(expected: Version, actual: Version) -> Result<(), ReadError> {
    if expected == actual {
        Ok(())
    }
    else {
        Err(ReadError::VersionMismatchError {
            expected,
            actual,
        })
    }
}

fn check_pointer_size(expected: PointerSize, actual: PointerSize) -> Result<(), ReadError> {
    if expected == actual {
        Ok(())
    }
    else {
        Err(ReadError::PointerSizeMismatchError {
            expected,
            actual,
        })
    }
}

fn check_endianness(expected: Endianness, actual: Endianness) -> Result<(), ReadError> {
    if expected == actual {
        Ok(())
    }
    else {
        Err(ReadError::EndiannessMismatchError {
            expected,
            actual,
        })
    }
}
