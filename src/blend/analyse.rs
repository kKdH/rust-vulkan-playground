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

fn read(input: Input, file_block: FileBlock) -> Result<Data> {
    let start = file_block.data_location();
    let end = start + file_block.length;
    Ok(&input.data[start..end])
}

#[cfg(test)]
mod test {
    use itertools::put_back;
    use crate::blend::analyse::{analyse, Input};
    use crate::blend::parse::{Blend, Dna, DnaField, FileBlock, Identifier, parse};

    #[test]
    fn test_analyse_dna() {
        let blend_data = std::fs::read("assets/cube.blend").unwrap();
        let blend = parse(blend_data.as_slice()).unwrap();

        // let input = Input::new(blend, blend_file.as_slice());
        // let dna = analyse_dna(input).ok().unwrap();
        // analyse(blend, blend_file.as_slice());

        let idType = blend.dna().types.iter()
            .enumerate()
            .find(|(_, tpe)| String::from("ID") == tpe.name)
            .unwrap();

        let idStruct = blend.dna().structs.iter()
            .enumerate()
            .find(|(_, stc)| idType.0 == stc.type_index)
            .unwrap();

        let objects = blend.blocks_by_identifier(Identifier::OB).unwrap();
        objects.iter().for_each(|object| {
            print_info(blend.dna(), object)
        });

        let meshes = blend.blocks_by_identifier(Identifier::ME).unwrap();
        meshes.iter().for_each(|mesh| {
            print_info(blend.dna(), mesh)
        });

        let scenes = blend.blocks_by_identifier(Identifier::SC).unwrap();
        scenes.iter().for_each(|scene| {
            print_info(blend.dna(), scene)
        });
    }

    fn read(data: Vec<u8>, blend: &Blend, block: &FileBlock, ) {

    }

    fn print_info(dna: &Dna, file_block: &FileBlock) {
        let strct = dna.struct_of(file_block).unwrap();
        let typ = dna.type_of(strct);



        println!("{}: {:?}", file_block.identifier, file_block);
        println!("  Type: {:?}", typ.unwrap().name);
        strct.fields.iter()
            .map(|field| (dna.field_name_of(field), dna.type_of(field)))
            .for_each(|(name, tpe)| {
                println!("    {:?}: {:?}", name, tpe.unwrap().name)
            });
    }
}
