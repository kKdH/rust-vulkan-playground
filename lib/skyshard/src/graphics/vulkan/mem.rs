use std::any::Any;
use std::marker::PhantomData;
use std::rc::Rc;
use std::thread::sleep;

use ash::{Device, vk};
use ash::Instance;
use ash::prelude::VkResult;
use ash::vk::Handle;
use ash::vk::PhysicalDevice;
use gpu_allocator::vulkan::Allocation;
use syn::token::printing;
use thiserror::Error;

use crate::graphics::Extent;
use crate::graphics::vulkan::VulkanObject;
use crate::util::HasBuilder;


type Allocator = ::gpu_allocator::vulkan::Allocator;
type Size = ::ash::vk::DeviceSize;
type Offset = ::ash::vk::DeviceSize;
type Result<T, E = MemoryManagerError> = ::std::result::Result<T, E>;

pub struct MemoryManager {
    allocator: Allocator,
    device: Device,
}

impl MemoryManager {

    pub fn new(instance: &Instance, device: &Device, physical_device: &PhysicalDevice) -> MemoryManager {
        MemoryManager {
            allocator: Allocator::new(
                &gpu_allocator::vulkan::AllocatorCreateDesc {
                    instance: Clone::clone(instance),
                    device: Clone::clone(device),
                    physical_device: Clone::clone(physical_device),
                    debug_settings: Default::default(),
                    buffer_device_address: true,
                }).expect("create allocator"),
            device: Clone::clone(device)
        }
    }

    pub fn create_buffer<A>(&mut self, name: &'static str, descriptor: &BufferAllocationDescriptor, capacity: usize) -> Result<Buffer<A>> {

        let element_size = ::std::mem::size_of::<A>();
        let size = (capacity * element_size) as Size;

        let buffer: ::ash::vk::Buffer = {

            let buffer_create_info = MemoryManager::buffer_descriptor_to_ash(descriptor, size)?;

            unsafe { self.device.create_buffer(&buffer_create_info, None) }
                .map_err(|error| MemoryManagerError::CreateBufferError { name })
        }?;

        let requirements = {
            unsafe { self.device.get_buffer_memory_requirements(buffer) }
        };

        let allocation = {
            self.allocator.allocate(&gpu_allocator::vulkan::AllocationCreateDesc {
                name: format!("{}.allocation", name).as_str(),
                requirements: requirements,
                location: MemoryManager::memory_usage_to_ash(descriptor.memory_usage)?,
                linear: true
            }).map_err(|error| MemoryManagerError::AllocateMemoryError)
        }?;

        unsafe {
            self.device.bind_buffer_memory(buffer, allocation.memory(), allocation.offset())
                .map_err(|error| MemoryManagerError::BindBufferError { name })
        }?;

        Ok(Buffer {
            name,
            capacity,
            size,
            element: Element {
                size: element_size,
                phantom: Default::default()
            },
            buffer,
            allocation,
        })
    }

    pub fn create_image(&mut self, name: &'static str, descriptor: &ImageAllocationDescriptor) -> Result<Image> {

        let image: ::ash::vk::Image = {

            let image_create_info = MemoryManager::image_descriptor_to_ash(descriptor)?;

            unsafe { self.device.create_image(&image_create_info, None) }
                .map_err(|error| MemoryManagerError::CreateImageError { name })
        }?;

        let requirements = {
            unsafe { self.device.get_image_memory_requirements(image) }
        };

        let allocation = {
            self.allocator.allocate(&gpu_allocator::vulkan::AllocationCreateDesc {
                name: name,
                requirements: requirements,
                location: MemoryManager::memory_usage_to_ash(descriptor.memory_usage)?,
                linear: true
            }).map_err(|error| MemoryManagerError::AllocateMemoryError)
        }?;

        unsafe {
            self.device.bind_image_memory(image, allocation.memory(), allocation.offset())
                .map_err(|error| MemoryManagerError::BindBufferError { name })
        }?;

        Ok(Image {
            image,
            allocation
        })
    }

    pub fn free<A>(&mut self, resource: A) -> Result<()>
    where A: Resource {
        // let allocation = *resource.allocation();
        // self.allocator.free(*resource.allocation());
        todo!("implement freeing allocations");
    }

    pub unsafe fn copy<T, Src, Dst>(&mut self, source: &Src, destination: &mut Dst, offset: usize, count: usize) -> Result<()>
    where Src: CopySource<T>,
          Dst: CopyDestination<T> {

        let dst: *mut T = destination.ptr().offset(offset as isize);

        ::std::ptr::copy_nonoverlapping(source.ptr(), dst, count);

        Ok(())
    }

    pub unsafe fn flush<A>(&mut self, resource: A, offset: Offset, size: Size) -> Result<()>
    where A: Resource {

        // let ranges = [
        //     ::ash::vk::MappedMemoryRange::builder()
        //         .memory(resource.allocation().memory())
        //         .offset(resource.allocation().offset() + offset)
        //         .size(size)
        //         .build()
        // ];
        //
        // self.device.flush_mapped_memory_ranges(&ranges);

        Ok(())
    }

    fn buffer_descriptor_to_ash(descriptor: &BufferAllocationDescriptor, size: Size) -> Result<ash::vk::BufferCreateInfo> {

        let mut builder = ash::vk::BufferCreateInfo::builder();

        builder = match descriptor.buffer_usage {
            BufferUsage::UniformBuffer => builder.usage(::ash::vk::BufferUsageFlags::UNIFORM_BUFFER),
            BufferUsage::VertexBuffer => builder.usage(::ash::vk::BufferUsageFlags::VERTEX_BUFFER),
            BufferUsage::IndexBuffer => builder.usage(::ash::vk::BufferUsageFlags::INDEX_BUFFER),
            BufferUsage::IndirectBuffer => builder.usage(::ash::vk::BufferUsageFlags::INDIRECT_BUFFER),
        };

        builder = builder.sharing_mode(ash::vk::SharingMode::EXCLUSIVE);
        builder = builder.size(size as ash::vk::DeviceSize);

        Ok(builder.build())
    }

    fn image_descriptor_to_ash(descriptor: &ImageAllocationDescriptor) -> Result<ash::vk::ImageCreateInfo> {

        let mut builder = ash::vk::ImageCreateInfo::builder();

        builder = builder
            .image_type(::ash::vk::ImageType::TYPE_2D)
            .extent((&descriptor.extent).into())
            .mip_levels(1)
            .array_layers(1)
            .format(ash::vk::Format::D32_SFLOAT_S8_UINT)
            .tiling(ash::vk::ImageTiling::OPTIMAL)
            .initial_layout(ash::vk::ImageLayout::UNDEFINED)
            .samples(ash::vk::SampleCountFlags::TYPE_1)
            .sharing_mode(ash::vk::SharingMode::EXCLUSIVE);

        builder = match descriptor.image_usage {
            ImageUsage::DepthStencilAttachment => builder.usage(ash::vk::ImageUsageFlags::DEPTH_STENCIL_ATTACHMENT)
        };

        Ok(builder.build())
    }

    fn memory_usage_to_ash(usage: MemoryLocation) -> Result<gpu_allocator::MemoryLocation> {
        let result = match usage {
            MemoryLocation::CpuToGpu => gpu_allocator::MemoryLocation::CpuToGpu,
            MemoryLocation::GpuToCpu => gpu_allocator::MemoryLocation::GpuToCpu,
            MemoryLocation::GpuOnly => gpu_allocator::MemoryLocation::GpuOnly,
        };
        Ok(result)
    }
}

#[derive(Error, Debug)]
pub enum MemoryManagerError {

    #[error("Failed to create buffer '{name}'!")]
    CreateBufferError { name: &'static str },

    #[error("Failed to bind buffer '{name}'!")]
    BindBufferError { name: &'static str },

    #[error("Failed to create image '{name}'!")]
    CreateImageError { name: &'static str },

    #[error("Failed to bind image '{name}'!")]
    BindImageError { name: &'static str },

    #[error("Failed to allocate memory!")]
    AllocateMemoryError,

    #[error("Somthing went wrong!")]
    UnknownFailure,
}

#[derive(Copy, Clone, Debug)]
pub enum BufferUsage {
    IndexBuffer,
    IndirectBuffer,
    UniformBuffer,
    VertexBuffer,
}

#[derive(Copy, Clone, Debug)]
pub enum ImageUsage {
    DepthStencilAttachment,
}

#[derive(Copy, Clone, Debug)]
pub enum MemoryLocation {
    CpuToGpu,
    GpuToCpu,
    GpuOnly,
}

#[derive(Debug)]
pub struct BufferAllocationDescriptor {
    pub buffer_usage: BufferUsage,
    pub memory_usage: MemoryLocation,
}

#[derive(Debug)]
pub struct ImageAllocationDescriptor {
    pub image_usage: ImageUsage,
    pub extent: Extent,
    pub memory_usage: MemoryLocation,
}

pub trait Resource {
    fn allocation(&self) -> &gpu_allocator::vulkan::Allocation;
}

pub struct Element<A> {
    size: usize,
    phantom: PhantomData<A>
}

pub struct Buffer<A> {
    name: &'static str,
    capacity: usize,
    size: Size,
    element: Element<A>,
    buffer: ::ash::vk::Buffer,
    allocation: ::gpu_allocator::vulkan::Allocation,
}

impl <A> VulkanObject for Buffer<A> {

    type A = ::ash::vk::Buffer;

    fn handle(&self) -> &Self::A {
        &self.buffer
    }

    fn hex_id(&self) -> String {
        format!("0x{:x?}", self.buffer.as_raw())
    }
}

impl <A> Resource for Buffer<A> {
    fn allocation(&self) -> &Allocation {
        &self.allocation
    }
}

impl <A> Resource for &mut Buffer<A> {
    fn allocation(&self) -> &Allocation {
        &self.allocation
    }
}

impl <A> CopyDestination<A> for Buffer<A> {
    fn ptr(&mut self) -> *mut A {
        self.allocation().mapped_ptr()
            .expect("expected host visible memory")
            .as_ptr() as *mut A
    }
}

impl <A> CopySource<A> for Buffer<A> {
    fn ptr(&self) -> *const A {
        self.allocation().mapped_ptr()
            .expect("expected host visible memory")
            .as_ptr() as *const A
    }
}

pub struct Image {
    image: ::ash::vk::Image,
    allocation: ::gpu_allocator::vulkan::Allocation,
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
    fn allocation(&self) -> &Allocation {
        &self.allocation
    }
}

pub trait CopySource<A> {
    fn ptr(&self) -> *const A;
}

pub trait CopyDestination<A> {
    fn ptr(&mut self) -> *mut A;
}

impl <A> CopySource<A> for Vec<A> {
    fn ptr(&self) -> *const A {
        self.as_ptr()
    }
}

impl <A, const N: usize> CopySource<A> for [A; N] {
    fn ptr(&self) -> *const A {
        self.as_ptr()
    }
}

impl <A> CopyDestination<A> for Vec<A> {
    fn ptr(&mut self) -> *mut A {
        self.as_mut_ptr()
    }
}
