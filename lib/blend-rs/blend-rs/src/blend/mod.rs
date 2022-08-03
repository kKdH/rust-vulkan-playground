mod reader;

use std::marker::PhantomData;
use std::num::NonZeroUsize;
use std::str::Utf8Error;
use blend_inspect_rs::{Address, AddressLike, Version};

pub use reader::{read, Reader, ReadError};

pub struct Void;

#[derive(Debug)]
pub struct Pointer<T, const SIZE: usize> {
    pub value: [u8; SIZE],
    phantom: PhantomData<T>
}

impl <T, const SIZE: usize> Pointer<T, SIZE> {

    pub fn new(value: [u8; SIZE]) -> Self {
        Pointer {
            value,
            phantom: Default::default()
        }
    }
}

impl <T, const SIZE: usize> AddressLike for Pointer<T, SIZE> {
    fn address(&self) -> Address {
        (&self).address()
    }
}

impl <T, const SIZE: usize> AddressLike for &Pointer<T, SIZE> {
    fn address(&self) -> Address {
        let result = self.value.iter().enumerate().fold(0usize, |result, (index, value)| {
            result + ((*value as usize) << (8 * index))
        });
        NonZeroUsize::new(result)
            .expect("Is not a valid address!")
    }
}

#[derive(Debug)]
pub struct Function<const SIZE: usize> {
    pub value: [u8; SIZE]
}

pub trait PointerLike<A> {
    fn address(&self) -> Address;
}

pub trait StringLike {
    fn to_str(&self) -> Result<&str, Utf8Error>;
    fn to_string(&self) -> Result<String, Utf8Error>;
}

impl StringLike for [i8] {

    fn to_str(&self) -> Result<&str, Utf8Error> {
        let slice: &[u8] = unsafe {
            core::slice::from_raw_parts(self.as_ptr() as *const u8, self.len())
        };
        let null = slice.iter()
            .position(|element| *element == 0x00)
            .unwrap_or(slice.len());

        std::str::from_utf8(&slice[0..null])
    }

    fn to_string(&self) -> Result<String, Utf8Error> {
        self.to_str().map(|value| String::from(value))
    }
}

pub trait GeneratedBlendStruct {
    const BLEND_VERSION: Version;
    const STRUCT_NAME: &'static str;
    const STRUCT_INDEX: usize;
}

#[cfg(feature = "blender2_7")]
pub mod blender2_7;

#[cfg(feature = "blender2_9")]
pub mod blender2_9;

#[cfg(feature = "blender3_0")]
pub mod blender3_0;

#[cfg(test)]
mod test {
    use crate::blend::{read, StringLike};
    use crate::blender3_0::{Object};

    #[test]
    fn test() {

        let blend_data = std::fs::read("test/resources/cube.blend").unwrap();

        let reader = read(&blend_data).unwrap();

        reader.structs::<Object>().for_each(|object| {
            println!("Name: {}", object.id.name.to_str().unwrap());
            // let x = reader.deref(&object.mpoly);
        });



        // println!("MPoly.address: {}", mesh.mpoly.address());
        // let mpoly_block = blend.look_up(&mesh.mpoly).unwrap();
    }
}
