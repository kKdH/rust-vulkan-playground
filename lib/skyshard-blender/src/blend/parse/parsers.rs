use std::num::NonZeroUsize;

use nom::{Err, InputIter, InputLength, IResult, Slice};
use nom::branch::alt;
use nom::bytes::complete::{tag, take, take_until, take_until1};
use nom::combinator::map;
use nom::error::{context, ErrorKind, make_error};
use nom::multi::{count, length_count};
use nom::sequence::{pair, preceded, terminated, tuple};

use crate::blend::parse::{BlendParseError, Dna, DnaField, DnaStruct, DnaType, Endianness, FileBlock, FileHeader, Identifier, Location, PointerSize, Version};
use crate::blend::parse::input::Input;

/// Value: `BLENDER`
const BLENDER_TAG: [u8; 7] = [0x42, 0x4c, 0x45, 0x4e, 0x44, 0x45, 0x52];

/// Value: `-`
const POINTER_SIZE_32_BIT_TAG: [u8; 1] = [0x2d];

/// Value: `_`
const POINTER_SIZE_64_BIT_TAG: [u8; 1] = [0x5f];

/// Value: `v`
const ENDIANNESS_LITTLE_TAG: [u8; 1] = [0x76];

/// Value: `V`
const ENDIANNESS_BIG_TAG: [u8; 1] = [0x56];

/// Value: `NAME`
const DNA_FIELD_NAMES_TAG: [u8; 4] = [0x4e, 0x41, 0x4d, 0x45];

/// Value: `TYPE`
const DNA_TYPE_NAMES_TAG: [u8; 4] = [0x54, 0x59, 0x50, 0x45];

/// Value: `TLEN`
const DNA_TYPE_SIZES_TAG: [u8; 4] = [0x54, 0x4c, 0x45, 0x4e];

/// Value: `STRC`
const DNA_STRUCTS_TAG: [u8; 4] = [0x53, 0x54, 0x52, 0x43];

const FILE_BLOCK_IDENTIFIER_SIZE: usize = 4;
const FILE_BLOCK_LENGTH_SIZE: usize = 4;
const FILE_BLOCK_DNA_SIZE: usize = 4;
const FILE_BLOCK_COUNT_SIZE: usize = 4;

type Result<'a, A> = IResult<Input<'a>, A>;

pub fn parse_blend(input: Input) -> ::std::result::Result<(FileHeader, Vec<FileBlock>, Dna), BlendParseError> {
    match parse_file_header(input) {
        Ok((file_blocks_input, header)) => {
            match parse_file_blocks(file_blocks_input) {
                Ok((_, blocks)) => {
                    let dna = {
                        let block = blocks.iter()
                            .find(|file_block| Identifier::DNA == file_block.identifier)
                            .expect("Failed to get DNA block");
                        let (input, _) = Input::new(input.data, file_blocks_input.pointer_size, file_blocks_input.endianness)
                            .split(block.data_location);
                        match parse_dna(input) {
                            Ok((_, dna)) => Ok(dna),
                            Err(Err::Incomplete(_)) => {
                                ::std::result::Result::Err(BlendParseError::IncompleteDnaError)
                            }
                            Err(Err::Failure(cause)) => {
                                ::std::result::Result::Err(BlendParseError::ParseDnaError { kind: String::from(cause.code.description()) })
                            },
                            Err(Err::Error(cause)) => {
                                ::std::result::Result::Err(BlendParseError::ParseDnaError { kind: String::from(cause.code.description()) })
                            }
                        }
                    }?;
                    ::std::result::Result::Ok((header, blocks, dna))
                }
                Err(_) => ::std::result::Result::Err(BlendParseError::ParseError)
            }
        }
        Err(_) => ::std::result::Result::Err(BlendParseError::ParseHeaderError)
    }
}

fn parse_dna(input: Input) -> Result<Dna> {
    let (input, _) = parse_dna_id(input)?;
    let (input, field_names) = parse_dna_field_names(input)?;
    let (input, type_names) = parse_dna_type_names(input)?;
    let (input, type_sizes) = parse_dna_type_sizes(input, type_names.len())?;
    let (input, structs) =
        // (input, Vec::new());
        parse_dna_structs(input)?;
    let types: Vec<DnaType> = type_names.into_iter()
        .zip(type_sizes)
        .map(|(name, length)| {
            DnaType { name, size: length }
        })
        .collect();

    Ok((input, Dna {
        field_names,
        types,
        structs,
        pointer_size: input.pointer_size
            .map(|pointer_size| pointer_size.size())
            .unwrap_or(4),
    }))
}

fn parse_dna_id(input: Input) -> Result<[u8; 4]> {
    context(
        "dna.id",
        map(take(4usize), |input: Input| {
            [input.data[0], input.data[1], input.data[2], input.data[3]]
        })
    )(input)
}

fn parse_dna_field_names(input: Input) -> Result<Vec<String>> {
    context(
        "dna.names",
        preceded(
            tag(&DNA_FIELD_NAMES_TAG[..]),
            length_count(
                parse_u32,
                map(terminated(
                    take_until1(&[0x00][..]),
                    take(1usize)
                ), |parsed: Input| {
                    parsed.data.iter()
                        .map(|byte| *byte as char)
                        .collect::<String>()
                })
            )
        )
    )(input)
}

fn parse_dna_type_names(input: Input) -> Result<Vec<String>> {
    context(
        "dna.types",
        preceded(
            pair(
                take_until(&DNA_TYPE_NAMES_TAG[..]),
                take(4usize)
            ),
            length_count(
                parse_u32,
                map(terminated(
                    take_until1(&[0x00][..]),
                    take(1usize)
                ), |parsed: Input| {
                    parsed.data.iter()
                        .map(|byte| *byte as char)
                        .collect::<String>()
                })
            )
        )
    )(input)
}

fn parse_dna_type_sizes(input: Input, types_count: usize) -> Result<Vec<usize>> {
    context(
        "dna.types-length",
        preceded(
            pair(
                take_until1(&DNA_TYPE_SIZES_TAG[..]),
                take(4usize)
            ),
            count(
                map(parse_u16,|length| length as usize),
                types_count
            )
        )
    )(input)
}

fn parse_dna_structs(input: Input) -> Result<Vec<DnaStruct>> {
    context(
        "dna.structs",
        preceded(
            pair(
                take_until(&DNA_STRUCTS_TAG[..]), // FIXME: take_until1 fails, why? Maybe input splitting or another mechanism is buggy.
                take(4usize)
            ),
            length_count(
                parse_u32,
                map(
                    pair(
                        parse_u16,
                        length_count(
                            parse_u16,
                            pair(
                                parse_u16,
                                parse_u16
                            )
                        )
                    ),
                    |(struct_type_index, fields)| {
                        DnaStruct {
                            type_index: struct_type_index as usize,
                            fields: fields.into_iter().map(|(type_index, name_index)| {
                                DnaField {
                                    name_index: name_index as usize,
                                    type_index: type_index as usize
                                }
                            }).collect()
                        }
                    }
                )
            )
        )
    )(input)
}

fn parse_file_header(input: Input) -> Result<FileHeader> {
    let parse_file_header = preceded(
        tag(&BLENDER_TAG[..]),
        tuple((
            parse_pointer_size,
            parse_endianness,
            parse_version
        ))
    );
    map(parse_file_header, |(pointer_size, endianness, version)| {
        FileHeader {
            pointer_size,
            endianness,
            version,
        }
    })(input)
        .map(|(input, result)| {
            (
                Input {
                    data: input.data,
                    position: input.position,
                    endianness: Some(result.endianness),
                    pointer_size: Some(result.pointer_size)
                },
                result
            )
        })
}

fn parse_file_blocks(input: Input) -> Result<Vec<FileBlock>> {
    let mut input = input;
    let mut file_blocks = Vec::new();
    loop {
        match parse_file_block(input) {
            Ok((remaining_input, file_block)) => {
                file_blocks.push(file_block);
                input = remaining_input;
            }
            Err(Err::Error(_)) => {
                return Ok((input, file_blocks));
            }
            Err(e) => {
                return Err(e);
            }
        }
    }
}

fn parse_file_block(input: Input) -> Result<FileBlock> {

    let block_location: Location = input.position;
    let data_location: Location = match input.pointer_size {
        None => Err(Err::Failure(make_error(input, ErrorKind::Fail))),
        Some(pointer_size) => {
            Ok(block_location
                + FILE_BLOCK_IDENTIFIER_SIZE
                + FILE_BLOCK_LENGTH_SIZE
                + pointer_size.size()
                + FILE_BLOCK_DNA_SIZE
                + FILE_BLOCK_COUNT_SIZE
                + 4
            )
        }
    }?;

    let (input, identifier) = parse_file_block_identifier(input)?;
    let (input, length) = parse_u32(input)?;
    let (input, address) = parse_pointer(input)?;
    let (input, _) = take(4usize)(input)?; //TODO: WTF? +4?
    let (input, sdna) = parse_u32(input)?;
    let (input, count) = parse_u32(input)?;
    let (input, _) = take(length)(input)?;

    Ok((
        input,
        FileBlock {
            identifier,
            length: length as usize,
            address: NonZeroUsize::new(address),
            sdna: sdna as usize,
            count: count as usize,
            block_location,
            data_location
        }
    ))
}

fn parse_pointer_size(input: Input) -> Result<PointerSize> {
    context(
        "pointer size",
        alt((
            map(tag(&POINTER_SIZE_32_BIT_TAG[..]), |_| PointerSize::Pointer4Bytes),
            map(tag(&POINTER_SIZE_64_BIT_TAG[..]), |_| PointerSize::Pointer8Bytes)
        ))
    )(input)
}

fn parse_endianness(input: Input) -> Result<Endianness> {
    context(
        "endianness",
        alt((
            map(tag(&ENDIANNESS_LITTLE_TAG[..]), |_| Endianness::Little),
            map(tag(&ENDIANNESS_BIG_TAG[..]), |_| Endianness::Big)
        ))
    )(input)
}

fn parse_version(input: Input) -> Result<Version> {
    context(
        "version",
        map(take(3usize), |parsed: Input| {
            Version {
                major: parsed.data[0] as char,
                minor: parsed.data[1] as char,
                patch: parsed.data[2] as char
            }
        })
    )(input)
}

fn parse_file_block_identifier(input: Input) -> Result<Identifier> {
    map(take(4usize), |parsed: Input| {
        match parsed.data {
            &[0x52, 0x45, 0x4e, 0x44] => Identifier::REND,
            &[0x54, 0x45, 0x53, 0x54] => Identifier::TEST,
            &[0x47, 0x4c, 0x4f, 0x42] => Identifier::GLOB,
            &[0x44, 0x41, 0x54, 0x41] => Identifier::DATA,
            &[0x57, 0x4d, 0x00, 0x00] => Identifier::WM,
            &[0x49, 0x4d, 0x00, 0x00] => Identifier::IM,
            &[0x53, 0x4e, 0x00, 0x00] => Identifier::SN,
            &[0x57, 0x53, 0x00, 0x00] => Identifier::WS,
            &[0x42, 0x52, 0x00, 0x00] => Identifier::BR,
            &[0x53, 0x43, 0x00, 0x00] => Identifier::SC,
            &[0x50, 0x4C, 0x00, 0x00] => Identifier::PL,
            &[0x4f, 0x42, 0x00, 0x00] => Identifier::OB,
            &[0x47, 0x52, 0x00, 0x00] => Identifier::GR,
            &[0x43, 0x41, 0x00, 0x00] => Identifier::CA,
            &[0x4c, 0x41, 0x00, 0x00] => Identifier::LA,
            &[0x4d, 0x45, 0x00, 0x00] => Identifier::ME,
            &[0x57, 0x4f, 0x00, 0x00] => Identifier::WO,
            &[0x4c, 0x53, 0x00, 0x00] => Identifier::LS,
            &[0x4d, 0x41, 0x00, 0x00] => Identifier::MA,
            &[0x44, 0x4e, 0x41, 0x31] => Identifier::DNA,
            &[0x45, 0x4e, 0x44, 0x42] => Identifier::ENDB,
            _ => Identifier::Unknown {
                code: [parsed.data[0], parsed.data[1], parsed.data[2], parsed.data[3]]
            },
        }
    })(input)
}

fn parse_pointer(input: Input) -> Result<usize> {
    match input.pointer_size {
        None => Err(Err::Failure(make_error(input, ErrorKind::Fail))),
        Some(PointerSize::Pointer4Bytes) => {
            parse_u32(input)
                .map(|(input, address)| (input, address as usize))
        }
        Some(PointerSize::Pointer8Bytes) => {
            parse_u64(input)
                .map(|(input, address)| (input, address as usize))
        }
    }
}

fn parse_u16(input: Input) -> Result<u16> {
    let bound: usize = 2;
    if input.data.input_len() < bound {
        Err(Err::Error(make_error(input, ErrorKind::Eof)))
    }
    else {
        let bytes = input.data.iter_indices().take(bound);
        let mut result = 0u16;
        match input.endianness {
            None => Err(Err::Failure(make_error(input, ErrorKind::Fail))),
            Some(Endianness::Little) => {
                for (index, byte) in  bytes {
                    result += (byte as u16) << (8 * index);
                }
                Ok((input.slice(bound..), result))
            }
            Some(Endianness::Big) => {
                for (_, byte) in bytes {
                    result = (result << 8) + byte as u16;
                }
                Ok((input.slice(bound..), result))
            }
        }
    }
}

fn parse_u32(input: Input) -> Result<u32> {
    let bound: usize = 4;
    if input.data.input_len() < bound {
        Err(Err::Error(make_error(input, ErrorKind::Eof)))
    }
    else {
        let bytes = input.data.iter_indices().take(bound);
        let mut result = 0u32;
        match input.endianness {
            None => Err(Err::Failure(make_error(input, ErrorKind::Fail))),
            Some(Endianness::Little) => {
                for (index, byte) in  bytes {
                    result += (byte as u32) << (8 * index);
                }
                Ok((input.slice(bound..), result))
            }
            Some(Endianness::Big) => {
                for (_, byte) in bytes {
                    result = (result << 8) + byte as u32;
                }
                Ok((input.slice(bound..), result))
            }
        }
    }
}

fn parse_u64(input: Input) -> Result<u64> {
    let bound: usize = 8;
    if input.data.input_len() < bound {
        Err(Err::Error(make_error(input, ErrorKind::Eof)))
    }
    else {
        let bytes = input.data.iter_indices().take(bound);
        let mut result = 0u64;
        match input.endianness {
            None => Err(Err::Failure(make_error(input, ErrorKind::Fail))),
            Some(Endianness::Little) => {
                for (index, byte) in  bytes {
                    result += (byte as u64) << (8 * index);
                }
                Ok((input.slice(bound..), result))
            }
            Some(Endianness::Big) => {
                for (_, byte) in bytes {
                    result = (result << 8) + byte as u64;
                }
                Ok((input.slice(bound..), result))
            }
        }
    }
}

#[cfg(test)]
mod test {
    use hamcrest2::{assert_that, equal_to, HamcrestMatcher, is};
    use nom::Finish;

    use crate::blend::parse::{Endianness, FileBlock, Identifier, PointerSize, Version};
    use crate::blend::parse::input::Input;
    use crate::blend::parse::parsers::{ENDIANNESS_BIG_TAG, ENDIANNESS_LITTLE_TAG, parse_dna, parse_u16, POINTER_SIZE_32_BIT_TAG, POINTER_SIZE_64_BIT_TAG};
    use crate::blend::parse::parsers::{parse_blend, parse_endianness, parse_file_block, parse_file_header, parse_pointer, parse_pointer_size, parse_u32, parse_u64, parse_version};

    fn advance_to_file_blocks(input: Input) -> Input {
        input.split(12usize).0
    }

    fn advance_to_dna_block(input: Input) -> (Input, FileBlock) {
        let (mut input, mut file_block) = parse_file_block(input).finish().ok().unwrap();
        while file_block.identifier != Identifier::DNA {
            let (next, block) = parse_file_block(input).finish().ok().unwrap();
            input = next;
            file_block = block;
        }

        (input, file_block)
    }

    #[test]
    fn test_parse_blend() {
        let blend_file = std::fs::read("test/resources/cube.blend").unwrap();
        let input = Input::new(blend_file.as_slice(), None, None);
        let (_, file_blocks, _) = parse_blend(input).unwrap();

        assert_that!(file_blocks.len(), is(equal_to(1958)));
    }

    #[test]
    fn test_parse_file_header() {

        let blend_file = std::fs::read("test/resources/cube.blend").unwrap();
        let input = Input::new(blend_file.as_slice(), None, None);
        let (remaining, file_header) = parse_file_header(input)
            .finish()
            .ok()
            .unwrap();

        assert_that!(file_header.pointer_size, is(equal_to(PointerSize::Pointer4Bytes)));
        assert_that!(file_header.endianness, is(equal_to(Endianness::Little)));
        assert_that!(file_header.version, is(equal_to(Version { major: '3', minor: '0', patch: '2' })));
        assert_that!(remaining.position, is(equal_to(12)));
        assert_that!(remaining.endianness, is(equal_to(Some(Endianness::Little))));
        assert_that!(remaining.pointer_size, is(equal_to(Some(PointerSize::Pointer4Bytes))));
    }

    #[test]
    fn test_parse_file_blocks() {

        let blend_file = std::fs::read("test/resources/cube.blend").unwrap();
        let input = Input::new(blend_file.as_slice(), None, None);
        let (input, _) = parse_file_header(input)
            .finish()
            .ok()
            .unwrap();

        // REND block
        let (input, file_block) = parse_file_block(input).finish().ok().unwrap();

        assert_that!(file_block.identifier, is(equal_to(Identifier::REND)));
        assert_that!(file_block.length, is(equal_to(72)));
        assert_that!(file_block.address.unwrap().get(), is(equal_to(3165959472)));
        assert_that!(file_block.block_location(), is(equal_to(12)));
        assert_that!(file_block.sdna, is(equal_to(0)));
        assert_that!(file_block.count, is(equal_to(1)));
        assert_that!(input.position, is(equal_to(108)));

        // TEST block
        let (input, file_block) = parse_file_block(input).finish().ok().unwrap();

        assert_that!(file_block.identifier, is(equal_to(Identifier::TEST)));
        assert_that!(file_block.length, is(equal_to(65544)));
        assert_that!(file_block.address.unwrap().get(), is(equal_to(337039816)));
        assert_that!(file_block.block_location(), is(equal_to(108)));
        assert_that!(file_block.sdna, is(equal_to(0)));
        assert_that!(file_block.count, is(equal_to(1)));
        assert_that!(input.position, is(equal_to(65676)));

        // GLOB block
        let (input, file_block) = parse_file_block(input).finish().ok().unwrap();

        assert_that!(file_block.identifier, is(equal_to(Identifier::GLOB)));
        assert_that!(file_block.length, is(equal_to(1104)));
        assert_that!(file_block.address.unwrap().get(), is(equal_to(3165959472)));
        assert_that!(file_block.block_location(), is(equal_to(65676)));
        assert_that!(file_block.sdna, is(equal_to(313)));
        assert_that!(file_block.count, is(equal_to(1)));
        assert_that!(input.position, is(equal_to(66804)));

        // WM block
        let (input, file_block) = parse_file_block(input).finish().ok().unwrap();

        assert_that!(file_block.identifier, is(equal_to(Identifier::WM)));
        assert_that!(file_block.length, is(equal_to(1448)));
        assert_that!(file_block.address.unwrap().get(), is(equal_to(1089463304)));
        assert_that!(file_block.block_location(), is(equal_to(66804)));
        assert_that!(file_block.sdna, is(equal_to(630)));
        assert_that!(file_block.count, is(equal_to(1)));
        assert_that!(input.position, is(equal_to(68276)));

        // First DATA block
        let (input, file_block) = parse_file_block(input).finish().ok().unwrap();

        assert_that!(file_block.identifier, is(equal_to(Identifier::DATA)));
        assert_that!(file_block.length, is(equal_to(336)));
        assert_that!(file_block.address.unwrap().get(), is(equal_to(486540040)));
        assert_that!(file_block.block_location(), is(equal_to(68276)));
        assert_that!(file_block.sdna, is(equal_to(631)));
        assert_that!(file_block.count, is(equal_to(1)));
        assert_that!(input.position, is(equal_to(68636)));

        let (input, file_block) = advance_to_dna_block(input);

        assert_that!(file_block.identifier, is(equal_to(Identifier::DNA)));
        assert_that!(file_block.length, is(equal_to(116004)));
        assert_that!(file_block.address.unwrap().get(), is(equal_to(216471668)));
        assert_that!(file_block.block_location(), is(equal_to(718624)));
        assert_that!(file_block.sdna, is(equal_to(0)));
        assert_that!(file_block.count, is(equal_to(1)));
        assert_that!(input.position, is(equal_to(834652)));

        // ENDB block
        let (input, file_block) = parse_file_block(input).finish().ok().unwrap();

        assert_that!(file_block.identifier, is(equal_to(Identifier::ENDB)));
        assert_that!(file_block.length, is(equal_to(0)));
        assert_that!(file_block.address, is(equal_to(None)));
        assert_that!(file_block.block_location(), is(equal_to(834652)));
        assert_that!(file_block.sdna, is(equal_to(0)));
        assert_that!(file_block.count, is(equal_to(0)));
        assert_that!(input.position, is(equal_to(834676)));
    }

    #[test]
    fn test_parse_dna() {

        let blend_file = std::fs::read("test/resources/cube.blend").unwrap();
        let input = Input::new(blend_file.as_slice(), Some(PointerSize::Pointer4Bytes), Some(Endianness::Little));
        let (_, dna_block) = advance_to_dna_block(advance_to_file_blocks(input));
        let (input, _) = input.split(dna_block.data_location());
        let (_, dna) = parse_dna(input).unwrap();

        assert_that!(dna.field_names.len(), is(equal_to(4959)));
        assert_that!(dna.types.len(), is(equal_to(926)));
        assert_that!(dna.structs.len(), is(equal_to(798)));
    }

    #[test]
    fn test_parse_pointer_size_32bit() {
        let data = [POINTER_SIZE_32_BIT_TAG[0], 0xaa, 0xbb];
        let input = Input::new(&data, None, None);
        let (remaining, actual) = parse_pointer_size(input).finish().ok().unwrap();

        assert_that!(actual, is(equal_to(PointerSize::Pointer4Bytes)));
        assert_that!(remaining.data, is(equal_to(&[0xaa, 0xbb])));
    }

    #[test]
    fn test_parse_pointer_size_64bit() {
        let data = [POINTER_SIZE_64_BIT_TAG[0], 0xaa, 0xbb];
        let input = Input::new(&data, None, None);
        let (remaining, actual) = parse_pointer_size(input).finish().ok().unwrap();

        assert_that!(actual, is(equal_to(PointerSize::Pointer8Bytes)));
        assert_that!(remaining.data, is(equal_to(&[0xaa, 0xbb])));
        assert_that!(remaining.position, is(equal_to(1)))
    }

    #[test]
    fn test_parse_endianness_little() {
        let data = [ENDIANNESS_LITTLE_TAG[0], 0xaa, 0xbb];
        let input = Input::new(&data, None, None);
        let (remaining, actual) = parse_endianness(input).finish().ok().unwrap();

        assert_that!(actual, is(equal_to(Endianness::Little)));
        assert_that!(remaining.data, is(equal_to(&[0xaa, 0xbb])));
        assert_that!(remaining.position, is(equal_to(1)))
    }

    #[test]
    fn test_parse_endianness_big() {
        let data = [ENDIANNESS_BIG_TAG[0], 0xaa, 0xbb];
        let input = Input::new(&data, None, None);
        let (remaining, actual) = parse_endianness(input).finish().ok().unwrap();

        assert_that!(actual, is(equal_to(Endianness::Big)));
        assert_that!(remaining.data, is(equal_to(&[0xaa, 0xbb])));
        assert_that!(remaining.position, is(equal_to(1)))
    }

    #[test]
    fn test_parse_version() {
        let data = [0x01, 0x02, 0x03, 0xaa, 0xbb];
        let input = Input::new(&data, None, None);
        let (remaining, actual) = parse_version(input).finish().ok().unwrap();

        assert_that!(actual, is(equal_to(Version::new('\u{1}', '\u{2}', '\u{3}'))));
        assert_that!(remaining.data, is(equal_to(&[0xaa, 0xbb])));
        assert_that!(remaining.position, is(equal_to(3)))
    }

    #[test]
    fn test_parse_u16_le() {
        let data = [0x54, 0x45, 0xaa, 0xbb];
        let input = Input::new(&data, None, Some(Endianness::Little));
        let (remaining, actual) = parse_u16(input).finish().ok().unwrap();

        assert_that!(actual, is(equal_to(17748u16)));
        assert_that!(remaining.data, is(equal_to(&[0xaa, 0xbb])));
        assert_that!(remaining.position, is(equal_to(2)))
    }

    #[test]
    fn test_parse_u16_be() {
        let data = [0x54, 0x45, 0xaa, 0xbb];
        let input = Input::new(&data, None, Some(Endianness::Big));
        let (remaining, actual) = parse_u16(input).finish().ok().unwrap();

        assert_that!(actual, is(equal_to(21573u16)));
        assert_that!(remaining.data, is(equal_to(&[0xaa, 0xbb])));
        assert_that!(remaining.position, is(equal_to(2)))
    }

    #[test]
    fn test_parse_u32_le() {
        let data = [0x54, 0x45, 0x53, 0x54, 0xaa, 0xbb];
        let input = Input::new(&data, None, Some(Endianness::Little));
        let (remaining, actual) = parse_u32(input).finish().ok().unwrap();

        assert_that!(actual, is(equal_to(1414743380u32)));
        assert_that!(remaining.data, is(equal_to(&[0xaa, 0xbb])));
        assert_that!(remaining.position, is(equal_to(4)))
    }

    #[test]
    fn test_parse_u32_be() {
        let data = [0x54, 0x45, 0x53, 0x54, 0xaa, 0xbb];
        let input = Input::new(&data, None, Some(Endianness::Big));
        let (remaining, actual) = parse_u32(input).finish().ok().unwrap();

        assert_that!(actual, is(equal_to(1413829460u32)));
        assert_that!(remaining.data, is(equal_to(&[0xaa, 0xbb])));
        assert_that!(remaining.position, is(equal_to(4)))
    }

    #[test]
    fn test_parse_u64_le() {
        let data = [0x42, 0x4c, 0x45, 0x4e, 0x44, 0x45, 0x52, 0x2d, 0xaa, 0xbb];
        let input = Input::new(&data, None, Some(Endianness::Little));
        let (remaining, actual) = parse_u64(input).finish().ok().unwrap();

        assert_that!(actual, is(equal_to(3265748839470287938u64)));
        assert_that!(remaining.data, is(equal_to(&[0xaa, 0xbb])));
        assert_that!(remaining.position, is(equal_to(8)))
    }

    #[test]
    fn test_parse_u64_be() {
        let data = [0x42, 0x4c, 0x45, 0x4e, 0x44, 0x45, 0x52, 0x2d, 0xaa, 0xbb];
        let input = Input::new(&data, None, Some(Endianness::Big));
        let (remaining, actual) = parse_u64(input).finish().ok().unwrap();

        assert_that!(actual, is(equal_to(4777269507188412973u64)));
        assert_that!(remaining.data, is(equal_to(&[0xaa, 0xbb])));
        assert_that!(remaining.position, is(equal_to(8)))
    }

    #[test]
    fn test_parse_pointer_32bit_le() {
        let data = [0x42, 0x4c, 0x45, 0x4e, 0x44, 0x45, 0x52, 0x2d, 0xaa, 0xbb];
        let input = Input::new(&data, Some(PointerSize::Pointer4Bytes), Some(Endianness::Little));
        let (remaining, actual) = parse_pointer(input).finish().ok().unwrap();

        assert_that!(actual, is(equal_to(1313164354usize)));
        assert_that!(remaining.data, is(equal_to(&[0x44, 0x45, 0x52, 0x2d, 0xaa, 0xbb])));
        assert_that!(remaining.position, is(equal_to(4)))
    }

    #[test]
    fn test_parse_pointer_32bit_be() {
        let data = [0x42, 0x4c, 0x45, 0x4e, 0x44, 0x45, 0x52, 0x2d, 0xaa, 0xbb];
        let input = Input::new(&data, Some(PointerSize::Pointer4Bytes), Some(Endianness::Big));
        let (remaining, actual) = parse_pointer(input).finish().ok().unwrap();

        assert_that!(actual, is(equal_to(1112294734usize)));
        assert_that!(remaining.data, is(equal_to(&[0x44, 0x45, 0x52, 0x2d, 0xaa, 0xbb])));
        assert_that!(remaining.position, is(equal_to(4)))
    }


    #[test]
    fn test_parse_pointer_64bit_le() {
        let data = [0x42, 0x4c, 0x45, 0x4e, 0x44, 0x45, 0x52, 0x2d, 0xaa, 0xbb];
        let input = Input::new(&data, Some(PointerSize::Pointer8Bytes), Some(Endianness::Little));
        let (remaining, actual) = parse_pointer(input).finish().ok().unwrap();

        assert_that!(actual, is(equal_to(3265748839470287938usize)));
        assert_that!(remaining.data, is(equal_to(&[0xaa, 0xbb])));
        assert_that!(remaining.position, is(equal_to(8)))
    }

    #[test]
    fn test_parse_pointer_64bit_be() {
        let data = [0x42, 0x4c, 0x45, 0x4e, 0x44, 0x45, 0x52, 0x2d, 0xaa, 0xbb];
        let input = Input::new(&data, Some(PointerSize::Pointer8Bytes), Some(Endianness::Big));
        let (remaining, actual) = parse_pointer(input).finish().ok().unwrap();

        assert_that!(actual, is(equal_to(4777269507188412973usize)));
        assert_that!(remaining.data, is(equal_to(&[0xaa, 0xbb])));
        assert_that!(remaining.position, is(equal_to(8)))
    }
}
