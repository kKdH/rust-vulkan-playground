use std::marker::PhantomData;
use crate::blend::{GeneratedBlendStruct, PointerLike, PointerTarget, Reader};

pub trait DoubleLinked<Ptr> : Sized + PointerTarget<Self>
where Ptr: PointerLike<Self> {
    fn next(&self) -> &Ptr;
    fn prev(&self) -> &Ptr;
}

pub struct DoubleLinkedIter<'a, D, Ptr>
where D: 'a + DoubleLinked<Ptr> + GeneratedBlendStruct,
      Ptr: PointerLike<D> {
    reader: &'a Reader<'a>,
    first: &'a D,
    next: Option<&'a D>,
    d_phantom: PhantomData<&'a D>,
    p_phantom: PhantomData<&'a Ptr>,
}

impl<'a, D, Ptr> DoubleLinkedIter<'a, D, Ptr>
where D: 'a + DoubleLinked<Ptr> + GeneratedBlendStruct,
      Ptr: PointerLike<D> {

    pub fn new(reader: &'a Reader<'a>, first: &'a D) -> Self {
        DoubleLinkedIter {
            reader,
            first,
            next: Some(first),
            d_phantom: Default::default(),
            p_phantom: Default::default(),
        }
    }

    pub fn find<P>(&self, predicate: P) -> Option<&'a D>
    where P: Fn(&'a D) -> bool {
        let mut current = self.first;
        loop {
            if predicate(current) {
                return Some(current)
            }
            else {
                if let Some(next) = self.reader.deref_single(&current.next().as_instance_of::<D>()).ok() {
                    current = next
                }
                else {
                    return None
                }
            }
        }
    }
}

impl <'a, D, Ptr> Iterator for DoubleLinkedIter<'a, D, Ptr>
where D: 'a + DoubleLinked<Ptr> + GeneratedBlendStruct,
      Ptr: PointerLike<D> {

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
