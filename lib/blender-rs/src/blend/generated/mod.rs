use std::marker::PhantomData;

pub mod blender3_0;

pub struct Void;

pub struct Pointer<T, const SIZE: usize> {
    value: [u8; SIZE],
    phantom: PhantomData<T>
}
