use std::fmt::Debug;
use std::marker::PhantomData;
use std::str::Utf8Error;

use blend_inspect_rs::Address;

pub use blend_inspect_rs::{Version, Endianness};
pub use reader::{read, Reader, ReadError, StructIter};

pub mod traverse;

mod reader;

#[derive(Debug, Copy, Clone)]
pub struct Void;

#[derive(Debug, Clone)]
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

#[derive(Debug, Copy, Clone)]
pub struct Function<const SIZE: usize> {
    pub value: [u8; SIZE]
}

pub trait GeneratedBlendStruct {
    const BLEND_VERSION: Version;
    const BLEND_POINTER_SIZE: usize;
    const BLEND_ENDIANNESS: Endianness;
    const STRUCT_NAME: &'static str;
    const STRUCT_INDEX: usize;
    const STRUCT_TYPE_INDEX: usize;
}

pub trait PointerLike<A, const SIZE: usize> : Sized {

    fn as_instance_of<B>(&self) -> Pointer<B, SIZE>;

    fn address(&self) -> Option<Address>;

    fn is_valid(&self) -> bool;

    fn is_invalid(&self) -> bool {
        !self.is_valid()
    }
}

impl <A, const SIZE: usize> PointerLike<A, SIZE> for Pointer<A, SIZE> {

    fn as_instance_of<B>(&self) -> Pointer<B, SIZE> {
        Pointer::new(self.value)
    }

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
        if !self_ref.is_empty() {
            let slice: &[u8] = unsafe {
                core::slice::from_raw_parts(self_ref.as_ptr() as *const u8, self_ref.len())
            };
            let null = slice.iter()
                .position(|element| *element == 0x00)
                .unwrap_or(slice.len());
            std::str::from_utf8(&slice[0..null])
        }
        else {
            Ok("")
        }
    }
}

pub trait NameLike {

    const NAME_PREFIXES: [&'static str; 17] = [
        "OB", "ME", "WM", "IM", "SN",
        "WS", "BR", "SC", "PL", "OB",
        "GR", "CA", "LA", "ME", "WO",
        "LS", "MA",
    ];

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
            if Self::NAME_PREFIXES.contains(&&value[0..2]) {
                &value[2..]
            }
            else {
                &value
            }
        })
    }
}

#[cfg(test)]
mod test {
    use crate::blend::{read, PointerLike, NameLike};
    use crate::blend::traverse::Named;
    use crate::blender3_2::{bNode, bNodeSocket, bNodeTree, Image, Link, Material, Mesh, MLoop, MVert, Object};

    #[test]
    fn test() {

        let blend_data = std::fs::read("examples/example-3.2.blend").unwrap();
        let reader = read(&blend_data).unwrap();

        let cube: &Object = reader.iter::<Object>().unwrap()
            .find(|object| object.id.name.to_name_str_unchecked() == "Cube")
            .unwrap();

        println!("Object: {}", cube.id.get_name());

        let parent = reader.deref(&cube.parent).unwrap().first().unwrap();

        println!("Parent: {}", parent.id.get_name());

        let mesh = reader.deref_single(&cube.data.as_instance_of::<Mesh>())
            .unwrap();

        println!("Mesh: {}", mesh.id.get_name());

        let mesh_loop: Vec<&MLoop> = reader.deref(&mesh.mloop).unwrap().collect();
        let mesh_vertices: Vec<&MVert> = reader.deref(&mesh.mvert).unwrap().collect();

        let mesh_polygon = reader.deref(&mesh.mpoly).unwrap();
        let _vertices = mesh_polygon
            .map(|polygon| {
                (polygon.loopstart..polygon.loopstart + polygon.totloop).into_iter().map(|loop_index| {
                    mesh_vertices[mesh_loop[loop_index as usize].v as usize].co
                })
            })
            .flatten()
            .collect::<Vec<[f32; 3]>>();

        // vertices.iter().for_each(|vert| println!("Vert: {:?}", vert));

        let mat = reader.deref(&mesh.mat.as_instance_of::<Link>())
            .map(|links| {
                let link = links.first().unwrap();
                reader.deref(&link.next.as_instance_of::<Material>()).unwrap()
            })
            .unwrap()
            .first()
            .unwrap();

        println!("Material: {}, use_nodes: {}", mat.id.get_name(), &mat.use_nodes);

        let tree: &bNodeTree = reader.deref_single(&mat.nodetree)
            .unwrap();

        let node = reader.deref_single(&tree.nodes.last.as_instance_of::<bNode>()) // FIXME: `last` is improper.
            .unwrap();

        let base_color_socket = reader.deref_single(&node.inputs.first.as_instance_of::<bNodeSocket>())
            .unwrap();

        let link = reader.deref_single(&base_color_socket.link)
            .unwrap();

        let tex_node = reader.deref_single(&link.fromnode)
            .unwrap();

        let tex_image = reader.deref_single(&tex_node.id.as_instance_of::<Image>())
            .unwrap();

        let image_packed_file = reader.deref_single(&tex_image.packedfile)
            .unwrap();

        let data = reader.deref_raw_range(&image_packed_file.data, 0..image_packed_file.size as usize)
            .unwrap();

        std::fs::write("/tmp/texture.jpg", data)
            .unwrap();

        let nodes = reader.traverse_double_linked(&tree.nodes.first.as_instance_of::<bNode>())
            .unwrap();

        nodes.for_each(|node| {
            println!("Node: {}", node.name.to_name_str_unchecked());
        });
    }
}
