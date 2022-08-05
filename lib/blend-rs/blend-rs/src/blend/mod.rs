mod reader;
mod util;

use std::marker::PhantomData;
use std::num::NonZeroUsize;
use std::str::Utf8Error;
use blend_inspect_rs::{Address, AddressLike, Version};

pub use reader::{read, Reader, ReadError};
pub use util::{StringLike, NameLike};

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

    fn cast_to<B>(&self) -> Pointer<B, SIZE> {
        Pointer::new(self.value)
    }
}

pub trait PointerLike<A> {

    fn address(&self) -> Option<Address>;

    fn is_valid(&self) -> bool;

    fn is_invalid(&self) -> bool {
        !self.is_valid()
    }
}

impl <A, const SIZE: usize> PointerLike<A> for Pointer<A, SIZE> {

    fn address(&self) -> Option<Address> {
        (&self).address()
    }

    fn is_valid(&self) -> bool {
        (&self).is_valid()
    }
}

impl <A, const SIZE: usize> PointerLike<A> for &Pointer<A, SIZE> {

    fn address(&self) -> Option<Address> {
        let result = self.value.iter().enumerate().fold(0usize, |result, (index, value)| {
            result + ((*value as usize) << (8 * index))
        });
        Address::new(result)
    }

    fn is_valid(&self) -> bool {
        self.value.iter().sum::<u8>() > 0
    }
}

#[derive(Debug)]
pub struct Function<const SIZE: usize> {
    pub value: [u8; SIZE]
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
    use crate::blend::{read, NameLike, PointerLike, Void, Pointer};
    use crate::blender3_0::{Mesh, Object};

    #[test]
    fn test() {

        let blend_data = std::fs::read("test/resources/cube.blend").unwrap();

        let reader = read(&blend_data).unwrap();

        let objects: Vec<&Object> = reader.structs::<Object>().collect();
        let cube = objects.iter()
            .find(|object| object.id.name.to_name_str_unchecked() == "Cube")
            .unwrap();

        let parent = *reader.deref(&cube.parent).collect::<Vec<&Object>>().first().unwrap();
        println!("Parent: {}", parent.id.name.to_name_str_unchecked());

        let mesh = *reader.deref(&cube.data.cast_to::<Mesh>()).collect::<Vec<&Mesh>>().first().unwrap();
        println!("Name: {}", mesh.id.name.to_name_str().unwrap());
        reader.deref(&mesh.mvert).enumerate().for_each(|(index, vert) | {
            println!("{:?}: {:?}", index, vert.co)
        });
    }
}
