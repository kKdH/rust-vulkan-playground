extern crate core;

pub mod blend;

#[cfg(feature = "blender2_7")]
pub use blend::blender2_7;

#[cfg(feature = "blender2_9")]
pub use blend::blender2_9;

#[cfg(feature = "blender3_0")]
pub use blend::blender3_0;
