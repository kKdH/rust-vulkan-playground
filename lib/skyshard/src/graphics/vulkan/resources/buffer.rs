use std::marker::PhantomData;
use ash::vk::Handle;

use crate::graphics::vulkan::resources::{Allocation, Offset, Resource, Size};
use crate::graphics::vulkan::VulkanObject;


pub struct Buffer<A> {
    id: String,
    name: String,
    capacity: usize,
    size: Size,
    element: Element<A>,
    buffer: ::ash::vk::Buffer,
    allocation: ::gpu_allocator::vulkan::Allocation,
}

impl <A> Buffer<A> {

    pub fn new(
        name: String,
        capacity: usize,
        size: usize,
        buffer: ::ash::vk::Buffer,
        allocation: ::gpu_allocator::vulkan::Allocation
    ) -> Buffer<A> {

        Buffer {
            id: String::from(format!("0x{:x?}", buffer.as_raw())),
            name: name,
            capacity: capacity,
            size: (capacity * size) as Size,
            element: Element {
                size,
                phantom: PhantomData::default()
            },
            buffer: buffer,
            allocation: allocation
        }
    }
}

pub struct Element<A> {
    size: usize,
    phantom: PhantomData<A>
}

impl <A> VulkanObject for Buffer<A> {

    type A = ::ash::vk::Buffer;

    fn handle(&self) -> &Self::A {
        &self.buffer
    }

    fn hex_id(&self) -> String {
        String::from(self.id())
    }
}

impl <A> Resource for Buffer<A> {

    fn id(&self) -> &str {
        &self.id
    }

    fn name(&self) -> &str {
        &self.name
    }

    fn capacity(&self) -> usize {
        self.capacity
    }

    fn byte_offset(&self, offset: usize) -> Offset {
        (offset * self.element.size) as Offset
    }

    fn byte_size(&self, count: usize) -> Size {
        (count * self.element.size) as Size
    }

    fn allocation(&self) -> &Allocation {
        &self.allocation
    }

    fn take_allocation(&mut self) -> Allocation {
        ::std::mem::take(&mut self.allocation)
    }
}

impl <A> Resource for &mut Buffer<A> {

    fn id(&self) -> &str {
        &self.id
    }

    fn name(&self) -> &str {
        &self.name
    }

    fn capacity(&self) -> usize {
        self.capacity
    }

    fn byte_offset(&self, offset: usize) -> Offset {
        (offset * self.element.size) as Offset
    }

    fn byte_size(&self, count: usize) -> Size {
        (count * self.element.size) as Size
    }

    fn allocation(&self) -> &Allocation {
        &self.allocation
    }

    fn take_allocation(&mut self) -> Allocation {
        ::std::mem::take(&mut self.allocation)
    }
}
