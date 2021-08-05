use crate::graphics::vulkan::device::DeviceRef;
use std::rc::Rc;
use crate::graphics::vulkan::queue::QueueFamily;
use ash::vk::{Handle};
use std::cell::{RefCell, Ref};

pub struct CommandBuffer {
    inner: Rc<RefCell<InternalCommandBuffer>>,
}

impl CommandBuffer {

}

pub(super) struct InternalCommandBuffer {
    handle: ash::vk::CommandBuffer
}

impl InternalCommandBuffer {
    pub(super) fn new(handle: ash::vk::CommandBuffer) -> CommandBuffer {
        CommandBuffer {
            inner: Rc::new(RefCell::new(InternalCommandBuffer {
                handle
            }))
        }
    }
}
