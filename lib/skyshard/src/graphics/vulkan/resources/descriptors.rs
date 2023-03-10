use std::ops::BitOr;

use crate::graphics::Extent;
use crate::graphics::vulkan::resources::ResourceManagerError;

#[derive(Debug)]
pub struct BufferAllocationDescriptor<const UsagesCount: usize> {
    pub usage: [BufferUsage; UsagesCount],
    pub memory: MemoryLocation,
}

impl <const UsagesCount: usize> TryFrom<&BufferAllocationDescriptor<UsagesCount>> for ::ash::vk::BufferCreateInfoBuilder<'_> {

    type Error = ResourceManagerError;

    fn try_from(descriptor: &BufferAllocationDescriptor<UsagesCount>) -> Result<Self, Self::Error> {
        let builder = ash::vk::BufferCreateInfo::builder()
            .sharing_mode(ash::vk::SharingMode::EXCLUSIVE)
            .usage(descriptor.usage.iter().fold(::ash::vk::BufferUsageFlags::empty(), |result, usage| {
                result.bitor(match usage {
                    BufferUsage::UniformBuffer => ::ash::vk::BufferUsageFlags::UNIFORM_BUFFER,
                    BufferUsage::VertexBuffer => ::ash::vk::BufferUsageFlags::VERTEX_BUFFER,
                    BufferUsage::IndexBuffer => ::ash::vk::BufferUsageFlags::INDEX_BUFFER,
                    BufferUsage::IndirectBuffer => ::ash::vk::BufferUsageFlags::INDIRECT_BUFFER,
                    BufferUsage::TransferSourceBuffer => ::ash::vk::BufferUsageFlags::TRANSFER_SRC,
                    BufferUsage::TransferDestinationBuffer => ::ash::vk::BufferUsageFlags::TRANSFER_DST,
                })
            }));

        Ok(builder)
    }
}

#[derive(Debug)]
pub struct ImageAllocationDescriptor<const N: usize> {
    pub usage: [ImageUsage; N],
    pub extent: Extent,
    pub format: ImageFormat,
    pub tiling: ImageTiling,
    pub memory: MemoryLocation,
}

impl <const N: usize> TryFrom<&ImageAllocationDescriptor<N>> for ::ash::vk::ImageCreateInfoBuilder<'_> {

    type Error = ResourceManagerError;

    fn try_from(descriptor: &ImageAllocationDescriptor<N>) -> Result<Self, Self::Error> {
        let builder = ash::vk::ImageCreateInfo::builder()
            .image_type(::ash::vk::ImageType::TYPE_2D)
            .extent((&descriptor.extent).into())
            .mip_levels(1)
            .array_layers(1)
            .format(descriptor.format.into())
            .tiling(descriptor.tiling.into())
            .initial_layout(ash::vk::ImageLayout::UNDEFINED)
            .samples(ash::vk::SampleCountFlags::TYPE_1)
            .sharing_mode(ash::vk::SharingMode::EXCLUSIVE)
            .usage(descriptor.usage.iter().fold(::ash::vk::ImageUsageFlags::empty(), | mut result, usage| {
                result.bitor(match usage {
                    ImageUsage::DepthStencilAttachment => ::ash::vk::ImageUsageFlags::DEPTH_STENCIL_ATTACHMENT,
                    ImageUsage::Sampled => ::ash::vk::ImageUsageFlags::SAMPLED,
                    ImageUsage::TransferDestination => ::ash::vk::ImageUsageFlags::TRANSFER_DST,
                    ImageUsage::TransferSource => ::ash::vk::ImageUsageFlags::TRANSFER_SRC,
                    ImageUsage::ColorAttachment => ::ash::vk::ImageUsageFlags::COLOR_ATTACHMENT,
                })
            }));
        Ok(builder)
    }
}

#[derive(Copy, Clone, Debug)]
pub enum BufferUsage {
    IndexBuffer,
    IndirectBuffer,
    UniformBuffer,
    VertexBuffer,
    TransferSourceBuffer,
    TransferDestinationBuffer,
}

#[derive(Copy, Clone, Debug)]
pub enum ImageUsage {
    DepthStencilAttachment,
    Sampled,
    TransferDestination,
    TransferSource,
    ColorAttachment,
}

#[derive(Copy, Clone, Debug)]
pub enum MemoryLocation {
    CpuToGpu,
    GpuToCpu,
    GpuOnly,
}

impl From<MemoryLocation> for gpu_allocator::MemoryLocation {
    fn from(memory_location: MemoryLocation) -> Self {
        match memory_location {
            MemoryLocation::CpuToGpu => gpu_allocator::MemoryLocation::CpuToGpu,
            MemoryLocation::GpuToCpu => gpu_allocator::MemoryLocation::GpuToCpu,
            MemoryLocation::GpuOnly => gpu_allocator::MemoryLocation::GpuOnly,
        }
    }
}

#[derive(Debug, Copy, Clone)]
pub enum ImageFormat {
    RGBA8,
    UInt32,
    DepthStencil,
}

impl From<ImageFormat> for ::ash::vk::Format {
    fn from(format: ImageFormat) -> Self {
        match format {
            ImageFormat::RGBA8 => ::ash::vk::Format::R8G8B8A8_UNORM,
            ImageFormat::UInt32 => ::ash::vk::Format::R32_UINT,
            ImageFormat::DepthStencil => ::ash::vk::Format::D32_SFLOAT_S8_UINT,
        }
    }
}

#[derive(Debug, Copy, Clone)]
pub enum ImageTiling {
    Linear,
    Optimal,
}

impl From<ImageTiling> for ::ash::vk::ImageTiling {
    fn from(format: ImageTiling) -> Self {
        match format {
            ImageTiling::Linear => ::ash::vk::ImageTiling::LINEAR,
            ImageTiling::Optimal => ::ash::vk::ImageTiling::OPTIMAL,
        }
    }
}

