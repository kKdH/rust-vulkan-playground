
mod buffer;
mod image;
mod copy;

pub mod descriptors;

pub use buffer::{Buffer, Element};
pub use image::{Image};
pub use copy::{CopyDestination, CopySource};

use descriptors::{BufferUsage, MemoryLocation, ImageUsage, BufferAllocationDescriptor, ImageAllocationDescriptor};

use thiserror::Error;

type Device = ::ash::Device;
type Instance = ::ash::Instance;
type PhysicalDevice = ::ash::vk::PhysicalDevice;
type Allocator = ::gpu_allocator::vulkan::Allocator;

type Size = ::ash::vk::DeviceSize;
type Offset = ::ash::vk::DeviceSize;
type Result<T, E = MemoryManagerError> = ::std::result::Result<T, E>;
type Allocation = ::gpu_allocator::vulkan::Allocation;

pub struct ResourceManager {
    device: Device,
    allocator: Allocator,
}

impl ResourceManager {

    pub fn new(instance: &Instance, device: &Device, physical_device: &PhysicalDevice) -> ResourceManager {
        ResourceManager {
            device: Clone::clone(device),
            allocator: Allocator::new(
                &gpu_allocator::vulkan::AllocatorCreateDesc {
                    instance: Clone::clone(instance),
                    device: Clone::clone(device),
                    physical_device: Clone::clone(physical_device),
                    debug_settings: Default::default(),
                    buffer_device_address: true,
                }).expect("create allocator"),
        }
    }

    pub fn create_buffer<A>(&mut self, name: &'static str, descriptor: &BufferAllocationDescriptor, capacity: usize) -> Result<Buffer<A>> {

        let element_size = ::std::mem::size_of::<A>();
        let size = (capacity * element_size) as Size;

        let buffer: ::ash::vk::Buffer = {

            let buffer_create_info = ResourceManager::buffer_descriptor_to_ash(descriptor, size)?;

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
                location: ResourceManager::memory_usage_to_ash(descriptor.memory_usage)?,
                linear: true
            }).map_err(|error| MemoryManagerError::AllocateMemoryError { cause: error })
        }?;

        unsafe {
            self.device.bind_buffer_memory(buffer, allocation.memory(), allocation.offset())
                .map_err(|error| MemoryManagerError::BindBufferError { name })
        }?;

        Ok(Buffer::new(name, capacity, element_size, buffer, allocation))
    }

    pub fn create_image(&mut self, name: &'static str, descriptor: &ImageAllocationDescriptor) -> Result<Image> {

        let image: ::ash::vk::Image = {

            let image_create_info = ResourceManager::image_descriptor_to_ash(descriptor)?;

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
                location: ResourceManager::memory_usage_to_ash(descriptor.memory_usage)?,
                linear: true
            }).map_err(|error| MemoryManagerError::AllocateMemoryError { cause: error })
        }?;

        unsafe {
            self.device.bind_image_memory(image, allocation.memory(), allocation.offset())
                .map_err(|error| MemoryManagerError::BindBufferError { name })
        }?;

        Ok(Image::new(name, image, allocation))
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
            .map_err(|error| MemoryManagerError::FlushMemoryError { name: resource.name() } )
    }

    pub fn free<A>(&mut self, mut resource: A) -> Result<()>
        where A: Resource {

        let allocation = resource.take_allocation();
        self.allocator.free(allocation)
            .map_err(|error| MemoryManagerError::FreeMemoryError { name: resource.name() })
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

pub trait Resource {
    fn name(&self) -> &'static str;
    fn byte_offset(&self, offset: usize) -> Offset;
    fn byte_size(&self, count: usize) -> Size;
    fn allocation(&self) -> &gpu_allocator::vulkan::Allocation;
    fn take_allocation(&mut self) -> gpu_allocator::vulkan::Allocation;
}

#[derive(Error, Debug)]
pub enum MemoryManagerError {

    #[error("Failed to create buffer '{name}'!")]
    CreateBufferError {
        name: &'static str
    },

    #[error("Failed to bind buffer '{name}'!")]
    BindBufferError {
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

    #[error("Somthing went wrong!")]
    UnknownFailure,
}
