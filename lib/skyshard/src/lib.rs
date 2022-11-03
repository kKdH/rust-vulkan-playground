#[cfg(test)] #[macro_use] extern crate hamcrest2;

#[macro_use]
extern crate memoffset;

mod assets;
mod engine;
mod util;

pub mod entity;
pub mod graphics;

use engine::Engine;

pub use engine::create;
pub use engine::create_geometry;
pub use engine::update_geometry;
pub use engine::render;
pub use engine::prepare;
pub use engine::Vertex;
pub use engine::InstanceData;
pub use assets::load_node;
