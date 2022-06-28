use nom::branch::alt;
use nom::bytes::complete::{tag, take, take_until, take_until1};
use nom::combinator::map;
use nom::{Err, Finish, IResult};
use nom::error::Error;
use nom::sequence::{pair, preceded, terminated, tuple};

type Input<'a> = &'a[u8];
type Result<'a, A> = IResult<&'a[u8], A>;

#[derive(Debug, PartialEq)]
struct FileHeader {
    pointer_size: PointerSize,
    endianness: Endianness,
    version: Version,
}

#[derive(Debug, PartialEq)]
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

#[derive(Debug, PartialEq)]
enum Endianness {
    Little,
    Big
}

#[derive(Debug, PartialEq)]
struct Version {
    major: char,
    minor: char,
    patch: char,
}

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

fn parse_file_header(input: Input) -> IResult<&[u8], FileHeader> {
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
            terminated(
                parse_version,
                tag(&REND[..])
            )
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

mod test {
    use nom::Finish;
    use hamcrest2::{assert_that, HamcrestMatcher, equal_to, is};
    use crate::blend::{Endianness, parse_file_header, PointerSize, Version};

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
}
