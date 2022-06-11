pub mod vulkan;

mod camera;

use nalgebra::Matrix4;
use crate::engine::{InstanceData, Vertex};
pub use crate::graphics::camera::Camera;
use crate::graphics::vulkan::resources::Buffer;

pub struct Renderer {

}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Position {
    pub x: f32,
    pub y: f32,
    pub z: f32
}

impl Position {
    pub fn new(x: f32, y: f32, z: f32) -> Position {
        Position { x, y, z }
    }
}

#[derive(Debug)]
pub struct Extent {
    width: u32,
    height: u32,
    depth: u32,
}

impl Extent {

    pub fn new() -> Extent {
        Extent {
            width: 0,
            height: 0,
            depth: 0
        }
    }

    pub fn from(width: u32, height: u32, depth: u32) -> Extent {
        Extent {
            width,
            height,
            depth
        }
    }
}

pub struct Geometry {
    pub indices: Vec<u32>,
    pub index_buffer: Buffer<u32>,
    pub vertices: Vec<Vertex>,
    pub vertex_buffer: Buffer<Vertex>,
    pub instances: Vec<InstanceData>,
    pub instances_buffer: Buffer<InstanceData>,
}
