extern crate core;

pub mod blend;

#[cfg(feature = "blender2_7")]
include!(concat!(env!("OUT_DIR"), "/blender2_7.rs"));

#[cfg(feature = "blender2_9")]
include!(concat!(env!("OUT_DIR"), "/blender2_9.rs"));

#[cfg(feature = "blender3_0")]
include!(concat!(env!("OUT_DIR"), "/blender3_0.rs"));
