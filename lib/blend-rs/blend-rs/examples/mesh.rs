extern crate blend_rs;

use blend_rs::blend::{read, NameLike};
use blend_rs::blender3_0::{Object, Mesh, MLoop, MVert};

#[derive(Debug)]
pub struct Vertex {
    pub position: [f32; 3],
}

fn main() {

    let blend_data = std::fs::read("examples/example-3.2.blend")
        .expect("Failed to open file!");

    let reader = read(&blend_data)
        .expect("Failed to read blend data!");

    let cube: &Object = reader.structs::<Object>()
         .expect("Failed to create a StructIter for Object!")
         .find(|object| object.id.name.to_name_str_unchecked() == "Cube")
         .expect("Cube could not be found!");

    let mesh: &Mesh = reader.deref(&cube.data.cast_to::<Mesh>())
         .expect("Failed to deref 'data' pointer!")
         .first()
         .expect("Expected at least one element!");

    let mesh_loop: Vec<&MLoop> = reader.deref(&mesh.mloop).unwrap().collect();
    let mesh_vertices: Vec<&MVert> = reader.deref(&mesh.mvert).unwrap().collect();
    let mesh_polygon = reader.deref(&mesh.mpoly).unwrap();

    let vertices: Vec<Vertex> = mesh_polygon
        .map(|polygon| {
            (polygon.loopstart..polygon.loopstart + polygon.totloop).into_iter().map(|loop_index| {
                Vertex {
                    position: mesh_vertices[mesh_loop[loop_index as usize].v as usize].co
                }
            })
        })
        .flatten()
        .collect();

    println!("\nTriangles of mesh '{}':", mesh.id.name.to_name_str_unchecked());
    vertices.iter().enumerate().for_each(|(index, vertex)| {
        if index % 3 == 0 {
            println!()
        }
        println!("{:?}", vertex)
    });

    println!();
}
