use std::collections::HashMap;
use std::num::NonZeroU64;

mod parse {

    use std::collections::HashMap;
    use std::num::NonZeroU64;
    use std::default::Default;
    use nom::branch::alt;
    use nom::bytes::complete::{tag, take, take_until, take_until1};
    use nom::combinator::map;
    use nom::{Err, Finish, IResult};
    use nom::error::{Error, ParseError};
    use nom::number::complete::{be_u32, be_u64, le_u32, le_u64};
    use nom::sequence::{pair, preceded, terminated, tuple};
    use thiserror::Error;

    type Input<'a> = &'a[u8];
    type Result<'a, A> = IResult<&'a[u8], A>;

    const BLENDER: [u8; 7] = [0x42, 0x4c, 0x45, 0x4e, 0x44, 0x45, 0x52];
    const REND: [u8; 4] = [0x52, 0x45, 0x4e, 0x44];

    #[derive(Debug)]
    struct Blend {
        header: FileHeader,
        blocks: HashMap<NonZeroU64, FileBlock>
    }

    #[derive(Debug, Copy, Clone, PartialEq)]
    struct FileHeader {
        pointer_size: PointerSize,
        endianness: Endianness,
        version: Version,
    }

    #[derive(Debug, Copy, Clone, PartialEq)]
    enum PointerSize {
        Pointer4Bytes,
        Pointer8Bytes
    }

    impl PointerSize {
        fn size(&self) -> usize {
            match self {
                PointerSize::Pointer4Bytes => 4,
                PointerSize::Pointer8Bytes => 8
            }
        }
    }

    #[derive(Debug, Copy, Clone, PartialEq)]
    enum Endianness {
        Little,
        Big
    }

    #[derive(Debug, Copy, Clone, PartialEq)]
    struct Version {
        major: char,
        minor: char,
        patch: char,
    }

    #[derive(Debug, Copy, Clone)]
    struct FileBlock {
        identifier: Identifier,
        length: usize,
        address: Option<NonZeroU64>,
        index: usize,
        count: usize,
    }

    #[derive(Debug, Copy, Clone, PartialEq)]
    enum Identifier {
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
    enum BlendParseError {

        #[error("Failed to parse header!")]
        ParseHeaderError,

        #[error("An error occurred parsing blend file!")]
        ParseError,
    }

    fn parse_blend(input: Input) -> ::core::result::Result<Blend, BlendParseError> {
        match parse_file_header(input) {
            Ok((input, file_header)) => {
                match parse_file_blocks(file_header.pointer_size, file_header.endianness, input) {
                    Ok((_, file_blocks)) => {
                        ::std::result::Result::Ok(
                            Blend {
                                header: file_header,
                                blocks: file_blocks,
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
            tag(&BLENDER[..]),
            tuple((
                alt((
                    parse_pointer_4_bytes,
                    parse_pointer_8_bytes
                )),
                alt((
                    parse_endianness_little,
                    parse_endianness_big
                )),
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

    fn parse_file_blocks(pointer_size: PointerSize, endianness: Endianness, input: Input) -> Result<HashMap<NonZeroU64, FileBlock>> {
        let mut input = input;
        let mut file_blocks: HashMap<NonZeroU64, FileBlock> = HashMap::new();
        loop {
            match parse_file_block(pointer_size, endianness, input) {
                Ok((remaining_input, file_block)) => {
                    if let Some(address) = file_block.address {
                        file_blocks.insert(address, file_block);
                    }
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

        let (input, code) = parse_file_block_identifier(input)?;

        let (input, length) = match endianness {
            Endianness::Little => le_u32,
            Endianness::Big => be_u32
        }(input)?;

        let (input, address) = match (pointer_size, endianness) {
            (PointerSize::Pointer4Bytes, Endianness::Little) => {
                map(le_u32, |parsed| u64::from(parsed))(input)?
            },
            (PointerSize::Pointer4Bytes, Endianness::Big) => {
                map(be_u32, |parsed| u64::from(parsed))(input)?
            },
            (PointerSize::Pointer8Bytes, Endianness::Little) => {
                le_u64(input)?
            },
            (PointerSize::Pointer8Bytes, Endianness::Big) => {
                be_u64(input)?
            }
        };

        let (input, index) = match endianness {
            Endianness::Little => le_u32,
            Endianness::Big => be_u32
        }(input)?;

        let (input, count) = match endianness {
            Endianness::Little => le_u32,
            Endianness::Big => be_u32
        }(input)?;

        let (input, _) = take(length + 4)(input)?; //TODO: WTF? +4?

        Ok((
            input,
            FileBlock {
                identifier: code,
                length: length as usize,
                address: NonZeroU64::new(address),
                index: index as usize,
                count: count as usize,
            }
        ))
    }

    fn parse_pointer_4_bytes(input: Input) -> Result<PointerSize> {
        map(tag(&[0x2d][..]), |_| PointerSize::Pointer4Bytes)(input)
    }

    fn parse_pointer_8_bytes(input: Input) -> Result<PointerSize> {
        map(tag(&[0x5f][..]), |_| PointerSize::Pointer8Bytes)(input)
    }

    fn parse_endianness_little(input: Input) -> Result<Endianness> {
        map(tag(&[0x76][..]), |_| Endianness::Little)(input)
    }

    fn parse_endianness_big(input: Input) -> Result<Endianness> {
        map(tag(&[0x56][..]), |_| Endianness::Big)(input)
    }

    fn parse_version(input: Input) -> Result<Version> {
        map(take(3usize), |parsed: &[u8]| {
            Version {
                major: parsed[0] as char,
                minor: parsed[1] as char,
                patch: parsed[2] as char
            }
        })(input)
    }

    fn parse_file_block_identifier(input: Input) -> Result<Identifier> {
        map(take(4usize), |parsed: &[u8]| {
            match parsed {
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
                _ => Identifier::Unknown { code: [parsed[0], parsed[1], parsed[2], parsed[3]]},
            }
        })(input)
    }

    mod test {
        use nom::{Err, Finish};
        use hamcrest2::{assert_that, HamcrestMatcher, equal_to, is};
        use nom::bytes::complete::take;
        use nom::error::Error;
        use crate::blend::parse::{Identifier, Endianness, PointerSize, Version, parse_blend};
        use crate::blend::parse::{parse_file_block, parse_file_header};

        #[test]
        fn test_parse_blend() {
            let blend_file = std::fs::read("assets/cube.blend").unwrap();
            let blend = parse_blend(blend_file.as_slice()).unwrap();

            assert_that!(blend.blocks.len(), is(equal_to(1936)));
        }

        #[test]
        fn test_parse_file_header() {

            let blend_file = std::fs::read("assets/cube.blend").unwrap();
            let (_, file_header) = parse_file_header(blend_file.as_slice())
                .finish()
                .ok()
                .unwrap();

            assert_that!(file_header.pointer_size, is(equal_to(PointerSize::Pointer4Bytes)));
            assert_that!(file_header.endianness, is(equal_to(Endianness::Little)));
            assert_that!(file_header.version, is(equal_to(Version { major: '3', minor: '0', patch: '2' })));
        }

        #[test]
        fn test_parse_file_blocks() {

            let blend_file = std::fs::read("assets/cube.blend").unwrap();
            let (input, file_header) = parse_file_header(blend_file.as_slice()).finish().ok().unwrap();

            // REND block
            let (input, file_block) = parse_file_block(file_header.pointer_size, file_header.endianness, input).finish().ok().unwrap();

            assert_that!(file_block.identifier, is(equal_to(Identifier::REND)));
            assert_that!(file_block.length, is(equal_to(72)));
            assert_that!(file_block.address.unwrap().get(), is(equal_to(4005448480)));
            assert_that!(file_block.index, is(equal_to(32766)));
            assert_that!(file_block.count, is(equal_to(0)));

            // TEST block
            let (input, file_block) = parse_file_block(file_header.pointer_size, file_header.endianness, input).finish().ok().unwrap();

            assert_that!(file_block.identifier, is(equal_to(Identifier::TEST)));
            assert_that!(file_block.length, is(equal_to(65544)));
            assert_that!(file_block.address.unwrap().get(), is(equal_to(1949085320)));
            assert_that!(file_block.index, is(equal_to(32737)));
            assert_that!(file_block.count, is(equal_to(0)));

            // GLOB block
            let (input, file_block) = parse_file_block(file_header.pointer_size, file_header.endianness, input).finish().ok().unwrap();

            assert_that!(file_block.identifier, is(equal_to(Identifier::GLOB)));
            assert_that!(file_block.length, is(equal_to(1104)));
            assert_that!(file_block.address.unwrap().get(), is(equal_to(4005448480)));
            assert_that!(file_block.index, is(equal_to(32766)));
            assert_that!(file_block.count, is(equal_to(314)));

            // WM block
            let (input, file_block) = parse_file_block(file_header.pointer_size, file_header.endianness, input).finish().ok().unwrap();

            assert_that!(file_block.identifier, is(equal_to(Identifier::WM)));
            assert_that!(file_block.length, is(equal_to(1448)));
            assert_that!(file_block.address.unwrap().get(), is(equal_to(3024905224)));
            assert_that!(file_block.index, is(equal_to(32737)));
            assert_that!(file_block.count, is(equal_to(631)));

            // First DATA block
            let (input, file_block) = parse_file_block(file_header.pointer_size, file_header.endianness, input).finish().ok().unwrap();

            assert_that!(file_block.identifier, is(equal_to(Identifier::DATA)));
            assert_that!(file_block.length, is(equal_to(336)));
            assert_that!(file_block.address.unwrap().get(), is(equal_to(2429373576)));
            assert_that!(file_block.index, is(equal_to(32737)));
            assert_that!(file_block.count, is(equal_to(632)));

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
            assert_that!(file_block.index, is(equal_to(0)));
            assert_that!(file_block.count, is(equal_to(0)));

            // ENDB block
            let (input, file_block) = parse_file_block(file_header.pointer_size, file_header.endianness, input).finish().ok().unwrap();

            assert_that!(file_block.identifier, is(equal_to(Identifier::ENDB)));
            assert_that!(file_block.length, is(equal_to(0)));
            assert_that!(file_block.address, is(equal_to(None)));
            assert_that!(file_block.index, is(equal_to(0)));
            assert_that!(file_block.count, is(equal_to(0)));
        }
    }
}
