//! [![github]](https://example.com)&ensp;[![crates-io]](https://crates.io/crates/blend-rs)&ensp;[![docs-rs]](https://docs.rs/blend-rs)
//!
//! [github]: https://img.shields.io/badge/github-8da0cb?style=for-the-badge&labelColor=555555&logo=github
//! [crates-io]: https://img.shields.io/badge/crates.io-fc8d62?style=for-the-badge&labelColor=555555&logo=rust
//! [docs-rs]: https://img.shields.io/badge/docs.rs-66c2a5?style=for-the-badge&labelColor=555555&logo=docs.rs
//!
//! # blender-rs
//!
//! A Rust library to read Blender's .blend files without the hassle of byte parsing.
//!
//! **Note:** Due to the functioning of this library not all blender version are supported.
//! The table about [Crate Features](#crate-features) lists the supported blender version.
//!
//! ## Example
//! The example below illustrates how to extract the coordinates of each vertex of an object's mesh.
//! ```rust
//! use blend_rs::blend::{read, StructIter, PointerLike, NameLike};
//! use blend_rs::blend::traverse::Named;
//!
//! use blend_rs::blender3_2::{Object, Mesh, MPoly, MVert, MLoop};
//!
//! pub struct Vertex {
//!     pub position: [f32; 3],
//! }
//!
//! fn main() {
//!
//!    let blend_data = std::fs::read("examples/example-3.2.blend")
//!        .expect("Blend file not found!");
//!
//!    let reader = read(&blend_data)
//!        .expect("Failed to read blend data!");
//!
//!    let plane: &Object = reader.iter::<Object>().unwrap()
//!        .find(|object| object.id.get_name() == "Plane")
//!        .unwrap();
//!
//!    let mesh = reader.deref_single(&plane.data.as_instance_of::<Mesh>())
//!        .expect("Could not get mesh from object!");
//!
//!    let mesh_polygons: StructIter<MPoly> = reader.deref(&mesh.mpoly)
//!        .expect("Could not get polygons from mesh!");
//!
//!    let mesh_loops: Vec<&MLoop> = reader.deref(&mesh.mloop)
//!        .expect("Could not get loops from mesh!")
//!        .collect();
//!
//!    let mesh_vertices: Vec<&MVert> = reader.deref(&mesh.mvert)
//!        .expect("Could not get vertices from mesh!")
//!        .collect();
//!
//!    let vertices_per_polygon: Vec<Vec<Vertex>> = mesh_polygons
//!        .map(|polygon| {
//!            (polygon.loopstart..polygon.loopstart + polygon.totloop).into_iter()
//!                .map(|loop_index| {
//!                    Vertex {
//!                        position: mesh_vertices[mesh_loops[loop_index as usize].v as usize].co
//!                    }
//!                })
//!                .collect()
//!        })
//!        .collect();
//! }
//! ```
//!
//! ## Crate Features
//! Enable or disable features according to your needs and in order to optimize compile time.
//!
//! | Feature           | Default  | Description                                                        |
//! | ----------------- |:--------:| ------------------------------------------------------------------ |
//! | blender2_79       | &#x2717; | Generate and include code for blender 2.79 (64 Bit, little endian) |
//! | blender2_80       | &#x2717; | Generate and include code for blender 2.80 (64 Bit, little endian) |
//! | blender2_80x86    | &#x2717; | Generate and include code for blender 2.80 (32 Bit, little endian) |
//! | blender2_93       | &#x2717; | Generate and include code for blender 2.93 (64 Bit, little endian) |
//! | blender3_2        | &#x2714; | Generate and include code for blender 3.2 (64 Bit, little endian)  |
//! | blender3_3        | &#x2717; | Generate and include code for blender 3.3 (64 Bit, little endian)  |
//! | all               | &#x2717; | Generate and include code for all above blender versions.          |
//!
//! <sup>&#x2714; enabled, &#x2717; disabled</sup>
//!
//! ## Details
//!
//! blend-rs depends heavily on code generated from the Blender DNA. The Blender DNA is a part of
//! each `*.blend` file and contains a description for all structs, types and names within the file.
//! Blender uses the DNA for forward- and backward compatibility. blend-rs uses the DNA to generate
//! rust code.
//!
//! This library belongs to a set of three libraries which are all related to the topic of reading
//! Blender's .blend files:
//!
//! <p style="text-align: center;">
        #![doc=include_str!("../overview.svg")]
//! </p>
//!
//! * [blend-inspect-rs](https://docs.rs/blend-inspect-rs):
//! A Rust library to parse and analyse Blender's .blend files.
//! * [blend-bindgen-rs](https://docs.rs/blend-bindgen-rs):
//! A Rust library to generated Blender's data structures.
//! * [blend-rs](https://docs.rs/blend-rs):
//! A Rust library to read Blender's .blend files.
//!
//!
extern crate core;

pub mod blend;

#[cfg(feature = "blender3_3")]
include!(concat!(env!("OUT_DIR"), "/blender3_3.rs"));

#[cfg(feature = "blender3_2")]
include!(concat!(env!("OUT_DIR"), "/blender3_2.rs"));

#[cfg(feature = "blender2_93")]
include!(concat!(env!("OUT_DIR"), "/blender2_93.rs"));

#[cfg(feature = "blender2_80")]
include!(concat!(env!("OUT_DIR"), "/blender2_80.rs"));

#[cfg(feature = "blender2_80x86")]
include!(concat!(env!("OUT_DIR"), "/blender2_80x86.rs"));

#[cfg(feature = "blender2_79")]
include!(concat!(env!("OUT_DIR"), "/blender2_79.rs"));
