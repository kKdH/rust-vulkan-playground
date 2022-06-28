use std::num::NonZeroU64;

#[derive(Debug, Copy, Clone, PartialEq)]
struct FileHeader {
    pointer_size: PointerSize,
    endianness: Endianness,
    version: Version,
}

#[derive(Debug)]
struct FileBlock {
    code: [u8; 4],
    length: usize,
    address: NonZeroU64,
    index: usize,
    count: usize,
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

mod parse {
    use std::num::NonZeroU64;
    use std::default::Default;
    use nom::branch::alt;
    use nom::bytes::complete::{tag, take, take_until, take_until1};
    use nom::combinator::map;
    use nom::{Err, Finish, IResult};
    use nom::error::{Error, ParseError};
    use nom::number::complete::{be_u32, be_u64, le_u32, le_u64};
    use nom::sequence::{pair, preceded, terminated, tuple};
    use crate::blend::{Endianness, FileBlock, FileHeader, PointerSize, Version};

    type Input<'a> = &'a[u8];
    type Result<'a, A> = IResult<&'a[u8], A>;

    const BLENDER: [u8; 7] = [0x42, 0x4c, 0x45, 0x4e, 0x44, 0x45, 0x52];
    const REND: [u8; 4] = [0x52, 0x45, 0x4e, 0x44];

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

    fn parse_file_block(header: FileHeader, input: Input) -> Result<FileBlock> {

        let (input, code) = take(4usize)(input)?;

        let (input, length) = match header.endianness {
            Endianness::Little => le_u32,
            Endianness::Big => be_u32
        }(input)?;

        let (input, address) = match (header.pointer_size, header.endianness) {
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

        let (input, index) = match header.endianness {
            Endianness::Little => le_u32,
            Endianness::Big => be_u32
        }(input)?;

        let (input, count) = match header.endianness {
            Endianness::Little => le_u32,
            Endianness::Big => be_u32
        }(input)?;

        let (input, _) = take(length + 4)(input)?; //TODO: WTF? +4?

        Ok((
            input,
            FileBlock {
                code: [code[0], code[1], code[2], code[3]],
                length: length as usize,
                address: NonZeroU64::new(address).unwrap(),
                index: index as usize,
                count: count as usize,
            }
        ))
    }

    mod test {
        use nom::{Err, Finish};
        use hamcrest2::{assert_that, HamcrestMatcher, equal_to, is};
        use nom::bytes::complete::take;
        use nom::error::Error;
        use crate::blend::{Endianness, PointerSize, Version};
        use crate::blend::parse::{parse_file_block, parse_file_header};

        #[test]
        fn test_parse_file_header() {
            let blend = std::fs::read("assets/cube.blend").unwrap();

            let (_, file_header) = parse_file_header(blend.as_slice())
                .finish()
                .ok()
                .unwrap();

            assert_that!(file_header.pointer_size, is(equal_to(PointerSize::Pointer4Bytes)));
            assert_that!(file_header.endianness, is(equal_to(Endianness::Little)));
            assert_that!(file_header.version, is(equal_to(Version { major: '3', minor: '0', patch: '2' })));
        }

        #[test]
        fn test_parse_file_blocks() {

            let blend = std::fs::read("assets/cube.blend").unwrap();
            let (input, file_header) = parse_file_header(blend.as_slice()).finish().ok().unwrap();

            // REND block
            let (input, file_block) = parse_file_block(file_header, input).finish().ok().unwrap();

            println!("FileBlock: {:?}", file_block);

            assert_that!(file_block.code, is(equal_to([0x52, 0x45, 0x4e, 0x44])));
            assert_that!(file_block.length, is(equal_to(72)));
            assert_that!(file_block.address.get(), is(equal_to(4005448480)));
            assert_that!(file_block.index, is(equal_to(32766)));
            assert_that!(file_block.count, is(equal_to(0)));

            // TEST block
            let (input, file_block) = parse_file_block(file_header, input).finish().ok().unwrap();

            println!("FileBlock: {:?}", file_block);

            assert_that!(file_block.code, is(equal_to([0x54, 0x45, 0x53, 0x54])));
            assert_that!(file_block.length, is(equal_to(65544)));
            assert_that!(file_block.address.get(), is(equal_to(1949085320)));
            assert_that!(file_block.index, is(equal_to(32737)));
            assert_that!(file_block.count, is(equal_to(0)));

            // GLOB block
            let (input, file_block) = parse_file_block(file_header, input).finish().ok().unwrap();

            println!("FileBlock: {:?}", file_block);

            assert_that!(file_block.code, is(equal_to([0x47, 0x4c, 0x4f, 0x42])));
            assert_that!(file_block.length, is(equal_to(1104)));
            assert_that!(file_block.address.get(), is(equal_to(4005448480)));
            assert_that!(file_block.index, is(equal_to(32766)));
            assert_that!(file_block.count, is(equal_to(314)));

            // WM block
            let (input, file_block) = parse_file_block(file_header, input).finish().ok().unwrap();

            println!("FileBlock: {:?}", file_block);

            assert_that!(file_block.code, is(equal_to([0x57, 0x4d, 0x0, 0x0])));
            assert_that!(file_block.length, is(equal_to(1448)));
            assert_that!(file_block.address.get(), is(equal_to(3024905224)));
            assert_that!(file_block.index, is(equal_to(32737)));
            assert_that!(file_block.count, is(equal_to(631)));

            // First DATA block
            let (input, file_block) = parse_file_block(file_header, input).finish().ok().unwrap();

            println!("FileBlock: {:?}", file_block);

            assert_that!(file_block.code, is(equal_to([0x44, 0x41, 0x54, 0x41])));
            assert_that!(file_block.length, is(equal_to(336)));
            assert_that!(file_block.address.get(), is(equal_to(2429373576)));
            assert_that!(file_block.index, is(equal_to(32737)));
            assert_that!(file_block.count, is(equal_to(632)));
        }
    }
}
