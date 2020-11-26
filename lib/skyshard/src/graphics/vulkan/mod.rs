use std::borrow::Cow;
use std::ffi::CStr;
use std::fmt;

use ash::vk::{Handle, Result};
use log::{debug, error, info, log, warn};
use thiserror::Error;

mod util;
pub mod instance;
pub mod device;
pub mod surface;
pub mod queue;
pub mod swapchain;
pub mod buffer;
pub mod renderpass;
pub mod mem;

trait VulkanObject {

    /// Returns this vulkan object's handle as hex-encoded string.
    /// Example: `0x5654cd8dfce0`
    fn hex_id(&self) -> String;
}

#[derive(Error, Debug)]
pub enum VulkanError {

    #[error("A host memory allocation has failed")]
    OutOfHostMemoryError,

    #[error("A device memory allocation has failed")]
    OutOfDeviceMemoryError,

    #[error("Initialization of a object has failed")]
    InitializationFailedError,

    #[error("The logical device has been lost. See <<devsandqueues-lost-device>>")]
    DeviceLostError,

    #[error("Mapping of a memory object has failed")]
    MemoryMapFailedError,

    #[error("Layer specified does not exist")]
    LayerNotPresentError,

    #[error("Extension specified does not exist")]
    ExtensionNotPresentError,

    #[error("Requested feature is not available on this device")]
    FeatureNotPresentError,

    #[error("Unable to find a Vulkan driver")]
    IncompatibleDriverError,

    #[error("Too many objects of the type have already been created")]
    TooManyObjectsError,

    #[error("Requested format is not supported on this device")]
    FormatNotSupportedError,

    #[error("A requested pool allocation has failed due to fragmentation of the pool\'s memory")]
    FragmentedPoolError,

    #[error("An unknown error has occurred.")]
    Unknown
}

impl From<ash::vk::Result> for VulkanError {

    fn from(result: ash::vk::Result) -> Self {
        match result {
            ash::vk::Result::ERROR_OUT_OF_HOST_MEMORY => VulkanError::OutOfHostMemoryError,
            ash::vk::Result::ERROR_OUT_OF_DEVICE_MEMORY => VulkanError::OutOfDeviceMemoryError,
            ash::vk::Result::ERROR_INITIALIZATION_FAILED => VulkanError::InitializationFailedError,
            ash::vk::Result::ERROR_DEVICE_LOST => VulkanError::DeviceLostError,
            ash::vk::Result::ERROR_MEMORY_MAP_FAILED => VulkanError::MemoryMapFailedError,
            ash::vk::Result::ERROR_LAYER_NOT_PRESENT => VulkanError::LayerNotPresentError,
            ash::vk::Result::ERROR_EXTENSION_NOT_PRESENT => VulkanError::ExtensionNotPresentError,
            ash::vk::Result::ERROR_FEATURE_NOT_PRESENT => VulkanError::FeatureNotPresentError,
            ash::vk::Result::ERROR_INCOMPATIBLE_DRIVER => VulkanError::IncompatibleDriverError,
            ash::vk::Result::ERROR_TOO_MANY_OBJECTS => VulkanError::TooManyObjectsError,
            ash::vk::Result::ERROR_FORMAT_NOT_SUPPORTED => VulkanError::FormatNotSupportedError,
            ash::vk::Result::ERROR_FRAGMENTED_POOL => VulkanError::FragmentedPoolError,
            ash::vk::Result::ERROR_UNKNOWN => VulkanError::Unknown,
            _ => VulkanError::Unknown
        }
    }
}


#[derive(PartialEq, Eq)]
pub struct DebugLevel {
    mask: u32,
    name: &'static str
}

impl DebugLevel {

    const DEBUG_MASK: u32 = 1 ;
    const WARNING_MASK: u32 = 4;
    const INFO_MASK: u32 = 2;
    const ERROR_MASK: u32 = 8;

    pub const ERROR: Self = Self {
        mask: Self::ERROR_MASK,
        name: "ERROR"
    };

    pub const WARNING: Self = Self {
        mask: Self::ERROR_MASK | Self::WARNING_MASK,
        name: "WARNING"
    };

    pub const INFO: Self = Self {
        mask: Self::ERROR_MASK | Self::WARNING_MASK | Self::INFO_MASK,
        name: "INFO"
    };

    pub const DEBUG: Self = Self {
        mask: Self::ERROR_MASK | Self::WARNING_MASK | Self::INFO_MASK | Self::DEBUG_MASK,
        name: "DEBUG"
    };

    pub fn as_flags(&self) -> ash::vk::DebugUtilsMessageSeverityFlagsEXT {

        if self.mask & Self::DEBUG_MASK == Self::DEBUG_MASK {
            return ash::vk::DebugUtilsMessageSeverityFlagsEXT::ERROR
                | ash::vk::DebugUtilsMessageSeverityFlagsEXT::WARNING
                | ash::vk::DebugUtilsMessageSeverityFlagsEXT::INFO
                | ash::vk::DebugUtilsMessageSeverityFlagsEXT::VERBOSE;
        }

        if self.mask & Self::INFO_MASK == Self::INFO_MASK {
            return ash::vk::DebugUtilsMessageSeverityFlagsEXT::ERROR
                | ash::vk::DebugUtilsMessageSeverityFlagsEXT::WARNING
                | ash::vk::DebugUtilsMessageSeverityFlagsEXT::INFO;
        }

        if self.mask & Self::WARNING_MASK == Self::WARNING_MASK {
            return ash::vk::DebugUtilsMessageSeverityFlagsEXT::ERROR
                | ash::vk::DebugUtilsMessageSeverityFlagsEXT::WARNING;
        }

        if self.mask & Self::ERROR_MASK == Self::ERROR_MASK {
            return ash::vk::DebugUtilsMessageSeverityFlagsEXT::ERROR;
        }

        panic!("this should not be happen")
    }
}

impl From<ash::vk::DebugUtilsMessageSeverityFlagsEXT> for DebugLevel {
    fn from(flags: ash::vk::DebugUtilsMessageSeverityFlagsEXT) -> Self {
        match flags {
            ash::vk::DebugUtilsMessageSeverityFlagsEXT::ERROR => DebugLevel::ERROR,
            ash::vk::DebugUtilsMessageSeverityFlagsEXT::WARNING => DebugLevel::WARNING,
            ash::vk::DebugUtilsMessageSeverityFlagsEXT::INFO => DebugLevel::INFO,
            _ => DebugLevel::DEBUG,
        }
    }
}

impl fmt::Debug for DebugLevel {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.name)
    }
}

impl fmt::Display for DebugLevel {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.name)
    }
}

pub struct DebugUtil {
    loader: ash::extensions::ext::DebugUtils,
    callback: ash::vk::DebugUtilsMessengerEXT
}

impl DebugUtil {

    pub fn new(loader: ash::extensions::ext::DebugUtils, level: DebugLevel) -> DebugUtil {

        let debug_info = ash::vk::DebugUtilsMessengerCreateInfoEXT::builder()
            .message_severity(level.as_flags())
            .message_type(ash::vk::DebugUtilsMessageTypeFlagsEXT::all())
            .pfn_user_callback(Some(DebugUtil::vulkan_debug_callback));

        let callback = unsafe {
            loader.create_debug_utils_messenger(&debug_info, None)
        }.unwrap();

        let debug_util = DebugUtil {
            loader,
            callback
        };

        info!("Vulkan debug messanger <{}> created.", debug_util.hex_id());

        debug_util
    }

    unsafe extern "system" fn vulkan_debug_callback(
        message_severity: ash::vk::DebugUtilsMessageSeverityFlagsEXT,
        message_type: ash::vk::DebugUtilsMessageTypeFlagsEXT,
        p_callback_data: *const ash::vk::DebugUtilsMessengerCallbackDataEXT,
        _user_data: *mut std::os::raw::c_void,
    ) -> ash::vk::Bool32 {

        let callback_data = *p_callback_data;
        let message_id_number: i32 = callback_data.message_id_number as i32;

        let message_id_name = if callback_data.p_message_id_name.is_null() {
            Cow::from("")
        }
        else {
            CStr::from_ptr(callback_data.p_message_id_name).to_string_lossy()
        };

        let message = if callback_data.p_message.is_null() {
            Cow::from("")
        }
        else {
            CStr::from_ptr(callback_data.p_message).to_string_lossy()
        };

        let level = match DebugLevel::from(message_severity) {
            DebugLevel::ERROR => log::Level::Error,
            DebugLevel::WARNING => log::Level::Warn,
            DebugLevel::INFO => log::Level::Info,
            _ => log::Level::Debug
        };

        log!(level,
            "[{:?}: {}({})] {}",
            message_type,
            message_id_name,
            &message_id_number.to_string(),
            message
        );

        ash::vk::FALSE
    }
}

impl VulkanObject for DebugUtil {
    fn hex_id(&self) -> String {
        format!("0x{:x?}", self.callback.as_raw())
    }
}
