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

pub trait PointerLike<A> {

    fn address(&self) -> Option<Address>;

    fn is_valid(&self) -> bool;

    fn is_invalid(&self) -> bool {
        !self.is_valid()
    }
}

impl <B, const SIZE: usize> PointerLike<B> for Pointer<B, SIZE> {

    fn address(&self) -> Option<Address> {
        (&self).address()
    }

    fn is_valid(&self) -> bool {
        (&self).is_valid()
    }
}

impl <B, const SIZE: usize> PointerLike<B> for &Pointer<B, SIZE> {

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

pub trait StringLike {

    fn to_str(&self) -> Result<&str, Utf8Error>;

    fn to_str_unchecked(&self) -> &str {
        self.to_str().expect("Failed to extract &str!")
    }

    fn to_string(&self) -> Result<String, Utf8Error> {
        self.to_str().map(|value| String::from(value))
    }

    fn to_string_unchecked(&self) -> String {
        self.to_string().expect("Failed to extract String!")
    }
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
}

const NAME_PREFIXES: [&str; 17] = [
    "OB", "ME", "WM", "IM", "SN",
    "WS", "BR", "SC", "PL", "OB",
    "GR", "CA", "LA", "ME", "WO",
    "LS", "MA",
];

pub trait NameLike {

    fn to_name_str(&self) -> Result<&str, Utf8Error>;

    fn to_name_string(&self) -> Result<String, Utf8Error> {
        self.to_name_str().map(|value| String::from(value))
    }

    fn to_name_str_unchecked(&self) -> &str {
        self.to_name_str().expect("Failed to convert to name!")
    }

    fn to_name_string_unchecked(&self) -> String {
        self.to_name_string().expect("Failed to convert to name!")
    }
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
    use crate::blend::{read, NameLike, PointerLike};
    use crate::blender3_0::{Mesh, Object};

    #[test]
    fn test() {

        let blend_data = std::fs::read("test/resources/cube.blend").unwrap();

        let reader = read(&blend_data).unwrap();

        let objects: Vec<&Object> = reader.structs::<Object>().collect();
        let cube = objects.iter()
            .find(|object| object.id.name.to_name_str_unchecked() == "Cube")
            .unwrap();

        reader.structs::<Mesh>().for_each(|mesh| {
            println!("Name: {}", mesh.id.name.to_name_str().unwrap());

            reader.deref(&mesh.mvert).enumerate().for_each(|(index, vert) | {
                println!("{:?}: {:?}", index, vert.co)
            });
        });
    }
}
