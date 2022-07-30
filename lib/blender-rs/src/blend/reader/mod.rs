mod builder;

use crate::blend::{Blend, BlendSource};
use crate::blend::reader::builder::{NoIdentifier, ReaderBuilder};


pub struct Reader {
}

impl Reader {

    pub fn builder() -> ReaderBuilder<NoIdentifier> {
        ReaderBuilder::new()
    }

    pub fn read<A>(blend: &Blend, source: A)
    where A: BlendSource {

    }
}
