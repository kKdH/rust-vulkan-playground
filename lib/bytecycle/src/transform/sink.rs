use crate::transform::{Offset, Size};
use crate::transform::writeable::Writeable;

pub struct Sink<'a> {
    writeable: &'a mut dyn Writeable
}

impl<'a> Sink<'a> {

    pub fn new(writeable: &'a mut dyn Writeable) -> Sink {
        Sink {
            writeable
        }
    }

    pub fn write(&mut self, offset: Offset, amount: Offset, buffer: &[u8]) {
        self.writeable.write(offset, amount, buffer);
    }
}

#[derive(Debug, Copy, Clone)]
pub struct SinkView {
    offset: Offset,
    size: Size,
}

impl SinkView {

    pub fn builder() -> SinkViewBuilder {
        SinkViewBuilder {
            inner: SinkView::default()
        }
    }
}

impl Default for SinkView {
    fn default() -> Self {
        SinkView {
            offset: 0,
            size: 0
        }
    }
}

pub struct SinkViewBuilder {
    inner: SinkView
}

impl SinkViewBuilder {

    pub fn offset(mut self, offset: Offset) -> SinkViewBuilder {
        self.inner.offset = offset;
        self
    }

    pub fn size(mut self, size: Size) -> SinkViewBuilder {
        self.inner.size = size;
        self
    }

    pub fn build(self) -> SinkView {
        self.inner
    }
}
