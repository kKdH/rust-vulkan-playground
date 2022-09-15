use std::marker::PhantomData;
use crate::blend::{GeneratedBlendStruct, PointerLike, PointerTarget, Reader};

pub trait DoubleLinked<P> : Sized + PointerTarget<Self>
where P: PointerLike<Self> {
    fn next(&self) -> &P;
    fn prev(&self) -> &P;
}

pub struct DoubleLinkedIter<'a, D, P>
where D: 'a + DoubleLinked<P> + GeneratedBlendStruct,
      P: PointerLike<D> {
    reader: &'a Reader<'a>,
    next: Option<&'a D>,
    d_phantom: PhantomData<&'a D>,
    p_phantom: PhantomData<&'a P>,
}

impl<'a, D, P> DoubleLinkedIter<'a, D, P>
where D: 'a + DoubleLinked<P> + GeneratedBlendStruct,
      P: PointerLike<D> {

    pub fn new(reader: &'a Reader<'a>, first: &'a D) -> Self {
        DoubleLinkedIter {
            reader,
            next: Some(first),
            d_phantom: Default::default(),
            p_phantom: Default::default(),
        }
    }
}

impl <'a, D, P> Iterator for DoubleLinkedIter<'a, D, P>
where D: 'a + DoubleLinked<P> + GeneratedBlendStruct,
      P: PointerLike<D> {

    type Item = &'a D;

    fn next(&mut self) -> Option<Self::Item> {
        let result = self.next;
        self.next = self.next
            .map(|current| {
                self.reader.deref_single(&current.next().as_instance_of::<D>()).ok()
            })
            .flatten();
        result
    }
}
