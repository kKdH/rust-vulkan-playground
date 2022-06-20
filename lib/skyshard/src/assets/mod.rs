use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::fs::read_dir;
use log::{debug, info};

use thiserror::Error;

type Result<T, E = AssetsManagerError> = ::std::result::Result<T, E>;

pub struct AssetsManager {
    base_dir: PathBuf,
    nodes: HashMap<String, Node>
}

impl AssetsManager {

    pub fn new(base_dir: &'static str) -> Result<AssetsManager> {
        let base_dir_path = PathBuf::from(base_dir);

        if base_dir_path.exists() {
            let file_path = base_dir_path.join("cube.gltf");
            let file_path_str = file_path.to_str().expect("Failed to resolve path");

            let gltf = gltf::Gltf::open(file_path_str)
                .map_err(|error| AssetsManagerError::OpenFileError { path: file_path_str.to_string() })?;

            let nodes = gltf.scenes()
                .flat_map(|scene| scene.nodes())
                .fold(HashMap::<String, Node>::new(), | mut result, node | {
                    let name = node.name()
                        .expect("Node does not have a name")
                        .to_string();
                    result.insert(Clone::clone(&name), Node {
                        file_path: Box::from(file_path.as_path()),
                        name: name
                    });
                    debug!("Added asset '{}' from file '{}'.", &name, file_path_str);
                    result
                });

            Ok(AssetsManager {
                base_dir: base_dir_path,
                nodes: nodes,
            })
        }
        else {
            Err(AssetsManagerError::BaseDirNotFoundError { path: base_dir })
        }
    }

}

#[derive(Error, Debug)]
pub enum AssetsManagerError {

    #[error("Failed to instantiate AssetsManager!")]
    AssetsManagerInstantiationError,

    #[error("The AssetsManager's base directory '{path}' does not exists!")]
    BaseDirNotFoundError { path: &'static str},

    #[error("Failed to open file '{path}'!")]
    OpenFileError { path: String }
}

pub struct Node {
    file_path: Box<Path>,
    name: String,
}
