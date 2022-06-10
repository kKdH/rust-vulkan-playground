use std::alloc::handle_alloc_error;
use std::borrow::Cow;
use std::cell::RefCell;
use std::convert::TryFrom;
use std::convert::TryInto;
use std::ffi::{CStr, CString};
use std::fmt;
use std::fmt::{Debug, Formatter};
use std::ops::Deref;
use std::rc::Rc;
use std::time::SystemTime;

use ash::vk::{DebugUtilsMessageSeverityFlagsEXT, Handle};
use chrono::prelude::*;
use log::{debug, info, warn, error};
use thiserror::Error;

use crate::graphics::vulkan::device::PhysicalDevice;
use crate::graphics::vulkan::{VulkanError, DebugLevel, DebugUtil, VulkanObject};
use crate::util::{InvalidVersionStringError, Version};
use std::any::Any;

#[derive(Error, Debug)]
pub enum  InstanceInstantiationError {

    #[error("Required parameter `{parameter}` was not specified!")]
    MissingParameter {
        parameter: String
    },

    #[error("An Vulkan error occurs during instantiation!")]
    VulkanError(#[from] VulkanError),

    #[error("Unknown Error")]
    Unknown,
}

#[derive(Debug)]
pub struct InstanceInfo {
    application_name: CString,
    application_version: Version,
    engine_name: CString,
    engine_version: Version,
    vulkan_version: Version,
    layer_names: Vec<CString>,
    extension_names: Vec<CString>
}

pub type InstanceRef = Rc<RefCell<Instance>>;

pub struct Instance {
    info: InstanceInfo,
    loader: ash::Entry,
    handle: ash::Instance,
    physical_devices: Vec<Rc<PhysicalDevice>>,
    debug_util: Option<DebugUtil>
}

impl Instance {

    pub fn builder() -> InstanceBuilder {
        InstanceBuilder {
            application_name: None,
            application_version: None,
            vulkan_version: None,
            layers: Vec::new(),
            extensions: Vec::new(),
            debug_enabled: false,
            debug_level: DebugLevel::ERROR
        }
    }

    pub fn loader(&self) -> &ash::Entry {
        &self.loader
    }

    pub fn handle(&self) -> &ash::Instance {
        &self.handle
    }

    pub fn physical_devices(&self) -> &Vec<Rc<PhysicalDevice>> {
        &self.physical_devices
    }
}

impl VulkanObject for Instance {

    type A = ::ash::Instance;

    fn handle(&self) -> &Self::A {
        &self.handle
    }

    fn hex_id(&self) -> String {
        format!("0x{:x?}", self.handle.handle().as_raw())
    }
}

impl fmt::Debug for Instance {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut formatter = f.debug_struct("Instance");
        formatter.field("info", &self.info);
        formatter.field("physical_devices", &self.physical_devices);
        formatter.finish()
    }
}

impl Drop for Instance {
    fn drop(&mut self) {
        unsafe {
            match &self.debug_util {
                Some(debug_util) => {
                    debug_util.loader.destroy_debug_utils_messenger(debug_util.callback, None);
                    info!("Vulkan debug messanger <{}> destroyed.", debug_util.hex_id());
                },
                _ => ()
            };
            self.handle.destroy_instance(None);
        }
        info!("Vulkan instance <{}> destroyed.", self.hex_id());
    }
}

pub struct InstanceBuilder {
    application_name: Option<CString>,
    application_version: Option<Version>,
    vulkan_version: Option<Version>,
    layers: Vec<CString>,
    extensions: Vec<CString>,
    debug_enabled: bool,
    debug_level: DebugLevel,
}

impl InstanceBuilder {

    pub fn application_name(mut self, value: &str) -> InstanceBuilder {
        self.application_name = Some(CString::new(value).unwrap());
        self
    }

    pub fn application_version(mut self, value: &Version) -> InstanceBuilder {
        self.application_version = Some(value.clone());
        self
    }

    pub fn vulkan_version(mut self, value: &Version) -> InstanceBuilder {
        self.vulkan_version = Some(value.clone());
        self
    }

    pub fn layers(mut self, value: &Vec<String>) -> InstanceBuilder {
        self.layers.extend(value
            .iter()
            .cloned()
            .map(|s| CString::new(s).unwrap())
            .collect::<Vec<_>>());
        self
    }

    pub fn extensions(mut self, value: &Vec<String>) -> InstanceBuilder {
        self.extensions.extend(value
            .iter()
            .cloned()
            .map(|s| CString::new(s).unwrap())
            .collect::<Vec<_>>());
        self
    }

    pub fn debug(mut self, enabled: bool, level: DebugLevel) -> InstanceBuilder {
        self.debug_enabled = enabled;
        self.debug_level = level;
        self
    }

    pub fn build(mut self) -> Result<Rc<RefCell<Instance>>, InstanceInstantiationError> {

        let application_name = self.application_name
            .ok_or(InstanceInstantiationError::MissingParameter { parameter: String::from("application name") })?;

        let application_version = self.application_version
            .ok_or(InstanceInstantiationError::MissingParameter { parameter: String::from("application version") })?;

        if self.debug_enabled {
            self.extensions.push(CString::from(ash::extensions::ext::DebugUtils::name()));
        }

        let info = InstanceInfo {
            application_name,
            application_version,
            engine_name: CString::new("skyshard").unwrap(),
            engine_version: Version::try_from("0.1.0").unwrap(),
            vulkan_version: self.vulkan_version.unwrap_or("1.2.0".try_into().unwrap()),
            layer_names: self.layers,
            extension_names: self.extensions
        };

        let vk_app_info = ash::vk::ApplicationInfo::builder()
            .application_name(&info.application_name)
            .application_version((&info.application_version).into())
            .engine_name(&info.engine_name)
            .engine_version((&info.engine_version).into())
            .api_version((&info.vulkan_version).into())
            .build();

        let layer_names = &info.layer_names.iter()
            .map(|name| name.as_ptr())
            .collect::<Vec<_>>();

        let extension_names = info.extension_names.iter()
            .map(|name| name.as_ptr())
            .collect::<Vec<_>>();

        let vk_create_info = ash::vk::InstanceCreateInfo::builder()
            .application_info(&vk_app_info)
            .enabled_layer_names(layer_names)
            .enabled_extension_names(&extension_names)
            .build();

        let vk_loader = unsafe {
            ::ash::Entry::load()
                .expect("Failed to load vulkan")
        };

        let vk_handle = unsafe {
            vk_loader.create_instance(&vk_create_info, None)
                .expect("Failed to create instance.")
        };

        let debug_util = if self.debug_enabled {
            info!("Vulkan debugging enabled, creating debug messanger with '{}' level.", self.debug_level);
            let loader = ash::extensions::ext::DebugUtils::new(&vk_loader, &vk_handle);
            Some(DebugUtil::new(loader, self.debug_level))
        }
        else {
            info!("Vulkan debugging disabled, no debug messanger has been created.");
            None
        };

        let physical_device_handles = match unsafe { (vk_handle).enumerate_physical_devices() } {
            Ok(handles) => Ok(handles),
            Err(result) => Err(VulkanError::from(result))
        }?;

        let mut instance = Rc::new(RefCell::new(Instance {
            info,
            loader: vk_loader,
            handle: vk_handle,
            physical_devices: Vec::new(),
            debug_util
        }));

        let mut physical_devices: Vec<Rc<PhysicalDevice>> = physical_device_handles.iter()
            .map(|handle| {
                Rc::new(PhysicalDevice::new(
                    Rc::clone(&instance),
                    *handle,
                ))
            })
            .collect();

        instance.borrow_mut().physical_devices.append(&mut physical_devices);

        info!("Vulkan instance <{}> created.", instance.borrow().hex_id());
        debug!("\n{:#?}", instance.borrow());

        Ok(instance)
    }
}
