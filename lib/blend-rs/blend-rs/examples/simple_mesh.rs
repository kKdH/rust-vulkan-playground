extern crate blend_rs;

use blend_rs::blend::{read, PointerLike, NameLike};
use blend_rs::blender3_0::{Object, Mesh, MLoop, MVert};

#[derive(Debug)]
pub struct Vertex {
    pub position: [f32; 3],
}

fn main() {

    let blend_data = std::fs::read("examples/example-3.2.blend")
        .expect("Blend file not found!");

    let reader = read(&blend_data)
        .expect("Failed to read blend data!");

    let plane: &Object = reader.iter::<Object>().unwrap()
        .find(|object| object.id.name.to_name_str_unchecked() == "Plane")
        .unwrap();

    let mesh = reader.deref_single(&plane.data.cast_to::<Mesh>())
        .expect("Could not get mesh from object!");

    let mesh_polygon = reader.deref(&mesh.mpoly)
        .expect("Could not get polygons from mesh!");

    let mesh_loop: Vec<&MLoop> = reader.deref(&mesh.mloop)
        .expect("Could not get loops from mesh!")
        .collect();

    let mesh_vertices: Vec<&MVert> = reader.deref(&mesh.mvert)
        .expect("Could not get vertices from mesh!")
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

    println!("\nPolygons ({:?}) of '{}':", polygon_count, plane.id.name.to_name_str_unchecked());
    vertices_per_polygon.iter().enumerate().for_each(|(index, vertices)| {
        println!();
        vertices.iter().for_each(|vertex| {
            println!("{:?}", vertex)
        });
    });

    println!()
}
