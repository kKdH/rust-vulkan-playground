#[cfg(test)] #[macro_use] extern crate hamcrest2;

#[macro_use]
extern crate memoffset;

mod engine;
mod util;

pub mod graphics;

use engine::Engine;

pub use engine::create;
pub use engine::render;
