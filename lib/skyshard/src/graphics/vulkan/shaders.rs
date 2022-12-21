use std::ffi::CString;
use std::io::Cursor;

use thiserror::Error;

use crate::graphics::vulkan::device::Device;

pub type Code = Vec<u32>;
pub type CodeSlice<'a> = &'a[u32];

pub trait ShaderBinary {
    fn stage(&self) -> ::ash::vk::ShaderStageFlags;
    fn code(&self) -> &[u32];
}

pub struct VertexShaderBinary {
    code: Code,
}

impl VertexShaderBinary {
    pub fn new(binary: &[u8]) -> VertexShaderBinary {
        VertexShaderBinary {
            code: ::ash::util::read_spv(&mut Cursor::new(binary)).unwrap()
        }
    }
}

impl ShaderBinary for VertexShaderBinary {

    fn stage(&self) -> ash::vk::ShaderStageFlags {
        ash::vk::ShaderStageFlags::VERTEX
    }

    fn code(&self) -> &[u32] {
        self.code.as_slice()
    }
}

pub struct FragmentShaderBinary {
    code: Code,
}

impl FragmentShaderBinary {
    pub fn new(binary: &[u8]) -> FragmentShaderBinary {
        FragmentShaderBinary {
            code: ::ash::util::read_spv(&mut Cursor::new(binary)).unwrap()
        }
    }
}

impl ShaderBinary for FragmentShaderBinary {

    fn stage(&self) -> ash::vk::ShaderStageFlags {
        ash::vk::ShaderStageFlags::FRAGMENT
    }

    fn code(&self) -> &[u32] {
        self.code.as_slice()
    }
}

pub struct GeometryShaderBinary {
    code: Code,
}

impl GeometryShaderBinary {
    pub fn new(binary: &[u8]) -> GeometryShaderBinary {
        GeometryShaderBinary {
            code: ::ash::util::read_spv(&mut Cursor::new(binary)).unwrap()
        }
    }
}

impl ShaderBinary for GeometryShaderBinary {

    fn stage(&self) -> ash::vk::ShaderStageFlags {
        ash::vk::ShaderStageFlags::GEOMETRY
    }

    fn code(&self) -> &[u32] {
        self.code.as_slice()
    }
}

#[derive(Error, Debug, PartialEq)]
pub enum ShaderModuleError {

    #[error("The entrypoint '{entrypoint}' is invalid!")]
    InvalidEntryPoint { entrypoint: String },

    #[error("Failed to load shader module: {cause}")]
    LoadFailure { cause: String },

    #[error("Failed to unload shader module: {cause}")]
    UnloadFailure { cause: String },
}

pub struct ShaderModule<A>
where A: ShaderBinary {
    shader: A,
    entrypoint: CString,
}

impl <A> ShaderModule<A>
where A: ShaderBinary {

    pub fn create(shader: A, entrypoint: &str) -> Result<Self, ShaderModuleError> {
        let entrypoint = CString::new(entrypoint)
            .map_err(|_| ShaderModuleError::InvalidEntryPoint { entrypoint: String::from(entrypoint) })?;
        Ok(ShaderModule {
            shader,
            entrypoint,
        })
    }

    pub fn load(self, device: &Device) -> Result<LoadedShaderModule<A>, ShaderModuleError> {

        let module_create_info = ::ash::vk::ShaderModuleCreateInfo::builder()
            .code(self.shader.code())
            .build();

        let handle = unsafe {
            device.handle().create_shader_module(&module_create_info, None)
        }.map_err(|result| ShaderModuleError::LoadFailure { cause: format!("{}", result)})?;

        Ok(LoadedShaderModule {
            inner: self,
            handle,
        })
    }
}

pub struct LoadedShaderModule<A>
where A: ShaderBinary {
    inner: ShaderModule<A>,
    handle: ::ash::vk::ShaderModule,
}

impl <A> LoadedShaderModule<A>
where A: ShaderBinary {

    pub fn unload(self, device: &Device) -> Result<ShaderModule<A>, ShaderModuleError> {

        unsafe {
            device.handle().destroy_shader_module(self.handle, None)
        }

        Ok(ShaderModule {
            shader: self.inner.shader,
            entrypoint: self.inner.entrypoint,
        })
    }

    pub fn create_pipeline_shader_stage_create_info(&self) -> ash::vk::PipelineShaderStageCreateInfo {
        ash::vk::PipelineShaderStageCreateInfo::builder()
            .stage(self.inner.shader.stage())
            .module(self.handle)
            .name(self.inner.entrypoint.as_c_str())
            .build()
    }
}
