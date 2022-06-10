use std::any::TypeId;
use std::num::NonZeroU64;
use std::ptr::addr_of_mut;
use std::slice::Iter;
use std::sync::atomic::{AtomicU64, Ordering};
use crate::engine::Vertex;
use crate::graphics::Geometry;

pub struct World {
    pub geometries: Vec<Geometry>,
}

impl World {

    pub fn new() -> Self {
        World {
            geometries: Vec::new()
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct WorldId(u64);

static WORLD_ID_COUNTER: AtomicU64 = AtomicU64::new(0);

impl WorldId {
    fn next() -> Self {
        WorldId(WORLD_ID_COUNTER.fetch_add(1, Ordering::Relaxed))
    }
}

impl Default for WorldId {
    fn default() -> Self {
        Self::next()
    }
}
