use std::marker::PhantomData;
use std::num::NonZeroUsize;
use blend_inspect_rs::{Address, AddressLike};

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

#[cfg(feature = "blender2_7")]
pub mod blender2_7;

#[cfg(feature = "blender2_9")]
pub mod blender2_9;

#[cfg(feature = "blender3_0")]
pub mod blender3_0;

#[cfg(test)]
mod test {
    use std::{mem, str};
    use std::ffi::CStr;
    use bytemuck::cast_slice;

    use hamcrest2::{assert_that, equal_to, is};
    use hamcrest2::HamcrestMatcher;

    use blend_inspect_rs::{analyse, Blend, BlendFile, Identifier, Mode, parse, AddressLike};
    use blend_inspect_rs::Type::Pointer;
    use crate::blender3_0::Mesh;
    use crate::blend;

    #[test]
    fn test() {

        let blend_data = std::fs::read("test/resources/cube.blend").unwrap();
        let blend: BlendFile = parse(blend_data.as_slice()).unwrap();

        let mesh_block = blend.blocks.iter()
            .find(|block| Identifier::ME == block.identifier)
            .unwrap();

        let blend_data = blend_data.as_slice();
        let mesh_data = &blend_data[mesh_block.data_location()..mesh_block.data_location() + mem::size_of::<Mesh>()];
        let (_, body, _) = unsafe { mesh_data.align_to::<blend::blender3_0::Mesh>() };
        let mesh = &body[0];
        let mesh_name_data: &[u8] = cast_slice(mesh.id.name.as_slice());

        let mesh_name = str::from_utf8(mesh_name_data).unwrap();
        println!("Name: {}", mesh_name);
        println!("MPoly.address: {}", mesh.mpoly.address());

        let mpoly_block = blend.look_up(&mesh.mpoly).unwrap();


    }
}
