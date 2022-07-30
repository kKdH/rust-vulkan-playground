mod builder;

use crate::{Blend, BlendSource};
use crate::reader::builder::{NoIdentifier, ReaderBuilder};


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
