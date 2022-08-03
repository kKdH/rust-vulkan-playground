mod reader;

use std::marker::PhantomData;
use std::num::NonZeroUsize;
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
    use std::{str};

    use bytemuck::cast_slice;
    use blend_inspect_rs::{BlendFile, parse};
    use crate::blend::read;

    use crate::blender3_0::{Object};

    #[test]
    fn test() {

        let blend_data = std::fs::read("test/resources/cube.blend").unwrap();
        let _blend: BlendFile = parse(&blend_data).unwrap();

        let reader = read(&blend_data).unwrap();

        let x = reader.structs::<Object>();
        x.for_each(|object| {
            let mesh_name_data: &[u8] = cast_slice(object.id.name.as_slice());
            let mesh_name = str::from_utf8(mesh_name_data).unwrap();
            println!("Name: {}", mesh_name);
        });

        // println!("MPoly.address: {}", mesh.mpoly.address());
        // let mpoly_block = blend.look_up(&mesh.mpoly).unwrap();
    }
}
