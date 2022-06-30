use std::collections::HashMap;
use std::num::NonZeroUsize;
use std::default::Default;
use std::iter::{Copied, Enumerate};
use std::ops::RangeFrom;
use std::slice::Iter;
use itertools::Itertools;
use nom::branch::alt;
use nom::bytes::complete::{tag, take, take_until, take_until1};
use nom::combinator::{map, map_res};
use nom::{AsBytes, Compare, CompareResult, Err, Finish, InputIter, InputLength, InputTake, IResult, Needed, Slice};
use nom::error::{context, Error, ErrorKind, make_error, ParseError};
use nom::number::complete::{be_u32, be_u64, le_u32, le_u64};
use nom::sequence::{pair, preceded, terminated, tuple};
use thiserror::Error;

type Result<'a, A> = IResult<Input<'a>, A>;
type Data<'a> = &'a [u8];

const BLENDER_TAG: [u8; 7] = [0x42, 0x4c, 0x45, 0x4e, 0x44, 0x45, 0x52];
const POINTER_SIZE_32_BIT_TAG: [u8; 1] = [0x2d];
const POINTER_SIZE_64_BIT_TAG: [u8; 1] = [0x5f];
const ENDIANNESS_LITTLE_TAG: [u8; 1] = [0x76];
const ENDIANNESS_BIG_TAG: [u8; 1] = [0x56];

#[derive(Debug, Copy, Clone)]
pub struct Input<'a> {
    data: &'a [u8],
    position: usize,
}

impl<'a> Input<'a> {

    pub fn new(data: &[u8]) -> Input {
        Input {
            data,
            position: 0
        }
    }

    pub fn new_with_position(data: &[u8], position: usize) -> Input {
        Input {
            data,
            position
        }
    }
}

impl <'a> AsBytes for Input<'a> {
    fn as_bytes(&self) -> &[u8] {
        self.data
    }
}

impl <'a> Compare<Data<'a>> for Input<'a> {

    fn compare(&self, tag: Data<'a>) -> CompareResult {

        if tag.len() > self.data.len() {
            return CompareResult::Incomplete
        }

        for (a, b) in tag.iter().zip(self.data) {
            if a != b {
                return CompareResult::Error
            }
        }

        CompareResult::Ok
    }

    fn compare_no_case(&self, _tag: Data<'a>) -> CompareResult {
        unimplemented!()
    }
}

impl <'a> InputLength for Input<'a> {
    fn input_len(&self) -> usize {
        self.data.len()
    }
}

impl <'a> InputTake for Input<'a> {

    fn take(&self, count: usize) -> Self {
        Input::new_with_position(&self.data[..count], self.position + count)
    }

    fn take_split(&self, count: usize) -> (Self, Self) {
        let head = &self.data[..count];
        let tail = &self.data[count..];
        (Input::new_with_position(tail, self.position + count), Input::new_with_position(head, self.position))
    }
}

impl<'a> InputIter for Input<'a> {
    type Item = Input<'a>;
    type Iter = Enumerate<Self::IterElem>;
    type IterElem = Copied<Iter<'a, Input<'a>>>;

    fn iter_indices(&self) -> Self::Iter {
        unimplemented!()
    }

    fn iter_elements(&self) -> Self::IterElem {
        unimplemented!()
    }

    fn position<P>(&self, _predicate: P) -> Option<usize> where P: Fn(Self::Item) -> bool {
        unimplemented!()
    }

    fn slice_index(&self, count: usize) -> ::std::result::Result<usize, Needed> {
        if self.data.len() >= count {
            Ok(count)
        } else {
            Err(Needed::new(count - self.data.len()))
        }
    }
}

impl<'a> Slice<RangeFrom<usize>> for Input<'a> {
    fn slice(&self, range: RangeFrom<usize>) -> Self {
        Input::new_with_position(&self.data[range.start..], self.position + range.start)
    }
}

#[derive(Debug)]
pub struct Blend {
    header: FileHeader,
    blocks: Vec<FileBlock>,
    blocks_by_identifier: HashMap<Identifier, Vec<FileBlock>>,
    blocks_by_address: HashMap<NonZeroUsize, Vec<FileBlock>>,
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
    pub location: usize,
    pub dna: usize,
    pub count: usize,
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

    #[error("An error occurred parsing blend file!")]
    ParseError,
}

pub fn parse(blend: Data) -> ::std::result::Result<Blend, BlendParseError> {
    let input = Input::new(blend);
    parse_blend(input)
}

fn parse_blend(input: Input) -> ::std::result::Result<Blend, BlendParseError> {
    match parse_file_header(input) {
        Ok((input, header)) => {
            match parse_file_blocks(header.pointer_size, header.endianness, input) {
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
}

fn parse_file_blocks(pointer_size: PointerSize, endianness: Endianness, input: Input) -> Result<Vec<FileBlock>> {
    let mut input = input;
    let mut file_blocks = Vec::new();
    loop {
        match parse_file_block(pointer_size, endianness, input) {
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

fn parse_file_block(pointer_size: PointerSize, endianness: Endianness, input: Input) -> Result<FileBlock> {

    let location = input.position;
    let (input, identifier) = parse_file_block_identifier(input)?;
    let (input, length) = parse_u32(endianness, input)?;
    let (input, address) = parse_pointer(pointer_size, endianness, input)?;
    let (input, dna) = parse_u32(endianness, input)?;
    let (input, count) = parse_u32(endianness, input)?;
    let (input, _) = take(length + 4)(input)?; //TODO: WTF? +4?

    Ok((
        input,
        FileBlock {
            identifier,
            length: length as usize,
            address: NonZeroUsize::new(address),
            location,
            dna: dna as usize,
            count: count as usize,
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

fn parse_pointer(pointer_size: PointerSize, endianness: Endianness, input: Input) -> Result<usize> {
    match pointer_size {
        PointerSize::Pointer4Bytes => {
            parse_u32(endianness, input)
                .map(|(input, address)| (input, address as usize))
        },
        PointerSize::Pointer8Bytes => {
            parse_u64(endianness, input)
                .map(|(input, address)| (input, address as usize))
        }
    }
}

fn parse_u32(endianness: Endianness, input: Input) -> Result<u32> {
    let bound: usize = 4;
    if input.data.input_len() < bound {
        Err(Err::Error(make_error(input, ErrorKind::Eof)))
    }
    else {
        let bytes = input.data.iter_indices().take(bound);
        let mut result = 0u32;
        match endianness {
            Endianness::Little => {
                for (index, byte) in  bytes {
                    result += (byte as u32) << (8 * index);
                }
            }
            Endianness::Big => {
                for (_, byte) in bytes {
                    result = (result << 8) + byte as u32;
                }
            }
        }
        Ok((input.slice(bound..), result))
    }
}

fn parse_u64(endianness: Endianness, input: Input) -> Result<u64> {
    let bound: usize = 8;
    if input.data.input_len() < bound {
        Err(Err::Error(make_error(input, ErrorKind::Eof)))
    }
    else {
        let bytes = input.data.iter_indices().take(bound);
        let mut result = 0u64;
        match endianness {
            Endianness::Little => {
                for (index, byte) in  bytes {
                    result += (byte as u64) << (8 * index);
                }
            }
            Endianness::Big => {
                for (_, byte) in bytes {
                    result = (result << 8) + byte as u64;
                }
            }
        }
        Ok((input.slice(bound..), result))
    }
}

#[cfg(test)]
mod test {

    use nom::{Err, Finish};
    use hamcrest2::{assert_that, HamcrestMatcher, equal_to, is};
    use nom::bytes::complete::take;
    use nom::error::Error;
    use crate::blend::parse::{Endianness, ENDIANNESS_BIG_TAG, ENDIANNESS_LITTLE_TAG, Identifier, Input, parse_blend, parse_endianness, parse_file_block, parse_file_header, parse_pointer_size, parse_u32, parse_version, POINTER_SIZE_32_BIT_TAG, POINTER_SIZE_64_BIT_TAG, PointerSize, Version};

    #[test]
    fn test_parse_blend() {
        let blend_file = std::fs::read("assets/cube.blend").unwrap();
        let input = Input::new(blend_file.as_slice());
        let blend = parse_blend(input).unwrap();

        assert_that!(blend.blocks.len(), is(equal_to(1938)));
    }

    #[test]
    fn test_parse_file_header() {

        let blend_file = std::fs::read("assets/cube.blend").unwrap();
        let input = Input::new(blend_file.as_slice());
        let (remaining, file_header) = parse_file_header(input)
            .finish()
            .ok()
            .unwrap();

        assert_that!(file_header.pointer_size, is(equal_to(PointerSize::Pointer4Bytes)));
        assert_that!(file_header.endianness, is(equal_to(Endianness::Little)));
        assert_that!(file_header.version, is(equal_to(Version { major: '3', minor: '0', patch: '2' })));
        assert_that!(remaining.position, is(equal_to(12)));
    }

    #[test]
    fn test_parse_file_blocks() {

        let blend_file = std::fs::read("assets/cube.blend").unwrap();
        let input = Input::new(blend_file.as_slice());
        let (input, file_header) = parse_file_header(input)
            .finish()
            .ok()
            .unwrap();

        // REND block
        let (input, file_block) = parse_file_block(file_header.pointer_size, file_header.endianness, input).finish().ok().unwrap();

        assert_that!(file_block.identifier, is(equal_to(Identifier::REND)));
        assert_that!(file_block.length, is(equal_to(72)));
        assert_that!(file_block.address.unwrap().get(), is(equal_to(4005448480)));
        assert_that!(file_block.location, is(equal_to(12)));
        assert_that!(file_block.dna, is(equal_to(32766)));
        assert_that!(file_block.count, is(equal_to(0)));
        assert_that!(input.position, is(equal_to(108)));

        // TEST block
        let (input, file_block) = parse_file_block(file_header.pointer_size, file_header.endianness, input).finish().ok().unwrap();

        assert_that!(file_block.identifier, is(equal_to(Identifier::TEST)));
        assert_that!(file_block.length, is(equal_to(65544)));
        assert_that!(file_block.address.unwrap().get(), is(equal_to(1949085320)));
        assert_that!(file_block.location, is(equal_to(108)));
        assert_that!(file_block.dna, is(equal_to(32737)));
        assert_that!(file_block.count, is(equal_to(0)));
        assert_that!(input.position, is(equal_to(65676)));

        // GLOB block
        let (input, file_block) = parse_file_block(file_header.pointer_size, file_header.endianness, input).finish().ok().unwrap();

        assert_that!(file_block.identifier, is(equal_to(Identifier::GLOB)));
        assert_that!(file_block.length, is(equal_to(1104)));
        assert_that!(file_block.address.unwrap().get(), is(equal_to(4005448480)));
        assert_that!(file_block.location, is(equal_to(65676)));
        assert_that!(file_block.dna, is(equal_to(32766)));
        assert_that!(file_block.count, is(equal_to(314)));
        assert_that!(input.position, is(equal_to(66804)));

        // WM block
        let (input, file_block) = parse_file_block(file_header.pointer_size, file_header.endianness, input).finish().ok().unwrap();

        assert_that!(file_block.identifier, is(equal_to(Identifier::WM)));
        assert_that!(file_block.length, is(equal_to(1448)));
        assert_that!(file_block.address.unwrap().get(), is(equal_to(3024905224)));
        assert_that!(file_block.location, is(equal_to(66804)));
        assert_that!(file_block.dna, is(equal_to(32737)));
        assert_that!(file_block.count, is(equal_to(631)));
        assert_that!(input.position, is(equal_to(68276)));

        // First DATA block
        let (input, file_block) = parse_file_block(file_header.pointer_size, file_header.endianness, input).finish().ok().unwrap();

        assert_that!(file_block.identifier, is(equal_to(Identifier::DATA)));
        assert_that!(file_block.length, is(equal_to(336)));
        assert_that!(file_block.address.unwrap().get(), is(equal_to(2429373576)));
        assert_that!(file_block.location, is(equal_to(68276)));
        assert_that!(file_block.dna, is(equal_to(32737)));
        assert_that!(file_block.count, is(equal_to(632)));
        assert_that!(input.position, is(equal_to(68636)));

        // Skip to DNA block
        let (input, file_block) = {
            let (mut input, mut file_block) = parse_file_block(file_header.pointer_size, file_header.endianness, input).finish().ok().unwrap();
            while file_block.identifier != Identifier::DNA {
                let (next, block) = parse_file_block(file_header.pointer_size, file_header.endianness, input).finish().ok().unwrap();
                input = next;
                file_block = block;
            }

            (input, file_block)
        };

        assert_that!(file_block.identifier, is(equal_to(Identifier::DNA)));
        assert_that!(file_block.length, is(equal_to(116240)));
        assert_that!(file_block.address.unwrap().get(), is(equal_to(216360692)));
        assert_that!(file_block.location, is(equal_to(713032)));
        assert_that!(file_block.dna, is(equal_to(0)));
        assert_that!(file_block.count, is(equal_to(0)));
        assert_that!(input.position, is(equal_to(829296)));

        // ENDB block
        let (input, file_block) = parse_file_block(file_header.pointer_size, file_header.endianness, input).finish().ok().unwrap();

        assert_that!(file_block.identifier, is(equal_to(Identifier::ENDB)));
        assert_that!(file_block.length, is(equal_to(0)));
        assert_that!(file_block.address, is(equal_to(None)));
        assert_that!(file_block.location, is(equal_to(829296)));
        assert_that!(file_block.dna, is(equal_to(0)));
        assert_that!(file_block.count, is(equal_to(0)));
        assert_that!(input.position, is(equal_to(829320)));
    }

    #[test]
    fn test_parse_pointer_size_32bit() {
        let data = [POINTER_SIZE_32_BIT_TAG[0], 0xaa, 0xbb];
        let input = Input::new(&data);
        let (remaining, actual) = parse_pointer_size(input).finish().ok().unwrap();

        assert_that!(actual, is(equal_to(PointerSize::Pointer4Bytes)));
        assert_that!(remaining.data, is(equal_to(&[0xaa, 0xbb])));
    }

    #[test]
    fn test_parse_pointer_size_64bit() {
        let data = [POINTER_SIZE_64_BIT_TAG[0], 0xaa, 0xbb];
        let input = Input::new(&data);
        let (remaining, actual) = parse_pointer_size(input).finish().ok().unwrap();

        assert_that!(actual, is(equal_to(PointerSize::Pointer8Bytes)));
        assert_that!(remaining.data, is(equal_to(&[0xaa, 0xbb])));
        assert_that!(remaining.position, is(equal_to(1)))
    }

    #[test]
    fn test_parse_endianness_little() {
        let data = [ENDIANNESS_LITTLE_TAG[0], 0xaa, 0xbb];
        let input = Input::new(&data);
        let (remaining, actual) = parse_endianness(input).finish().ok().unwrap();

        assert_that!(actual, is(equal_to(Endianness::Little)));
        assert_that!(remaining.data, is(equal_to(&[0xaa, 0xbb])));
        assert_that!(remaining.position, is(equal_to(1)))
    }

    #[test]
    fn test_parse_endianness_big() {
        let data = [ENDIANNESS_BIG_TAG[0], 0xaa, 0xbb];
        let input = Input::new(&data);
        let (remaining, actual) = parse_endianness(input).finish().ok().unwrap();

        assert_that!(actual, is(equal_to(Endianness::Big)));
        assert_that!(remaining.data, is(equal_to(&[0xaa, 0xbb])));
        assert_that!(remaining.position, is(equal_to(1)))
    }

    #[test]
    fn test_parse_version() {
        let data = [0x01, 0x02, 0x03, 0xaa, 0xbb];
        let input = Input::new(&data);
        let (remaining, actual) = parse_version(input).finish().ok().unwrap();

        assert_that!(actual, is(equal_to(Version::new('\u{1}', '\u{2}', '\u{3}'))));
        assert_that!(remaining.data, is(equal_to(&[0xaa, 0xbb])));
        assert_that!(remaining.position, is(equal_to(3)))
    }


    #[test]
    fn test_parse_u32_le() {
        let data = [0x54, 0x45, 0x53, 0x54, 0xaa, 0xbb];
        let input = Input::new(&data);
        let (remaining, actual) = parse_u32(Endianness::Little, input).finish().ok().unwrap();

        assert_that!(actual, is(equal_to(1414743380u32)));
        assert_that!(remaining.data, is(equal_to(&[0xaa, 0xbb])));
        assert_that!(remaining.position, is(equal_to(4)))
    }

    #[test]
    fn test_parse_u32_be() {
        let data = [0x54, 0x45, 0x53, 0x54, 0xaa, 0xbb];
        let input = Input::new(&data);
        let (remaining, actual) = parse_u32(Endianness::Big, input).finish().ok().unwrap();

        assert_that!(actual, is(equal_to(1413829460u32)));
        assert_that!(remaining.data, is(equal_to(&[0xaa, 0xbb])));
        assert_that!(remaining.position, is(equal_to(4)))
    }
}
