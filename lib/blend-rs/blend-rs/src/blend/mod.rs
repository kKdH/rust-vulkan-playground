mod reader;
mod util;

use std::fmt::{Debug};
use std::marker::PhantomData;

use blend_inspect_rs::Address;

pub use reader::{read, Reader, ReadError};
pub use util::{StringLike, NameLike};
pub use blend_inspect_rs::Version;


#[derive(Debug, Copy, Clone)]
pub struct Void;

#[derive(Debug,  Copy, Clone)]
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

    pub fn cast_to<B>(&self) -> Pointer<B, SIZE> {
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

#[derive(Debug, Copy, Clone)]
pub struct Function<const SIZE: usize> {
    pub value: [u8; SIZE]
}

pub trait GeneratedBlendStruct {
    const BLEND_VERSION: Version;
    const STRUCT_NAME: &'static str;
    const STRUCT_INDEX: usize;
    const STRUCT_TYPE_INDEX: usize;
}

#[cfg(test)]
mod test {
    
    use crate::blend::{read, NameLike};
    use crate::blender3_0::{bNode, bNodeSocket, bNodeTree, Image, Link, Material, Mesh, Object};

    #[test]
    fn test() {

        let blend_data = std::fs::read("test/resources/cube.blend").unwrap();
        let reader = read(&blend_data).unwrap();

        let cube: &Object = reader.structs::<Object>().unwrap()
            .find(|object| object.id.name.to_name_str_unchecked() == "Cube")
            .unwrap();

        println!("Object: {}", cube.id.name.to_name_str_unchecked());

        let parent = reader.deref(&cube.parent).unwrap().first().unwrap();
        println!("Parent: {}", parent.id.name.to_name_str_unchecked());

        let mesh = reader.deref(&cube.data.cast_to::<Mesh>()).unwrap().first().unwrap();
        println!("Mesh: {}", mesh.id.name.to_name_str().unwrap());
        reader.deref(&mesh.mloop).unwrap().enumerate().for_each(|(index, mloop)| {
            println!("{:?}: {}", index, mloop.v)
        });
        reader.deref(&mesh.mvert).unwrap().enumerate().for_each(|(index, vert) | {
            println!("{:?}: {:?}", index, vert.co)
        });

        let mat = reader.deref(&mesh.mat.cast_to::<Link>())
            .map(|links| {
                let link = links.first().unwrap();
                reader.deref(link.next.cast_to::<Material>()).unwrap()
            })
            .unwrap()
            .first()
            .unwrap();

        println!("Material: {}, use_nodes: {}", mat.id.name.to_name_str_unchecked(), &mat.use_nodes);

        let tree: &bNodeTree = reader.deref_single(&mat.nodetree)
            .unwrap();

        let node = reader.deref_single(&tree.nodes.last.cast_to::<bNode>()) // FIXME: `last` is improper.
            .unwrap();

        let base_color_socket = reader.deref_single(node.inputs.first.cast_to::<bNodeSocket>())
            .unwrap();

        let link = reader.deref_single(&base_color_socket.link)
            .unwrap();

        let tex_node = reader.deref_single(&link.fromnode)
            .unwrap();

        let tex_image = reader.deref_single(&tex_node.id.cast_to::<Image>())
            .unwrap();

        let image_packed_file = reader.deref_single(&tex_image.packedfile)
            .unwrap();

        let data = reader.deref_raw_range(&image_packed_file.data, 0..image_packed_file.size as usize)
            .unwrap();

        std::fs::write("/tmp/texture.jpg", data)
            .unwrap();
    }
}
