use std::collections::HashMap;

use crate::transform::{Offset, Size, SourceId, Stride, SourceViewId};
use crate::transform::readable::Readable;

pub struct Source<'a> {
    readable: &'a dyn Readable
}

impl<'a> Source<'a> {

    pub fn new(readable: &'a dyn Readable) -> Source {
        Source {
            readable
        }
    }

    pub fn read(&self, offset: Offset, amount: Size, buffer: &mut [u8]) {
        self.readable.read(offset, amount, buffer);
    }
}

pub struct SourceSet<'a> {
    sources: HashMap<SourceId, &'a Source<'a>>
}

impl<'a> SourceSet<'a> {

    pub fn new(sources: &[(SourceId, &'a Source)]) -> SourceSet<'a> {
        SourceSet {
            sources: sources.iter().cloned().collect()
        }
    }

    pub fn source(&self, id: SourceId) -> &'a Source {
        self.sources.get(&id)
            .expect(&format!("Invalid SourceId: {}", id))
    }

    pub fn sources(&self) -> &HashMap<SourceId, &'a Source> {
        &self.sources
    }
}

#[derive(Debug, Copy, Clone)]
pub struct SourceView {
    id: SourceViewId,
    source: SourceId,
    offset: Offset,
    size: Size,
    stride: Stride,
}

impl SourceView {

    pub fn id(&self) -> &SourceViewId {
        &self.id
    }

    pub fn source(&self) -> &SourceId {
        &self.source
    }

    pub fn offset(&self) -> &Offset {
        &self.offset
    }

    pub fn size(&self) -> &Size {
        &self.size
    }

    pub fn stride(&self) -> &Stride {
        &self.stride
    }

    pub fn builder() -> SourceViewBuilder {
        SourceViewBuilder {
            inner: SourceView::default()
        }
    }
}

impl Default for SourceView {
    fn default() -> Self {
        SourceView {
            id: 0,
            source: 0,
            offset: 0,
            size: 0,
            stride: 0
        }
    }
}

pub struct SourceViewBuilder {
    inner: SourceView
}

impl SourceViewBuilder {

    pub fn id(mut self, id: SourceViewId) -> SourceViewBuilder {
        self.inner.id = id;
        self
    }

    pub fn source(mut self, source: SourceId) -> SourceViewBuilder {
        self.inner.source = source;
        self
    }

    pub fn offset(mut self, offset: Offset) -> SourceViewBuilder {
        self.inner.offset = offset;
        self
    }

    pub fn size(mut self, size: Size) -> SourceViewBuilder {
        self.inner.size = size;
        self
    }

    pub fn stride(mut self, stride: Stride) -> SourceViewBuilder {
        self.inner.stride = stride;
        self
    }

    pub fn build(self) -> SourceView {
        self.inner
    }
}
