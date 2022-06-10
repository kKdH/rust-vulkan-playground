use crate::graphics::Extent;

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
