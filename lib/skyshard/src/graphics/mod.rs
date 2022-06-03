pub mod vulkan;

mod camera;

use ash::vk::Buffer;
use nalgebra::Matrix4;
use vk_mem::Allocation;
use crate::engine::Vertex;
pub use crate::graphics::camera::Camera;

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

pub struct Geometry {
    pub position: Position,
    pub indices: Vec<u32>,
    pub index_buffer: Buffer,
    pub index_allocation: Allocation,
    pub vertices: Vec<Vertex>,
    pub vertex_buffer: Buffer,
    pub vertex_allocation: Allocation,
}
