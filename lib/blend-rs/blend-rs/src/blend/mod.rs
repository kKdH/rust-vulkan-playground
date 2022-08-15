mod reader;
mod util;

pub mod traverse;

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
    use crate::blender3_0::{bNode, bNodeSocket, bNodeTree, Image, Link, Material, Mesh, MLoop, MVert, Object};

    #[test]
    fn test() {

        let blend_data = std::fs::read("examples/example-3.2.blend").unwrap();
        let reader = read(&blend_data).unwrap();

        let cube: &Object = reader.iter::<Object>().unwrap()
            .find(|object| object.id.name.to_name_str_unchecked() == "Cube")
            .unwrap();

        println!("Object: {}", cube.id.name.to_name_str_unchecked());

        let parent = reader.deref(&cube.parent).unwrap().first().unwrap();

        println!("Parent: {}", parent.id.name.to_name_str_unchecked());

        let mesh = reader.deref_single(&cube.data.cast_to::<Mesh>())
            .unwrap();

        println!("Mesh: {}", mesh.id.name.to_name_str_unchecked());

        let mesh_loop: Vec<&MLoop> = reader.deref(&mesh.mloop).unwrap().collect();
        let mesh_vertices: Vec<&MVert> = reader.deref(&mesh.mvert).unwrap().collect();

        let mesh_uv_loop = reader.deref(&mesh.mloopuv).unwrap();
        let mesh_polygon = reader.deref(&mesh.mpoly).unwrap();
        let vertices = mesh_polygon
            .map(|polygon| {
                (polygon.loopstart..polygon.loopstart + polygon.totloop).into_iter().map(|loop_index| {
                    mesh_vertices[mesh_loop[loop_index as usize].v as usize].co
                })
            })
            .flatten()
            .collect::<Vec<[f32; 3]>>();

        vertices.iter().for_each(|vert| println!("Vert: {:?}", vert));

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

        let x = reader.traverse(&tree.nodes.first.cast_to::<bNode>())
            .unwrap();

        x.for_each(|node| {

        });


    }

    fn x(name: &str, start: &bNode) -> Option<bNode> {
        todo!()
    }
}
