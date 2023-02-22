pub mod vulkan;

mod camera;

use std::fmt::{Display, Formatter};
use crate::engine::{InstanceData, Vertex};
pub use crate::graphics::camera::Camera;
use crate::graphics::vulkan::resources::{Buffer, Image};

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

#[derive(Debug, Copy, Clone)]
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

impl Default for Extent {
    fn default() -> Self {
        Extent::new()
    }
}

pub struct Geometry {
    pub index_buffer: Buffer<u32>,
    pub vertex_buffer: Buffer<Vertex>,
    pub instances_buffer: Buffer<InstanceData>,
    pub material: Material,
}

pub struct Material {
    pub descriptor_set: ::ash::vk::DescriptorSet,
    pub texture_buffer: Buffer<u8>,
    pub texture_image: Image,
    pub texture_image_view: ::ash::vk::ImageView,
}

#[derive(Copy, Clone, Debug)]
pub enum MSAA {
    Off,
    X2,
    X4,
    X8,
    X16,
    X32,
    X64,
}

impl Display for MSAA {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            MSAA::Off => write!(formatter, "off"),
            MSAA::X2 => write!(formatter, "2x"),
            MSAA::X4 => write!(formatter, "4x"),
            MSAA::X8 => write!(formatter, "8x"),
            MSAA::X16 => write!(formatter, "16x"),
            MSAA::X32 => write!(formatter, "32x"),
            MSAA::X64 => write!(formatter, "64x"),
        }
    }
}
