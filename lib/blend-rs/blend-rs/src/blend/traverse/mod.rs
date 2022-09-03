//!
//! # Traverse
//!
//! The Traverse module contains utilities for traversing some structures of a blend file.
//!
mod double_linked;
mod named;

pub use crate::blend::traverse::double_linked::{DoubleLinked, DoubleLinkedIter};
pub use crate::blend::traverse::named::{Named};
