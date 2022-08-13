extern crate blend_rs;

#[derive(Debug)]
pub struct Vertex {
    pub position: [f32; 3],
}

fn main() {
    use blend_rs::blend::{read, NameLike};
    use blend_rs::blender3_0::{Object, Mesh};

    let blend_data = std::fs::read("test/resources/cube.blend")
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

    let vertices: Vec<Vertex> = reader.deref(&mesh.mvert)
     .expect("Failed to deref 'mvert' pointer!")
     .map(|vert| {
         Vertex {
             position: vert.co
         }
     })
     .collect();

    println!("\nVertices of mesh '{}':", mesh.id.name.to_name_str_unchecked());
    vertices.iter().enumerate().for_each(|(index, vertex)| {
        println!("{}: {:?}", index, vertex);
    });
}
