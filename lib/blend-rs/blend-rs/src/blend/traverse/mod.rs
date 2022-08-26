//!
//! # Traverse
//!
//! The Traverse module contains utilities for traversing some structures of a blend file.
//!
use std::marker::PhantomData;
use crate::blend::{GeneratedBlendStruct, PointerLike, Reader};

pub trait DoubleLinked<P, const SIZE: usize> : Sized
where P: PointerLike<Self, SIZE> {
    fn next(&self) -> &P;
    fn prev(&self) -> &P;
}

pub struct DoubleLinkedIter<'a, D, P, const SIZE: usize>
where D: 'a + DoubleLinked<P, SIZE> + GeneratedBlendStruct,
      P: PointerLike<D, SIZE> {
    reader: &'a Reader<'a>,
    next: Option<&'a D>,
    d_phantom: PhantomData<&'a D>,
    p_phantom: PhantomData<&'a P>,
}

impl<'a, D, P, const SIZE: usize> DoubleLinkedIter<'a, D, P, SIZE>
where D: 'a + DoubleLinked<P, SIZE> + GeneratedBlendStruct,
      P: PointerLike<D, SIZE> {

    pub fn new(reader: &'a Reader<'a>, first: &'a D) -> Self {
        DoubleLinkedIter {
            reader,
            next: Some(first),
            d_phantom: Default::default(),
            p_phantom: Default::default(),
        }
    }
}

impl <'a, D, P, const SIZE: usize> Iterator for DoubleLinkedIter<'a, D, P, SIZE>
where D: 'a + DoubleLinked<P, SIZE> + GeneratedBlendStruct,
      P: PointerLike<D, SIZE> {

    type Item = &'a D;

    fn next(&mut self) -> Option<Self::Item> {
        let result = self.next;
        self.next = self.next
            .map(|current| {
                self.reader.deref_single(&current.next().cast_to::<D>()).ok()
            })
            .flatten();
        result
    }
}
