use std::convert::TryInto;
use std::num::NonZeroUsize;

use crate::blend::parse::{Blend, FileBlock, Identifier};

type Data<'a> = &'a[u8];
type Input<'a> = &'a[u8];

struct Structure {}

fn analyse(blend: Blend, input: Input) {

    let dna_block = blend.blocks_by_identifier(Identifier::DNA).unwrap()[0];
    let location = dna_block.location;
    let start: usize = location.into();
    let end: usize = start + dna_block.length;

    let data = read(dna_block, input);

    println!("x: {:?}", data);
}

fn read(file_block: FileBlock, input: Data) -> Data {
    let start = file_block.location + 24;
    let end = start + file_block.length;
    &input[start..end]
}



#[cfg(test)]
mod test {
    use crate::blend::analyse::{analyse, Input};
    use crate::blend::parse::parse;

    #[test]
    fn test_analyse() {
        let blend_file = std::fs::read("assets/cube.blend").unwrap();
        let blend = parse(blend_file.as_slice()).unwrap();

        analyse(blend, blend_file.as_slice());
    }
}
