use std::cell::RefCell;
use std::rc::Rc;

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
