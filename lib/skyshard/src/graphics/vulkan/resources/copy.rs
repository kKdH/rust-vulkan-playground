use crate::graphics::vulkan::resources::{Resource, Buffer};


pub trait CopySource<A> {
    fn ptr(&self) -> *const A;
}

pub trait CopyDestination<A> {
    fn ptr(&mut self) -> *mut A;
}

impl <A> CopyDestination<A> for Buffer<A> {
    fn ptr(&mut self) -> *mut A {
        self.allocation().mapped_ptr()
            .expect("expected host visible memory")
            .as_ptr() as *mut A
    }
}

impl <A> CopySource<A> for Buffer<A> {
    fn ptr(&self) -> *const A {
        self.allocation().mapped_ptr()
            .expect("expected host visible memory")
            .as_ptr() as *const A
    }
}

impl <A> CopySource<A> for Vec<A> {
    fn ptr(&self) -> *const A {
        self.as_ptr()
    }
}

impl <A, const N: usize> CopySource<A> for [A; N] {
    fn ptr(&self) -> *const A {
        self.as_ptr()
    }
}

impl <A> CopyDestination<A> for Vec<A> {
    fn ptr(&mut self) -> *mut A {
        self.as_mut_ptr()
    }
}
