use std::collections::HashMap;
use std::num::NonZeroUsize;
use crate::blend::parse::{Dna, FileBlock, FileHeader, Identifier};

mod analyse;
pub(crate) mod parse;

#[derive(Debug)]
pub struct Blend {
    header: FileHeader,
    blocks: Vec<FileBlock>,
    blocks_by_identifier: HashMap<Identifier, Vec<FileBlock>>,
    blocks_by_address: HashMap<NonZeroUsize, Vec<FileBlock>>,
    dna: Dna,
}

impl Blend {

    pub(crate) fn header(&self) -> &FileHeader {
        &self.header
    }

    pub(crate) fn dna(&self) -> &Dna {
        &self.dna
    }

    pub(crate) fn blocks_by_address(&self, address: NonZeroUsize) -> Option<&Vec<FileBlock>> {
        self.blocks_by_address.get(&address)
    }

    pub(crate) fn blocks_by_identifier(&self, identifier: Identifier) -> Option<&Vec<FileBlock>> {
        self.blocks_by_identifier.get(&identifier)
    }
}
