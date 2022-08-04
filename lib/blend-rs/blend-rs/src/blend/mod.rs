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

impl <A> StringLike for A
where A: AsRef<[i8]> {

    fn to_str(&self) -> Result<&str, Utf8Error> {
        let self_ref = self.as_ref();
        let slice: &[u8] = unsafe {
            core::slice::from_raw_parts(self_ref.as_ptr() as *const u8, self_ref.len())
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

const NAME_PREFIXES: [&str; 17] = [
    "OB", "ME", "WM", "IM", "SN",
    "WS", "BR", "SC", "PL", "OB",
    "GR", "CA", "LA", "ME", "WO",
    "LS", "MA",
];

pub trait NameLike {
    fn to_name_str(&self) -> Result<&str, Utf8Error>;
    fn to_name_string(&self) -> Result<String, Utf8Error>;
}

impl <A> NameLike for A
where A: StringLike {

    fn to_name_str(&self) -> Result<&str, Utf8Error> {
        self.to_str().map(|value| {
            if NAME_PREFIXES.contains(&&value[0..2]) {
                &value[2..]
            }
            else {
                &value
            }
        })
    }

    fn to_name_string(&self) -> Result<String, Utf8Error> {
        self.to_name_str().map(|value| String::from(value))
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
    use crate::blend::{read, NameLike};
    use crate::blender3_0::{Mesh};

    #[test]
    fn test() {

        let blend_data = std::fs::read("test/resources/cube.blend").unwrap();

        let reader = read(&blend_data).unwrap();

        reader.structs::<Mesh>().for_each(|object| {
            println!("Name: {}", object.id.name.to_name_str().unwrap());
            // let x = reader.deref(&object.mpoly);
        });

        // println!("MPoly.address: {}", mesh.mpoly.address());
        // let mpoly_block = blend.look_up(&mesh.mpoly).unwrap();
    }
}
