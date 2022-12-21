use std::fmt;
use std::rc::{Weak, Rc};
use std::cell::RefCell;
use crate::graphics::vulkan::device::PhysicalDevice;
use std::ops::BitAnd;
use log::{debug, info};
use std::fmt::{Formatter, Debug};

pub type DeviceQueueRef = Rc<DeviceQueue>;

pub struct DeviceQueue {
    handle: ash::vk::Queue,
    index: u32,
    family: QueueFamily,
}

impl fmt::Debug for DeviceQueue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut formatter = f.debug_struct("DeviceQueue");
        formatter.field("index", &self.index);
        formatter.field("family", &self.family);
        formatter.finish()
    }
}

impl DeviceQueue {

    pub fn new(handle: ash::vk::Queue, index: u32, family: QueueFamily) -> DeviceQueue {
        DeviceQueue {
            handle,
            index,
            family,
        }
    }

    pub fn handle(&self) -> &ash::vk::Queue {
        &self.handle
    }

    pub fn index(&self) -> u32 {
        self.index
    }
}

impl Drop for DeviceQueue {
    fn drop(&mut self) {
        info!("Vulkan queue #{} dropped.", self.index)
    }
}

#[derive(Copy, Clone)]
pub struct QueueFamily {
    index: u32,
    capabilities: u32,
    queues: u32,
}

impl QueueFamily {

    pub fn new(
        index: u32,
        capabilities: u32,
        queues: u32
    ) -> QueueFamily {
        QueueFamily {
            index,
            capabilities,
            queues
        }
    }

    pub fn index(&self) -> u32 {
        self.index
    }

    pub fn capabilities(&self) -> QueueCapabilities {
        QueueCapabilities { mask: self.capabilities }
    }

    /// Returns the number of queues of this queue family.
    pub fn queues(&self) -> u32 {
        self.queues
    }
}

pub trait CapabilitiesSupport {
    fn supports(&self, capabilities: QueueCapabilities) -> bool;
}

impl CapabilitiesSupport for QueueFamily {
    fn supports(&self, capabilities: QueueCapabilities) -> bool {
        QueueCapabilities::from(self.capabilities).supports(capabilities)
    }
}

impl CapabilitiesSupport for DeviceQueue {
    fn supports(&self, capabilities: QueueCapabilities) -> bool {
        QueueCapabilities::from(self.family.capabilities).supports(capabilities)
    }
}

impl fmt::Debug for QueueFamily {

    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut formatter = f.debug_struct("QueueFamily");
        formatter.field("index", &self.index);
        formatter.field("queues", &self.queues);
        formatter.field("capabilities", &FmtQueueCapabilities::from(self.capabilities));
        formatter.finish()
    }
}

#[derive(Debug, Copy, Clone)]
pub struct QueueCapabilities {
    pub mask: u32
}

impl QueueCapabilities {

    pub const ANY: Self = QueueCapabilities { mask: 0 };
    pub const GRAPHICS_OPERATIONS: Self = QueueCapabilities { mask: 1 };
    pub const COMPUTE_OPERATIONS: Self = QueueCapabilities { mask: 2 };
    pub const TRANSFER_OPERATIONS: Self = QueueCapabilities { mask: 4 };
    pub const SPARSE_BINDING: Self = QueueCapabilities { mask: 8 };

    pub fn supports(&self, other: QueueCapabilities) -> bool {
        self.mask & other.mask == other.mask
    }
}

impl From<u32> for QueueCapabilities {
    fn from(value: u32) -> Self {
        QueueCapabilities { mask: value }
    }
}

impl std::ops::BitAnd for QueueCapabilities {
    type Output = QueueCapabilities;
    fn bitand(self, rhs: Self) -> Self::Output {
        QueueCapabilities { mask: self.mask | rhs.mask }
    }
}

#[derive(Clone)]
pub struct FmtQueueCapabilities {
    graphics_operations: bool,
    compute_operations: bool,
    transfer_operations: bool,
    sparse_binding: bool
}

impl From<u32> for FmtQueueCapabilities {
    fn from(capabilities: u32) -> Self {
        FmtQueueCapabilities {
            graphics_operations: capabilities & QueueCapabilities::GRAPHICS_OPERATIONS.mask > 0,
            compute_operations: capabilities & QueueCapabilities::COMPUTE_OPERATIONS.mask > 0,
            transfer_operations: capabilities & QueueCapabilities::TRANSFER_OPERATIONS.mask > 0,
            sparse_binding: capabilities & QueueCapabilities::SPARSE_BINDING.mask > 0
        }
    }
}

impl From<QueueCapabilities> for FmtQueueCapabilities {
    fn from(capabilities: QueueCapabilities) -> Self {
        FmtQueueCapabilities::from(capabilities.mask)
    }
}

impl fmt::Debug for FmtQueueCapabilities {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut formatter = f.debug_struct("QueueCapabilities");
        formatter.field("graphics_operations", &self.graphics_operations);
        formatter.field("compute_operations", &self.compute_operations);
        formatter.field("transfer_operations", &self.transfer_operations);
        formatter.field("sparse_binding", &self.sparse_binding);
        formatter.finish()
    }
}

#[cfg(test)]
mod tests {
    // use ash::vk::QueueFlags;
    // use hamcrest2::prelude::*;
    // use crate::graphics::vulkan::queue::{QueueFamily, QueueCapabilities};
    // use crate::graphics::vulkan::queue::CapabilitiesSupport;
    //
    // #[test]
    // fn test_QueueFamily_supports() {
    //
    //     let mut queue = QueueFamily {
    //         index: 0,
    //         capabilities: QueueCapabilities::GRAPHICS_OPERATIONS.mask,
    //         queues: 42
    //     };
    //
    //     assert_that!(queue.supports(QueueFlags::GRAPHICS), is(true));
    //     assert_that!(queue.supports(
    //         QueueFlags::GRAPHICS & QueueFlags::TRANSFER
    //     ), is(false));
    //
    //     queue.capabilities = (QueueCapabilities::GRAPHICS_OPERATIONS & QueueCapabilities::COMPUTE_OPERATIONS).mask;
    //
    //     assert_that!(queue.supports(QueueFlags::GRAPHICS), is(true));
    //     assert_that!(queue.supports(QueueFlags::COMPUTE), is(true));
    //     assert_that!(queue.supports(QueueFlags::TRANSFER), is(false));
    //     assert_that!(queue.supports(
    //         QueueFlags::GRAPHICS & QueueFlags::COMPUTE
    //     ), is(true));
    // }
}
