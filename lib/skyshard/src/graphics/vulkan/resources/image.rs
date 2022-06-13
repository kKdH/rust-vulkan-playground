use ash::vk::Handle;
use crate::graphics::Extent;

use crate::graphics::vulkan::resources::{Allocation, Offset, Resource, Size};
use crate::graphics::vulkan::VulkanObject;


pub struct Image {
    name: &'static str,
    extent: Extent,
    image: ::ash::vk::Image,
    allocation: Allocation,
}

impl Image {

    pub fn new(
        name: &'static str,
        extent: Extent,
        image: ::ash::vk::Image,
        allocation: Allocation
    ) -> Image {
        Image {
            name,
            extent,
            image,
            allocation
        }
    }
}

impl Image {

    pub fn extent(&self) -> &Extent {
        &self.extent
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

    fn capacity(&self) -> usize {
        unimplemented!()
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
