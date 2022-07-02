use std::convert::TryInto;
use std::num::NonZeroUsize;
use nom::bytes::complete::take;
use nom::{Finish, IResult};
use nom::combinator::map;
use nom::sequence::terminated;
use thiserror::Error;
use crate::blend::analyse::input::Input;

use crate::blend::parse::{Blend, Data, FileBlock, Identifier};

type Result<'a, A> = ::std::result::Result<A, AnalyseError>;

#[derive(Error, Debug)]
enum AnalyseError {

    #[error("Dna not found!")]
    DnaNotFound
}

mod input {
    use crate::blend::parse::{Blend, Data};

    pub struct Input<'a> {
        pub blend: Blend,
        pub data: Data<'a>,
    }

    impl <'a> Input<'a> {
        pub fn new(blend: Blend, data: Data<'a>) -> Self {
            Self {
                blend,
                data,
            }
        }
    }
}

struct Structure {}

fn analyse(blend: Blend, input: Input) {

    let dna_block = blend.blocks_by_identifier(Identifier::DNA).unwrap()[0];
    let location = dna_block.data_location();
    let start: usize = location.into();
    let end: usize = start + dna_block.length;

    // let data = read(dna_block, input);
    //
    // println!("x: {:?}", data);
}

struct Dna {}

fn analyse_dna(input: Input) -> Result<Dna> {
    let dna_block = input.blend.blocks_by_identifier(Identifier::DNA).unwrap()[0];
    let dna_data = read(input, dna_block)?;
    let (dna_data, id) = parse_dna_id(dna_data).ok().unwrap(); //TODO
    // let (dna_data, names_count) =
    println!("id: {:?}", id);
    Ok(Dna {})
}

fn parse_dna_id(input: Data) -> IResult<Data, [u8; 4]> {
    map(terminated(take(4usize),take(4usize)), |parsed: &[u8]| {
        [parsed[0], parsed[1], parsed[2], parsed[3]]
    })(input)
}

// fn parse_dna_names_count(input: Data) -> IResult<Data, usize> {
//     map(, |parsed: &[u8]| {
//         [parsed[0], parsed[1], parsed[2], parsed[3]]
//     })(input)
// }

fn read(input: Input, file_block: FileBlock) -> Result<Data> {
    let start = file_block.data_location();
    let end = start + file_block.length;
    Ok(&input.data[start..end])
}

#[cfg(test)]
mod test {
    use crate::blend::analyse::{analyse, analyse_dna, Input};
    use crate::blend::parse::parse;

    #[test]
    fn test_analyse_dna() {
        let blend_file = std::fs::read("assets/cube.blend").unwrap();
        let blend = parse(blend_file.as_slice()).unwrap();

        let input = Input::new(blend, blend_file.as_slice());
        let dna = analyse_dna(input).ok().unwrap();
        // analyse(blend, blend_file.as_slice());
    }
}
