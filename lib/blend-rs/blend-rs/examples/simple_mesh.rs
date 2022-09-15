extern crate blend_rs;

use blend_rs::blend::{read, PointerLike};
use blend_rs::blend::traverse::Named;
use blend_rs::blender3_2::{Object, Mesh, MLoop, MVert};

#[derive(Debug)]
pub struct Vertex {
    pub position: [f32; 3],
}

fn main() {

    let blend_data = std::fs::read("examples/example-3.2.blend")
        .expect("file 'examples/example-3.2.blend' should exist and be readable");

    let reader = read(&blend_data)
        .expect("Blender file should be parsable");

    let plane: &Object = reader.iter::<Object>()
        .expect("Blender file should contains Objects")
        .find(|object| object.id.get_name() == "Plane")
        .expect("Blender file should contain an Object with name 'Plane'");

    let mesh = reader.deref_single(&plane.data.as_instance_of::<Mesh>())
        .expect("object 'Plane' should have a Mesh");

    let mesh_polygon = reader.deref(&mesh.mpoly)
        .expect("mesh of object 'Plane' should have polygons");

    let mesh_loop: Vec<&MLoop> = reader.deref(&mesh.mloop)
        .expect("mesh of object 'Plane' should have loops")
        .collect();

    let mesh_vertices: Vec<&MVert> = reader.deref(&mesh.mvert)
        .expect("mesh of object 'Plane' should have vertices")
        .collect();

    let polygon_count = mesh_polygon.len();

    let vertices_per_polygon: Vec<Vec<Vertex>> = mesh_polygon
        .map(|polygon| {
            (polygon.loopstart..polygon.loopstart + polygon.totloop).into_iter()
                .map(|loop_index| {
                    Vertex {
                        position: mesh_vertices[mesh_loop[loop_index as usize].v as usize].co,
                    }
                })
                .collect()
        })
        .collect();

    println!("\nPolygons ({:?}) of '{}':", polygon_count, plane.id.get_name());
    vertices_per_polygon.iter().enumerate().for_each(|(index, vertices)| {
        println!();
        vertices.iter().for_each(|vertex| {
            println!("{:?}", vertex)
        });
    });

    println!()
}
