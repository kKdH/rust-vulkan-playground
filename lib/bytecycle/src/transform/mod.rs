pub use source::{Source, SourceSet, SourceView, SourceViewBuilder};
pub use sink::{Sink, SinkView, SinkViewBuilder};
pub use transformer::ByteTransformer;

mod source;
mod sink;
mod transformer;
mod readable;
mod writeable;

type SourceId = usize;
type SourceViewId = usize;
type Offset = usize;
type Size = usize;
type Stride = usize;
