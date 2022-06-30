use std::num::NonZeroUsize;
use std::convert::TryInto;
use crate::blend::parse::{Blend, Identifier};

type Input<'a> = &'a[u8];

struct Structure {}

fn analyse(blend: Blend, input: Input) {

    let dna_block = blend.blocks_by_identifier(Identifier::DNA).unwrap()[0];
    let address = dna_block.address.unwrap();
    let start: usize = address.into();
    let end: usize = start + dna_block.length;

    let data = &input[start..end];
    let x: [u8; 4] =  data[..3].try_into().unwrap();
    println!("start: {:?}", start);
    println!("end: {:?}", end);
    println!("sdna: {:?}", x);
}

#[cfg(test)]
mod test {
    use crate::blend::analyse::{analyse, Input};
    // use crate::blend::parse::parse_blend;

    #[test]
    fn test_analyse() {
        let blend_file = std::fs::read("assets/cube.blend").unwrap();
        let input = crate::blend::parse::Input::new(blend_file.as_slice());
        // let blend = parse_blend(input).unwrap();
        //
        // analyse(blend, input.0)
    }
}
