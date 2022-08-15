use std::marker::PhantomData;
use crate::blend::{GeneratedBlendStruct, Pointer, PointerLike, Reader};

pub trait DoubleLinked<P, T>
where P: PointerLike<T> {
    fn next(&self) -> &P;
    fn prev(&self) -> &P;
}

pub struct DoubleLinkedIter<'a, D, P, T>
where D: DoubleLinked<P, T>,
      P: PointerLike<T>,
      T: 'a + GeneratedBlendStruct {
    reader: &'a Reader<'a>,
    first: &'a D,
    next: Option<&'a D>,
    d_phantom: PhantomData<&'a D>,
    p_phantom: PhantomData<&'a P>,
    t_phantom: PhantomData<&'a T>,
}

impl<'a, D, P, T> DoubleLinkedIter<'a, D, P, T>
where D: DoubleLinked<P, T>,
      P: PointerLike<T>,
      T: 'a + GeneratedBlendStruct {

    pub fn new(reader: &'a Reader<'a>, first: &'a D) -> Self {
        DoubleLinkedIter {
            reader,
            first,
            next: Some(first),
            d_phantom: Default::default(),
            p_phantom: Default::default(),
            t_phantom: Default::default(),
        }
    }
}

impl <'a, D, P, T> Iterator for &DoubleLinkedIter<'a, D, P, T>
where D: DoubleLinked<P, T>,
      P: PointerLike<T>,
      T: 'a + GeneratedBlendStruct  {

    type Item = &'a T;

    fn next(&mut self) -> Option<Self::Item> {
        todo!()
    }
}
