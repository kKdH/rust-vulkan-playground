mod buffer;
mod copy;
mod descriptors;
mod image;

pub use buffer::{Buffer, Element};
pub use copy::{CopyDestination, CopySource};
pub use descriptors::{BufferAllocationDescriptor, ImageAllocationDescriptor, BufferUsage, ImageUsage, ImageFormat, MemoryLocation};
pub use image::{Image};

use log::info;

use thiserror::Error;

type Device = ::ash::Device;
type Instance = ::ash::Instance;
type PhysicalDevice = ::ash::vk::PhysicalDevice;
type Allocator = ::gpu_allocator::vulkan::Allocator;

type Size = ::ash::vk::DeviceSize;
type Offset = ::ash::vk::DeviceSize;
type Result<T, E = ResourceManagerError> = ::std::result::Result<T, E>;
type Allocation = ::gpu_allocator::vulkan::Allocation;

pub struct ResourceManager {
    device: Device,
    allocator: Allocator,
}

impl ResourceManager {

    pub fn new(instance: &Instance, device: &Device, physical_device: &PhysicalDevice) -> Result<ResourceManager> {
        let result = ResourceManager {
            device: Clone::clone(device),
            allocator: Allocator::new(
                &gpu_allocator::vulkan::AllocatorCreateDesc {
                    instance: Clone::clone(instance),
                    device: Clone::clone(device),
                    physical_device: Clone::clone(physical_device),
                    debug_settings: Default::default(),
                    buffer_device_address: true,
                }).expect("create allocator"),
        };
        info!("ResourceManager created.");
        Ok(result)
    }

    pub fn create_buffer<A, const UsagesCount: usize>(&mut self, name: &'static str, descriptor: &BufferAllocationDescriptor<UsagesCount>, capacity: usize) -> Result<Buffer<A>> {

        let element_size = ::std::mem::size_of::<A>();
        let size = (capacity * element_size) as Size;

        let buffer: ::ash::vk::Buffer = {

            let buffer_create_info  = descriptor
                .try_into()
                .map(|builder: ::ash::vk::BufferCreateInfoBuilder| {
                    builder.size(size as ash::vk::DeviceSize)
                           .build()
                })?;

            unsafe { self.device.create_buffer(&buffer_create_info, None) }
                .map_err(|error| ResourceManagerError::CreateBufferError { name })
        }?;

        let requirements = {
            unsafe { self.device.get_buffer_memory_requirements(buffer) }
        };

        let allocation = {
            self.allocator.allocate(&gpu_allocator::vulkan::AllocationCreateDesc {
                name: format!("{}.allocation", name).as_str(),
                requirements: requirements,
                location: descriptor.memory.into(),
                linear: true
            }).map_err(|error| ResourceManagerError::AllocateMemoryError { cause: error })
        }?;

        unsafe {
            self.device.bind_buffer_memory(buffer, allocation.memory(), allocation.offset())
                .map_err(|error| ResourceManagerError::BindBufferError { name })
        }?;

        info!("Created buffer '{}' as {:?} with a capacity for {} elements respectifely {} bytes ({} bytes pre element).", name, descriptor.usage, capacity, size, element_size);

        Ok(Buffer::new(name, capacity, element_size, buffer, allocation))
    }

    pub fn create_image<const N: usize>(&mut self, name: &'static str, descriptor: &ImageAllocationDescriptor<N>) -> Result<Image> {

        let image: ::ash::vk::Image = {

            let image_create_info = descriptor
                .try_into()
                .map(|builder: ::ash::vk::ImageCreateInfoBuilder| {
                    builder.build()
                })?;

            unsafe { self.device.create_image(&image_create_info, None) }
                .map_err(|error| ResourceManagerError::CreateImageError { name })
        }?;

        let requirements = {
            unsafe { self.device.get_image_memory_requirements(image) }
        };

        let allocation = {
            self.allocator.allocate(&gpu_allocator::vulkan::AllocationCreateDesc {
                name: name,
                requirements: requirements,
                location: descriptor.memory.into(),
                linear: true
            }).map_err(|error| ResourceManagerError::AllocateMemoryError { cause: error })
        }?;

        unsafe {
            self.device.bind_image_memory(image, allocation.memory(), allocation.offset())
                .map_err(|error| ResourceManagerError::BindBufferError { name })
        }?;

        info!("Created image '{}' as {:?} with an extent of {}x{}x{}.", name, descriptor.usage, descriptor.extent.width, descriptor.extent.height, descriptor.extent.depth);

        Ok(Image::new(
            name,
            descriptor.extent,
            image,
            allocation
        ))
    }

    pub unsafe fn copy<T, Src, Dst>(&mut self, source: &Src, destination: &mut Dst, offset: usize, count: usize) -> Result<()>
        where Src: CopySource<T>,
              Dst: CopyDestination<T> {

        let dst: *mut T = destination.ptr().offset(offset as isize);

        ::std::ptr::copy_nonoverlapping(source.ptr(), dst, count);

        Ok(())
    }

    pub unsafe fn flush<A>(&mut self, resource: A, offset: usize, count: usize) -> Result<()>
        where A: Resource {

        let ranges = [
            ::ash::vk::MappedMemoryRange::builder()
                .memory(resource.allocation().memory())
                .offset(resource.allocation().offset() + resource.byte_offset(offset))
                .size(resource.byte_size(count))
                .build()
        ];

        self.device.flush_mapped_memory_ranges(&ranges)
            .map_err(|error| ResourceManagerError::FlushMemoryError { name: resource.name() } )
    }

    pub fn free<A>(&mut self, mut resource: A) -> Result<()>
        where A: Resource {

        let allocation = resource.take_allocation();
        self.allocator.free(allocation)
            .map_err(|error| ResourceManagerError::FreeMemoryError { name: resource.name() })
    }
}

pub trait Resource {
    fn name(&self) -> &'static str;
    fn capacity(&self) -> usize;
    fn byte_offset(&self, offset: usize) -> Offset;
    fn byte_size(&self, count: usize) -> Size;
    fn allocation(&self) -> &gpu_allocator::vulkan::Allocation;
    fn take_allocation(&mut self) -> gpu_allocator::vulkan::Allocation;
}

#[derive(Error, Debug)]
pub enum ResourceManagerError {

    #[error("Invalid allocation descriptor for buffer '{name}'!")]
    InvalidBufferAllocationDescriptorError {
        name: &'static str
    },

    #[error("Failed to create buffer '{name}'!")]
    CreateBufferError {
        name: &'static str
    },

    #[error("Failed to bind buffer '{name}'!")]
    BindBufferError {
        name: &'static str
    },

    #[error("Invalid allocation descriptor for image '{name}'!")]
    InvalidImageAllocationDescriptorError {
        name: &'static str
    },

    #[error("Failed to create image '{name}'!")]
    CreateImageError {
        name: &'static str
    },

    #[error("Failed to bind image '{name}'!")]
    BindImageError {
        name: &'static str
    },

    #[error("Failed to allocate memory!")]
    AllocateMemoryError {
        #[source]
        cause: ::gpu_allocator::AllocationError
    },

    #[error("Failed to flush memory of '{name}'!")]
    FlushMemoryError {
        name: &'static str
    },

    #[error("Failed to free memory of '{name}'!")]
    FreeMemoryError {
        name: &'static str
    },
}
