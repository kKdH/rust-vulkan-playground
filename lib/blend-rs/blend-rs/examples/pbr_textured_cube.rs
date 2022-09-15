extern crate blend_rs;

use blend_rs::blend::{read, PointerLike};
use blend_rs::blend::traverse::Named;
use blend_rs::blender3_2::{Object, Mesh, MLoop, MVert, MLoopUV};

#[derive(Debug)]
pub struct Vertex {
    pub position: [f32; 3],
    pub texcords: [f32; 2],
}

fn main() {

    let blend_data = std::fs::read("examples/example-3.2.blend")
        .expect("file 'examples/example-3.2.blend' should exist and be readable");

    let reader = read(&blend_data)
        .expect("Blender file should be parsable");

    let plane: &Object = reader.iter::<Object>()
        .expect("Blender file should contains Objects")
        .find(|object| object.id.get_name() == "Cube")
        .expect("Blender file should contain an Object with name 'Cube'");

    let mesh = reader.deref_single(&plane.data.as_instance_of::<Mesh>())
        .expect("object 'Cube' should have a Mesh");

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

    println!("\nTriangles of '{}':", mesh.id.get_name());
    vertices.iter().enumerate().for_each(|(index, vertex)| {
        if index % 3 == 0 {
            println!()
        }
        println!("{:?}", vertex)
    });

    println!();
}
