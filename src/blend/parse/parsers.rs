use std::collections::HashMap;
use std::num::NonZeroUsize;
use std::os::unix::raw::ino_t;
use itertools::Itertools;
use nom::bytes::complete::{tag, take, take_until, take_until1};
use nom::combinator::{map, rest};
use nom::{Err, InputIter, InputLength, IResult, Slice};
use nom::branch::alt;
use nom::error::{context, ErrorKind, make_error};
use nom::multi::length_count;
use nom::sequence::{preceded, terminated, tuple};
use winit::event::VirtualKeyCode::P;
use crate::blend::parse::{Blend, BlendParseError, Dna, Endianness, FileBlock, FileHeader, Identifier, Location, PointerSize, Version};
use crate::blend::parse::input::Input;

const BLENDER_TAG: [u8; 7] = [0x42, 0x4c, 0x45, 0x4e, 0x44, 0x45, 0x52];
const POINTER_SIZE_32_BIT_TAG: [u8; 1] = [0x2d];
const POINTER_SIZE_64_BIT_TAG: [u8; 1] = [0x5f];
const ENDIANNESS_LITTLE_TAG: [u8; 1] = [0x76];
const ENDIANNESS_BIG_TAG: [u8; 1] = [0x56];

const FILE_BLOCK_IDENTIFIER_SIZE: usize = 4;
const FILE_BLOCK_LENGTH_SIZE: usize = 4;
const FILE_BLOCK_DNA_SIZE: usize = 4;
const FILE_BLOCK_COUNT_SIZE: usize = 4;

type Result<'a, A> = IResult<Input<'a>, A>;

pub fn parse_blend(input: Input) -> ::std::result::Result<Blend, BlendParseError> {
    match parse_file_header(input) {
        Ok((input, header)) => {
            match parse_file_blocks(input) {
                Ok((_, blocks)) => {
                    let blocks_by_address: HashMap<NonZeroUsize, Vec<FileBlock>> = blocks.iter()
                        .cloned()
                        .filter(|block| block.address.is_some())
                        .into_group_map_by(|block| block.address.unwrap());
                    let blocks_by_identifier: HashMap<Identifier, Vec<FileBlock>> = blocks.iter()
                        .cloned()
                        .into_group_map_by(|block| block.identifier);
                    ::std::result::Result::Ok(
                        Blend {
                            header,
                            blocks,
                            blocks_by_identifier,
                            blocks_by_address,
                        }
                    )
                }
                Err(_) => ::std::result::Result::Err(BlendParseError::ParseError)
            }
        }
        Err(_) => ::std::result::Result::Err(BlendParseError::ParseHeaderError)
    }
}

fn parse_dna(input: Input) -> Result<Dna> {
    let (input, _) = parse_dna_id(input)?;
    let (input, names) = parse_dna_names(input)?;
    let (input, types) = parse_dna_types(input)?;
    Ok((input, Dna {
        names,
        types,
    }))
}

fn parse_dna_id(input: Input) -> Result<[u8; 4]> {
    map(take(4usize), |input: Input| {
        [input.data[0], input.data[1], input.data[2], input.data[3]]
    })(input)
}

fn parse_dna_names(input: Input) -> Result<Vec<String>> {
    context(
        "names",
        preceded(
            tag(&[0x4e, 0x41, 0x4d, 0x45][..]),
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

fn parse_dna_types(input: Input) -> Result<Vec<String>> {
    context(
        "types",
        preceded(
            tag(&[0x54, 0x59, 0x50, 0x45][..]),
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
    let (input, dna) = parse_u32(input)?;
    let (input, count) = parse_u32(input)?;
    let (input, _) = take(length + 4)(input)?; //TODO: WTF? +4?

    Ok((
        input,
        FileBlock {
            identifier,
            length: length as usize,
            address: NonZeroUsize::new(address),
            dna: dna as usize,
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
    use std::ops::RangeFrom;
    use nom::{Err, Finish, Slice};
    use hamcrest2::{assert_that, HamcrestMatcher, equal_to, is};
    use nom::bytes::complete::take;
    use nom::error::Error;
    use crate::blend::parse::{Endianness, Identifier, PointerSize, Version};
    use crate::blend::parse::input::Input;
    use crate::blend::parse::parsers::{ENDIANNESS_BIG_TAG, ENDIANNESS_LITTLE_TAG, parse_dna, POINTER_SIZE_32_BIT_TAG, POINTER_SIZE_64_BIT_TAG};
    use crate::blend::parse::parsers::{parse_blend, parse_endianness, parse_file_block, parse_file_header, parse_pointer, parse_pointer_size, parse_u32, parse_u64, parse_version};

    #[test]
    fn test_parse_blend() {
        let blend_file = std::fs::read("assets/cube.blend").unwrap();
        let input = Input::new(blend_file.as_slice(), None, None);
        let blend = parse_blend(input).unwrap();

        assert_that!(blend.blocks.len(), is(equal_to(1938)));
    }

    #[test]
    fn test_parse_file_header() {

        let blend_file = std::fs::read("assets/cube.blend").unwrap();
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

        let blend_file = std::fs::read("assets/cube.blend").unwrap();
        let input = Input::new(blend_file.as_slice(), None, None);
        let (input, file_header) = parse_file_header(input)
            .finish()
            .ok()
            .unwrap();

        // REND block
        let (input, file_block) = parse_file_block(input).finish().ok().unwrap();

        assert_that!(file_block.identifier, is(equal_to(Identifier::REND)));
        assert_that!(file_block.length, is(equal_to(72)));
        assert_that!(file_block.address.unwrap().get(), is(equal_to(4005448480)));
        assert_that!(file_block.block_location(), is(equal_to(12)));
        assert_that!(file_block.dna, is(equal_to(32766)));
        assert_that!(file_block.count, is(equal_to(0)));
        assert_that!(input.position, is(equal_to(108)));

        // TEST block
        let (input, file_block) = parse_file_block(input).finish().ok().unwrap();

        assert_that!(file_block.identifier, is(equal_to(Identifier::TEST)));
        assert_that!(file_block.length, is(equal_to(65544)));
        assert_that!(file_block.address.unwrap().get(), is(equal_to(1949085320)));
        assert_that!(file_block.block_location(), is(equal_to(108)));
        assert_that!(file_block.dna, is(equal_to(32737)));
        assert_that!(file_block.count, is(equal_to(0)));
        assert_that!(input.position, is(equal_to(65676)));

        // GLOB block
        let (input, file_block) = parse_file_block(input).finish().ok().unwrap();

        assert_that!(file_block.identifier, is(equal_to(Identifier::GLOB)));
        assert_that!(file_block.length, is(equal_to(1104)));
        assert_that!(file_block.address.unwrap().get(), is(equal_to(4005448480)));
        assert_that!(file_block.block_location(), is(equal_to(65676)));
        assert_that!(file_block.dna, is(equal_to(32766)));
        assert_that!(file_block.count, is(equal_to(314)));
        assert_that!(input.position, is(equal_to(66804)));

        // WM block
        let (input, file_block) = parse_file_block(input).finish().ok().unwrap();

        assert_that!(file_block.identifier, is(equal_to(Identifier::WM)));
        assert_that!(file_block.length, is(equal_to(1448)));
        assert_that!(file_block.address.unwrap().get(), is(equal_to(3024905224)));
        assert_that!(file_block.block_location(), is(equal_to(66804)));
        assert_that!(file_block.dna, is(equal_to(32737)));
        assert_that!(file_block.count, is(equal_to(631)));
        assert_that!(input.position, is(equal_to(68276)));

        // First DATA block
        let (input, file_block) = parse_file_block(input).finish().ok().unwrap();

        assert_that!(file_block.identifier, is(equal_to(Identifier::DATA)));
        assert_that!(file_block.length, is(equal_to(336)));
        assert_that!(file_block.address.unwrap().get(), is(equal_to(2429373576)));
        assert_that!(file_block.block_location(), is(equal_to(68276)));
        assert_that!(file_block.dna, is(equal_to(32737)));
        assert_that!(file_block.count, is(equal_to(632)));
        assert_that!(input.position, is(equal_to(68636)));

        // Skip to DNA block
        let (input, file_block) = {
            let (mut input, mut file_block) = parse_file_block(input).finish().ok().unwrap();
            while file_block.identifier != Identifier::DNA {
                let (next, block) = parse_file_block(input).finish().ok().unwrap();
                input = next;
                file_block = block;
            }

            (input, file_block)
        };

        assert_that!(file_block.identifier, is(equal_to(Identifier::DNA)));
        assert_that!(file_block.length, is(equal_to(116240)));
        assert_that!(file_block.address.unwrap().get(), is(equal_to(216360692)));
        assert_that!(file_block.block_location(), is(equal_to(713032)));
        assert_that!(file_block.dna, is(equal_to(0)));
        assert_that!(file_block.count, is(equal_to(0)));
        assert_that!(input.position, is(equal_to(829296)));

        // ENDB block
        let (input, file_block) = parse_file_block(input).finish().ok().unwrap();

        assert_that!(file_block.identifier, is(equal_to(Identifier::ENDB)));
        assert_that!(file_block.length, is(equal_to(0)));
        assert_that!(file_block.address, is(equal_to(None)));
        assert_that!(file_block.block_location(), is(equal_to(829296)));
        assert_that!(file_block.dna, is(equal_to(0)));
        assert_that!(file_block.count, is(equal_to(0)));
        assert_that!(input.position, is(equal_to(829320)));
    }

    #[test]
    fn test_parse_dna() {

        let blend_file = std::fs::read("assets/cube.blend").unwrap();
        let input = Input::new(blend_file.as_slice(), Some(PointerSize::Pointer4Bytes), Some(Endianness::Little));
        let (dna_input, _) = input.split(713032usize + 24);
        let (_, dna) = parse_dna(dna_input).unwrap();

        assert_that!(dna.names.len(), is(equal_to(4969)));
        assert_that!(dna.types.len(), is(equal_to(927)));
        println!("types: {:?}", dna.types);
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
