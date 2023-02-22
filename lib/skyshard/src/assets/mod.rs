use std::collections::HashMap;
use std::path::{Path, PathBuf};

use thiserror::Error;

pub mod mesh;

type Result<T, E = AssetsManagerError> = ::std::result::Result<T, E>;

pub struct AssetsManager {
    base_dir: PathBuf,
    assets: HashMap<String, AssetResource>
}

impl AssetsManager {

    pub fn new(base_dir: &String) -> Result<AssetsManager> {
        Ok(AssetsManager {
            base_dir: PathBuf::from(base_dir),
            assets: HashMap::new(),
        })
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

    fn new(name: String) -> Result<Node> {
        todo!()
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
