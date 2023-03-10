#[cfg(test)] #[macro_use] extern crate hamcrest2;

#[macro_use]
extern crate memoffset;

mod engine;
mod util;

pub mod assets;
pub mod entity;
pub mod graphics;

use engine::Engine;

pub use engine::create;
pub use engine::create_geometry;
pub use engine::update_geometry;
pub use engine::pick_object;
pub use engine::render;
pub use engine::prepare;
pub use engine::Vertex;
pub use engine::InstanceData;
