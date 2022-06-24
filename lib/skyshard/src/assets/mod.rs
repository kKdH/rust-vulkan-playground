use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::fs::{read, read_dir};
use ::gltf::buffer::Source;
use ::gltf::mesh::util::{ReadIndices, ReadTexCoords};
use log::{debug, info};
use nalgebra::{inf, Vector3};

use thiserror::Error;
use crate::Engine;

mod gltf;

type Result<T, E = AssetsManagerError> = ::std::result::Result<T, E>;

pub struct AssetsManager {
    base_dir: PathBuf,
    assets: HashMap<String, AssetResource>
}

impl AssetsManager {

    pub fn new(base_dir: &String) -> Result<AssetsManager> {
        let base_dir_path = PathBuf::from(base_dir);

        if base_dir_path.exists() {
            let file_path = base_dir_path.join("cube2.gltf");
            let file_path_str = file_path.to_str().expect("Failed to resolve path");

            let gltf = ::gltf::Gltf::open(file_path_str)
                .map_err(|error| AssetsManagerError::OpenFileError { path: file_path_str.to_string() })?;

            let nodes = gltf.scenes()
                .flat_map(|scene| scene.nodes())
                .fold(HashMap::<String, AssetResource>::new(), | mut result, node | {
                    let name = node.name()
                        .expect("Node does not have a name")
                        .to_string();
                    result.insert(Clone::clone(&name), AssetResource::Node {
                        file_path: Box::from(file_path.as_path()),
                        name: Clone::clone(&name)
                    });
                    debug!("Added asset '{}' from file '{}'.", &name, file_path_str);
                    result
                });

            Ok(AssetsManager {
                base_dir: base_dir_path,
                assets: nodes,
            })
        }
        else {
            Err(AssetsManagerError::BaseDirNotFoundError { path: Clone::clone(base_dir) })
        }
    }

    pub fn load_node(&self, name: &String) -> Result<Node> {
        match self.assets.get(name.as_str()) {
            None => { todo!() }
            Some(AssetResource::Node { name, file_path}) => {
                let path = file_path.to_str().expect("Failed to resolve path");
                match ::gltf::Gltf::open(path) {
                    Ok(gltf) => {
                        Ok(Node::new(Clone::clone(name), gltf).expect("Failed to load node"))
                    }
                    Err(_) => { todo!() }
                }
            }
            _ => { todo!() }
        }
    }
}

pub fn load_node<A, B>(engine: &Engine, name: &String, mut load_fn: A) -> Result<B>
where A: FnMut(&Node) -> B {
    match engine.asset_manager().assets.get(name.as_str()) {
        None => { todo!() }
        Some(AssetResource::Node { name, file_path}) => {
            let path = file_path.to_str().expect("Failed to resolve path");
            match ::gltf::Gltf::open(path) {
                Ok(gltf) => {
                    let node = Node::new(Clone::clone(name), gltf).expect("Failed to load node");
                    Ok(load_fn(&node))
                }
                Err(_) => { todo!() }
            }
        }
        _ => { todo!() }
    }
}

#[derive(Error, Debug)]
pub enum AssetsManagerError {

    #[error("Failed to instantiate AssetsManager!")]
    AssetsManagerInstantiationError,

    #[error("The AssetsManager's base directory '{path}' does not exists!")]
    BaseDirNotFoundError { path: String },

    #[error("Failed to open file '{path}'!")]
    OpenFileError { path: String },

    #[error("Asset '{name}' not found!")]
    AssetNotFound { name: String },
}

pub enum AssetResource {
    Node {
        name: String,
        file_path: Box<Path>,
    }
}

pub struct Node {
    name: String,
    data: NodeData,
}

impl Node {

    fn new(name: String, data: ::gltf::Gltf) -> Result<Node> {
        let buffers = gltf::load_buffers(&data);
        let node_data = data.nodes()
            .find(|node| {
                match node.name() {
                    None => false,
                    Some(actual_name) => name.as_str() == actual_name
                }
            })
            .map(|node| {
                node.mesh().map(|mesh| {

                    let primitive = &mesh.primitives().collect::<Vec<::gltf::Primitive>>()[0];
                    let reader = primitive.reader(|buffer| Some(&buffers[buffer.index()]));
                    let read_positions = reader.read_positions().expect("Failed to read positions");
                    let positions_count = read_positions.len();
                    let read_indices = reader.read_indices().expect("Failed to read indices");
                    let read_texture_coordinates = reader.read_tex_coords(0).expect("Failed to read texture coordinates");

                    let indices: Vec<u32> = match read_indices {
                        ReadIndices::U8(indices) => indices.map(|index| index as u32).collect(),
                        ReadIndices::U16(indices) => indices.map(|index| index as u32).collect(),
                        ReadIndices::U32(indices) => indices.map(|index| index as u32).collect()
                    };

                    let positions: Vec<[f32; 3]> = read_positions
                        .fold(Vec::<[f32; 3]>::with_capacity(positions_count), |mut result, position| {
                            result.push(position);
                            result
                        });

                    let texture_coordinates: Vec<[f32; 2]> = match read_texture_coordinates {
                        ReadTexCoords::U8(texture_coordinates) => todo!(),
                        ReadTexCoords::U16(texture_coordinates) => todo!(),
                        ReadTexCoords::F32(texture_coordinates) => {
                            let count = texture_coordinates.len();
                            texture_coordinates
                                .fold(Vec::<[f32; 2]>::with_capacity(2 * count), |mut result, texture_coordinate| {
                                    result.push(texture_coordinate);
                                    result
                                })
                        },
                    };

                    NodeData {
                        mesh: MeshData {
                            indices,
                            positions,
                            texture_coordinates: Vec::new(),
                        }
                    }
                }).expect("Failed to parse mesh data")
            })
            .expect("Failed to parse data");

        Ok(Node {
            name,
            data: node_data,
        })
    }

    pub fn name(&self) -> &String {
        &self.name
    }

    pub fn mesh(&self) -> &MeshData {
        &self.data.mesh
    }
}

pub struct NodeData {
    mesh: MeshData,
}

pub struct MeshData {
    pub indices: Vec<u32>,
    pub positions: Vec<[f32; 3]>,
    pub texture_coordinates: Vec<[f32; 2]>,
}
