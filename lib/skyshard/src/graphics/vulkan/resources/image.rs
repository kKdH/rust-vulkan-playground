use ash::vk::Handle;

use crate::graphics::vulkan::resources::{Allocation, Offset, Resource, Size};
use crate::graphics::vulkan::VulkanObject;


pub struct Image {
    name: &'static str,
    image: ::ash::vk::Image,
    allocation: Allocation,
}

impl Image {

    pub fn new(
        name: &'static str,
        image: ::ash::vk::Image,
        allocation: Allocation
    ) -> Image {

        Image {
            name,
            image,
            allocation
        }
    }
}

impl VulkanObject for Image {

    type A = ::ash::vk::Image;

    fn handle(&self) -> &Self::A {
        &self.image
    }

    fn hex_id(&self) -> String {
        format!("0x{:x?}", self.image.as_raw())
    }
}

impl Resource for Image {

    fn name(&self) -> &'static str {
        self.name
    }

    fn byte_offset(&self, offset: usize) -> Offset {
        unimplemented!()
    }

    fn byte_size(&self, count: usize) -> Size {
        unimplemented!()
    }

    fn allocation(&self) -> &Allocation {
        &self.allocation
    }

    fn take_allocation(&mut self) -> Allocation {
        ::std::mem::take(&mut self.allocation)
    }
}
