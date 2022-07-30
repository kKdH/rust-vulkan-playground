use core::result::Result;

use thiserror::Error;

use analyse::analyse;
use parse::parse;

use crate::blend::analyse::Structure;
use crate::blend::parse::{Dna, FileBlock, FileHeader, Version};

mod analyse;
mod parse;

pub type Data<'a> = &'a[u8];

#[derive(Debug)]
pub struct Blend {
    header: FileHeader,
    blocks: Vec<FileBlock>,
    dna: Dna,
    structure: Structure,
}

impl Blend {
    pub fn version(&self) -> &Version {
        &self.header.version
    }
}

pub trait BlendSource {
    fn data(&self) -> Data;
}

impl BlendSource for &[u8] {
    fn data(&self) -> Data {
        self
    }
}

impl BlendSource for Vec<u8> {
    fn data(&self) -> Data {
        self.as_slice()
    }
}

#[derive(Error, Debug)]
#[error("Failed to read blender data! {message}")]
pub struct BlendError {

    message: String,

    #[source]
    cause: Box<dyn std::error::Error>,
}

pub fn read<A>(source: A) -> Result<Blend, BlendError>
where A: BlendSource {
    let (header, blocks, dna) = parse(source.data())
        .map_err(|cause| {
            BlendError {
                message: String::from("Could not parse header, blocks and dna!"),
                cause: Box::new(cause)
            }
        })?;
    let structure = analyse(&header, &blocks, &dna)
        .map_err(|cause| {
            BlendError {
                message: String::from("Could not analyse the structure of the blender data!"),
                cause: Box::new(cause)
            }
        })?;
    Ok(Blend { header, blocks, dna, structure })
}

#[cfg(test)]
mod test {
    use hamcrest2::{assert_that, equal_to, HamcrestMatcher, is};

    use crate::blend::parse::Version;
    use crate::blend::read;

    #[test]
    fn test_read() {

        let blend_data = std::fs::read("test/resources/cube.blend").unwrap();
        let blend = read(blend_data).ok().unwrap();

        assert_that!(blend.version(), is(equal_to(&Version { major: '3', minor: '0', patch: '2' })))
    }
}
