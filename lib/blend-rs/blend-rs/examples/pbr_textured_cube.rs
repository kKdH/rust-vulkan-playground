extern crate blend_rs;

use blend_rs::blend::{read, PointerLike, NameLike};
use blend_rs::blender3_2::{Object, Mesh, MLoop, MVert, MLoopUV};

#[derive(Debug)]
pub struct Vertex {
    pub position: [f32; 3],
    pub texcords: [f32; 2],
}

fn main() {

    let blend_data = std::fs::read("examples/example-3.2.blend")
        .expect("file 'examples/example-3.2.blend' to be readable");

    let reader = read(&blend_data)
        .expect("Blender data should be parsable");

    let plane: &Object = reader.iter::<Object>()
        .expect("an iterator over all Objects")
        .find(|object| object.id.name.to_name_str_unchecked() == "Cube")
        .expect("an Object with name 'Cube'");

    let mesh = reader.deref_single(&plane.data.cast_to::<Mesh>())
        .expect("a Mesh of the 'Cube'");

    let mesh_loop: Vec<&MLoop> = reader.deref(&mesh.mloop).unwrap().collect();
    let mesh_vertices: Vec<&MVert> = reader.deref(&mesh.mvert).unwrap().collect();
    let mesh_uvs: Vec<&MLoopUV> = reader.deref(&mesh.mloopuv).unwrap().collect();
    let mesh_polygon = reader.deref(&mesh.mpoly).unwrap();

    let vertices: Vec<Vertex> = mesh_polygon
        .map(|polygon| {
            (polygon.loopstart..polygon.loopstart + polygon.totloop).into_iter().map(|loop_index| {
                Vertex {
                    position: mesh_vertices[mesh_loop[loop_index as usize].v as usize].co,
                    texcords: mesh_uvs[loop_index as usize].uv,
                }
            })
        })
        .flatten()
        .collect();

    println!("\nTriangles of '{}':", mesh.id.name.to_name_str_unchecked());
    vertices.iter().enumerate().for_each(|(index, vertex)| {
        if index % 3 == 0 {
            println!()
        }
        println!("{:?}", vertex)
    });

    println!();
}
