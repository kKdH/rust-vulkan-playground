use std::collections::HashMap;
use std::hash::{Hash, Hasher};
use std::marker::PhantomData;

use nalgebra::Vector3;
use ordered_float::OrderedFloat;

use crate::assets::mesh::attr::AttributeSet;

pub mod attr;

type Index = u32;


#[derive(PartialEq, Eq, Hash, Clone, Debug)]
struct Coordinate {
    co: Vector3<OrderedFloat<f32>>,
    // y: OrderedFloat<f32>,
    // z: OrderedFloat<f32>,
}

impl Coordinate {

    pub fn new(x: f32, y: f32, z: f32) -> Coordinate {
        Coordinate {
            co: Vector3::new(OrderedFloat::from(x), OrderedFloat::from(y), OrderedFloat::from(z)),
            // y: OrderedFloat::from(y),
            // z: OrderedFloat::from(z),
        }
    }
}

impl From<[f32; 3]> for Coordinate {
    fn from(value: [f32; 3]) -> Self {
        Coordinate::new(value[0], value[1], value[2])
    }
}

struct Mesh {
    coordinates: AttributeSet<Vector3<OrderedFloat<f32>>>,
}

impl Mesh {

    fn new() -> Mesh {
        Mesh {
            coordinates: AttributeSet::<Vector3<OrderedFloat<f32>>>::new()
        }
    }

    pub fn builder<V, VB>() -> MeshBuilder<V, VB>
    where V: Vertex<V, VB>,
          VB: VertexBuilder<V, VB> {
        MeshBuilder::new()
    }

    pub fn insert_coordinate(&mut self, coordinate: Vector3<OrderedFloat<f32>>) -> Index {
        self.coordinates.insert(coordinate)
    }
}

trait Vertex<V, VB>
where V: Vertex<V, VB>,
      VB: VertexBuilder<V, VB> {

}

trait VertexBuilder<V, VB>
where V: Vertex<V, VB>,
      VB: VertexBuilder<V, VB> {

}

struct FubarVertex {

}

struct FubarVertexBuilder {}

impl Vertex<FubarVertex, FubarVertexBuilder> for FubarVertex {

}

impl VertexBuilder<FubarVertex, FubarVertexBuilder> for FubarVertexBuilder {

}

struct MeshBuilder<V, VB>
where V: Vertex<V, VB>,
      VB: VertexBuilder<V, VB> {
    vertices: Vec<V>,
    phantom: PhantomData<VB>
}

impl <V, VB> MeshBuilder<V, VB>
where V: Vertex<V, VB>,
      VB: VertexBuilder<V, VB> {

    pub fn new() -> MeshBuilder<V, VB> {
        MeshBuilder {
            vertices: Vec::new(),
            phantom: PhantomData::default(),
        }
    }

    pub fn start_polygon(self) -> PolygonBuilder<V, VB> {
        PolygonBuilder::new(self)
    }

    pub fn build(self) -> Mesh {
        todo!()
    }
}

struct PolygonBuilder<V, VB>
where V: Vertex<V, VB>,
      VB: VertexBuilder<V, VB> {
    parent: Option<MeshBuilder<V, VB>>
}

impl <V, VB> PolygonBuilder<V, VB>
where V: Vertex<V, VB>,
      VB: VertexBuilder<V, VB> {

    fn new(parent: MeshBuilder<V, VB>) -> PolygonBuilder<V, VB> {
        PolygonBuilder {
            parent: Some(parent)
        }
    }

    pub fn insert_vertex(mut self, vertex: V) -> Self {
        self
    }

    pub fn close(mut self) -> MeshBuilder<V, VB> {
        self.parent.take().unwrap()
    }
}

pub fn indices_with_adjacency(indices: &Vec<Index>) -> Vec<Index> {

    let third_table = compute_third_table(indices);

    let mut indices_with_adjacency: Vec<Index> = Vec::with_capacity(2 * indices.len());
    for index in 0..indices.len() {
        let edge = if index % 3 == 0 || index % 3 == 1 {
            [indices[index + 1], indices[index]]
        }
        else {
            [indices[index - 2], indices[index]]
        };
        indices_with_adjacency.push(indices[index]);
        indices_with_adjacency.push(*third_table.get(&edge).unwrap_or(&0));
    }

    indices_with_adjacency
}

fn compute_third_table(indices: &Vec<Index>) -> HashMap<[Index; 2], Index> {

    let mut third_table: HashMap<[Index; 2], Index> = HashMap::with_capacity(3 * indices.len());

    for index in 0..indices.len() {
        if index % 3 == 0 {
            third_table.insert([indices[index], indices[index + 1]], indices[index + 2]);
        }
        else if index % 3 == 1 {
            third_table.insert([indices[index], indices[index + 1]], indices[index - 1]);
        }
        else {
            third_table.insert([indices[index], indices[index - 2]], indices[index - 1]);
        }
    }

    third_table
}

#[cfg(test)]
#[allow(non_snake_case)]
mod MeshSpec {
    use nalgebra::Vector3;
    use ordered_float::OrderedFloat;

    use crate::assets::mesh::{FubarVertex, FubarVertexBuilder, Index, indices_with_adjacency, Mesh};

    #[allow(non_upper_case_globals)]
    const positions: [[f32; 3]; 12] = [
        [0.0, 0.5, 0.75],
        [0.0, -0.75, 0.0],
        [0.75, 0.5, -0.5],
        [0.0, 0.5, 0.75],
        [0.75, 0.5, -0.5],
        [-0.75, 0.5, -0.5],
        [0.75, 0.5, -0.5],
        [0.0, -0.75, 0.0],
        [-0.75, 0.5, -0.5],
        [-0.75, 0.5, -0.5],
        [0.0, -0.75, 0.0],
        [0.0, 0.5, 0.75],
    ];

    #[allow(non_upper_case_globals)]
    const uvs: [[f32; 2]; 12] = [
        [0.03333336, 0.0],
        [0.1341146, 0.6979167],
        [0.34309894, 0.9938802],
        [0.5333333, 0.0],
        [0.84309894, 0.9938802],
        [0.5390625, 0.9662761],
        [0.31380206, 0.5622396],
        [0.4817708, 0.10416669],
        [0.17187501, 0.1674478],
        [0.039062515, 0.9662761],
        [0.1341146, 0.6979167],
        [0.03333336, 0.0],
    ];

    #[test]
    fn test() {

        let mut mesh = Mesh::new();
        let indices: Vec<Index> = positions.iter()
            .map(|position| {
                mesh.insert_coordinate(Vector3::<OrderedFloat<f32>>::new(
                    OrderedFloat::from(position[0]),
                    OrderedFloat::from(position[1]),
                    OrderedFloat::from(position[2])
                ))
            })
            .collect();

        println!("Indices: {:?}", indices);
        // indices.iter()
        //     .map(|index| mesh.coordinates.get(*index))
        //     .for_each(|position| println!("{:?}", position));

        let indices = indices_with_adjacency(&indices);
        println!("Indices: {:?}", indices);

    }

    #[test]
    fn test_mesh_builder() {

        let mesh = Mesh::builder::<FubarVertex, FubarVertexBuilder>()
            .start_polygon()
                .insert_vertex(FubarVertex {})
                .insert_vertex(FubarVertex {})
                .insert_vertex(FubarVertex {})
                .close()
            .start_polygon()
                .insert_vertex(FubarVertex {})
                .close()
            .build();

    }
}

#[cfg(test)]
mod test {
    use crate::assets::mesh::{compute_third_table, Index, indices_with_adjacency};

    #[test]
    fn test_compute_third_table() {

        let indices: Vec<Index> = vec![
            0, 1, 2,
            2, 1, 3,
            3, 4, 2,
            1, 5, 3
        ];

        let third_table = compute_third_table(&indices);

        assert_eq!(third_table[&[0, 1]], 2);
        assert_eq!(third_table[&[1, 2]], 0);
        assert_eq!(third_table[&[2, 0]], 1);
        assert_eq!(third_table[&[2, 1]], 3);
        assert_eq!(third_table[&[1, 3]], 2);
        assert_eq!(third_table[&[3, 2]], 1);
        assert_eq!(third_table[&[2, 3]], 4);
        assert_eq!(third_table[&[3, 4]], 2);
        assert_eq!(third_table[&[4, 2]], 3);
        assert_eq!(third_table[&[1, 5]], 3);
        assert_eq!(third_table[&[5, 3]], 1);
        assert_eq!(third_table[&[3, 1]], 5);
    }

    #[test]
    fn test_indices_with_adjacency() {

        let indices: Vec<Index> = vec![
            0, 1, 2,
            2, 1, 3,
            3, 4, 2,
            1, 5, 3
        ];

        let result = indices_with_adjacency(&indices);

        println!("{:?}", result);
    }
}
