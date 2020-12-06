use std::collections::HashMap;
use std::iter;

use crate::transform::{Offset, SinkView, Sink, Size, Source, SourceViewId};
use crate::transform::source::{SourceSet, SourceView};


pub enum ByteTransformerError {

}

pub struct ByteTransformer<'a> {
    transformations: Vec<Box<dyn Transformation + 'a>>,
    sink_view: SinkView,
}

impl<'a> ByteTransformer<'a> {

    pub fn transform(&self, source_set: &SourceSet, sink: &mut Sink) -> Result<(), ByteTransformerError> {
        let mut state = TransformationState::new();
        self.transformations.iter().for_each(|transformation| {
            transformation.apply(&mut state, source_set, &self.sink_view, sink)
        });
        Ok(())
    }

    pub fn builder() -> ByteTransformerBuilder<'a> {
        ByteTransformerBuilder {
            inner: ByteTransformer {
                transformations: Vec::new(),
                sink_view: SinkView::default()
            },
        }
    }
}

pub struct ByteTransformerBuilder<'a> {
    inner: ByteTransformer<'a>
}

impl<'a> ByteTransformerBuilder<'a> {

    pub fn repeat(self, source: SourceView) -> RepeatTransformationBuilder<'a, ByteTransformerBuilder<'a>> {
        RepeatTransformationBuilder::<ByteTransformerBuilder>::new(self, source)
    }

    pub fn append(mut self, source: SourceView) -> ByteTransformerBuilder<'a> {
        self.inner.transformations.push(Box::new(AppendTransformation {
            source
        }));
        self
    }

    pub fn to(mut self, sink: SinkView) -> ByteTransformer<'a> {
        self.inner.sink_view = sink;
        self.inner
    }
}

pub struct TransformationState {
    positions: HashMap<SourceViewId, Offset>
}

impl TransformationState {

    pub fn new() -> TransformationState {
        TransformationState {
            positions: HashMap::new()
        }
    }

    pub fn position(&self, view: SourceView) -> Offset {
        *self.positions.get(view.id()).unwrap_or(view.offset())
    }

    pub fn update_position(&mut self, view: SourceView, increment: Offset) {
        self.positions.entry(*view.id())
            .and_modify(|value| { *value += increment})
            .or_insert(view.offset() + increment);
    }
}

pub trait Transformation {
    fn apply(&self, state: &mut TransformationState, source_set: &SourceSet, sink_view: &SinkView, sink: &mut Sink);
}

struct RepeatTransformation<'a> {
    transformations: Vec<Box<dyn Transformation + 'a>>,
    source: SourceView,
}

impl Transformation for RepeatTransformation<'_> {

    fn apply(&self, state: &mut TransformationState, source_set: &SourceSet, sink_view: &SinkView, sink: &mut Sink) {
        while state.position(self.source) < *self.source.size() {
            self.transformations.iter().for_each(|transformation| {
                transformation.apply(state, source_set, sink_view, sink)
            });
        }
    }
}

pub struct RepeatTransformationBuilder<'a, P: TransformationBuilderParent<'a>> {
    parent: P,
    inner: RepeatTransformation<'a>
}

impl<'a, P: TransformationBuilderParent<'a>> RepeatTransformationBuilder<'a, P> {

    fn new(parent: P, source: SourceView) -> RepeatTransformationBuilder<'a, P> {
        RepeatTransformationBuilder {
            parent,
            inner: RepeatTransformation {
                transformations: Vec::new(),
                source,
            }
        }
    }

    pub fn repeat(mut self, source: SourceView) -> RepeatTransformationBuilder<'a, RepeatTransformationBuilder<'a, P>> {
        RepeatTransformationBuilder::<RepeatTransformationBuilder<P>>::new(self, source)
    }

    pub fn once(mut self, source: SourceView) -> OnceTransformationBuilder<'a, RepeatTransformationBuilder<'a, P>> {
        OnceTransformationBuilder::<RepeatTransformationBuilder<P>>::new(self, source)
    }

    pub fn take(mut self, amount: Size) -> RepeatTransformationBuilder<'a, P> {
        self.inner.transformations.push(Box::new(TakeTransformation::new(self.inner.source, amount)));
        self
    }

    pub fn done(mut self) -> P {
        let mut parent = self.parent;
        parent.push(Box::new(self.inner));
        parent
    }
}

struct OnceTransformation<'a> {
    transformations: Vec<Box<dyn Transformation + 'a>>,
    source: SourceView,
}

impl Transformation for OnceTransformation<'_> {
    fn apply(&self, state: &mut TransformationState, source_set: &SourceSet, sink_view: &SinkView, sink: &mut Sink) {
        self.transformations.iter().for_each(|transformation| {
            transformation.apply(state, source_set, sink_view, sink)
        });
    }
}

pub struct OnceTransformationBuilder<'a, P: TransformationBuilderParent<'a>> {
    parent: P,
    inner: OnceTransformation<'a>
}

impl<'a, P: TransformationBuilderParent<'a>> OnceTransformationBuilder<'a, P> {

    pub fn new(parent: P, source: SourceView) -> OnceTransformationBuilder<'a, P> {
        OnceTransformationBuilder {
            parent,
            inner: OnceTransformation {
                transformations: Vec::new(),
                source
            }
        }
    }

    pub fn take(mut self, amount: Size) -> OnceTransformationBuilder<'a, P> {
        self.inner.transformations.push(Box::new(TakeTransformation::new(self.inner.source, amount)));
        self
    }

    pub fn done(mut self) -> P {
        let mut parent = self.parent;
        parent.push(Box::new(self.inner));
        parent
    }
}

pub trait TransformationBuilderParent<'a> {
    fn push(&mut self, transformation: Box<(dyn Transformation + 'a)>);
}

impl<'a> TransformationBuilderParent<'a> for ByteTransformerBuilder<'a>{
    fn push(&mut self, transformation: Box<(dyn Transformation + 'a)>) {
        self.inner.transformations.push(transformation)
    }
}

impl<'a, P: TransformationBuilderParent<'a>> TransformationBuilderParent<'a> for RepeatTransformationBuilder<'a, P> {
    fn push(&mut self, transformation: Box<(dyn Transformation + 'a)>) {
        self.inner.transformations.push(transformation);
    }
}

impl<'a, P: TransformationBuilderParent<'a>> TransformationBuilderParent<'a> for OnceTransformationBuilder<'a, P> {
    fn push(&mut self, transformation: Box<(dyn Transformation + 'a)>) {
        self.inner.transformations.push(transformation);
    }
}

struct AppendTransformation {
    source: SourceView
}

impl Transformation for AppendTransformation {
    fn apply(&self, state: &mut TransformationState, source_set: &SourceSet, sink_view: &SinkView, sink: &mut Sink) {
        let source: &Source = source_set.source(*self.source.source());
        let offset: Offset = state.position(self.source);
        let amount: Size = *self.source.size();
        let mut buffer: Vec<u8> = iter::repeat(0).take(amount).collect();

        source.read(offset, amount, &mut buffer);
        sink.write(0, amount, &buffer);

        state.update_position(self.source, amount);
    }
}

struct TakeTransformation {
    source: SourceView,
    amount: Size,
}

impl TakeTransformation {

    fn new(source: SourceView, amount: Size) -> TakeTransformation {
        TakeTransformation {
            source,
            amount,
        }
    }
}

impl Transformation for TakeTransformation {
    
    fn apply(&self, state: &mut TransformationState, source_set: &SourceSet, sink_view: &SinkView, sink: &mut Sink) {
        let source: &Source = source_set.source(*self.source.source());
        let offset: Offset = state.position(self.source);
        let mut buffer: Vec<u8> = iter::repeat(0).take(self.amount).collect();

        source.read(offset, self.amount, &mut buffer);
        sink.write(0, self.amount, &buffer);
        state.update_position(self.source, self.amount)
    }
}

#[cfg(test)]
#[allow(non_snake_case)]
mod test {
    use hamcrest2::prelude::*;

    use crate::transform::{Sink, SinkView, Source, SourceView};
    use crate::transform::source::SourceSet;
    use crate::transform::transformer::ByteTransformer;

    #[test]
    fn test() {

        let mut dst_vec = Vec::new();

        {
            let src_vec1 = vec![1, 1, 1, 1, 2, 2, 2, 2, 3, 3, 3, 3];
            let src_vec2 = vec![4, 4, 4, 4, 4, 4, 4, 4];

            let source1 = Source::new(&src_vec1);
            let source2 = Source::new(&src_vec2);
            let mut sink = Sink::new(&mut dst_vec);

            let s1 = SourceView::builder()
                .id(1)
                .source(0)
                .size(4)
                .build();

            let s2 = SourceView::builder()
                .id(2)
                .source(0)
                .offset(4)
                .size(4)
                .build();

            let s3 = SourceView::builder()
                .id(3)
                .source(0)
                .offset(8)
                .size(4)
                .build();

            let s4 = SourceView::builder()
                .id(4)
                .source(1)
                .size(4)
                .build();

            let sinkView = SinkView::default();

            let source_set = SourceSet::new(&[
                (0, &source1),
                (1, &source2)
            ]);

            let transformer = ByteTransformer::builder()
                .repeat(s1)
                    .take(1)
                    .once(s2)
                        .take(1)
                    .done()
                    .once(s4)
                        .take(2)
                    .done()
                .done()
                .append(s3)
                .to(sinkView);

            assert_that!(transformer.transform(&source_set, &mut sink).is_ok(), is(true));
        }

        assert_that!(dst_vec, is(equal_to(vec![
            1, 2, 4, 4,
            1, 2, 4, 4,
            1, 2, 4, 4,
            1, 2, 4, 4,
            3, 3, 3, 3
        ])));
    }
}
