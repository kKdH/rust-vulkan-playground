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
        .expect("file 'examples/example-3.2.blend' to be readable");

    let reader = read(&blend_data)
        .expect("Blender data should be parsable");

    let plane: &Object = reader.iter::<Object>()
        .expect("an iterator over all Objects")
        .find(|object| object.id.get_name() == "Plane")
        .expect("an Object with name 'Plane'");

    let mesh = reader.deref_single(&plane.data.as_instance_of::<Mesh>())
        .expect("the Mesh of the 'Plane'");

    let mesh_polygon = reader.deref(&mesh.mpoly)
        .expect("an iterator over all polygons of the Mesh");

    let mesh_loop: Vec<&MLoop> = reader.deref(&mesh.mloop)
        .expect("an iterator over all loops of the Mesh")
        .collect();

    let mesh_vertices: Vec<&MVert> = reader.deref(&mesh.mvert)
        .expect("an iterator over all vertices of the Mesh")
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
