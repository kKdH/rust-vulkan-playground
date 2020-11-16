use core::fmt;
use std::borrow::Borrow;
use std::cell::{Ref, RefCell};
use std::ffi::CStr;
use std::fmt::{Formatter, Write};
use std::ops::Deref;
use std::rc::{Rc, Weak};

use ash::version::{DeviceV1_0, InstanceV1_0};
use ash::vk::Handle;
use cgmath::num_traits::ToPrimitive;
use log::{debug, info};
use thiserror::Error;

use crate::graphics::vulkan::device::DeviceError::NoSatisfyingQueueFamilyFound;
use crate::graphics::vulkan::instance::{Instance, InstanceRef};
use crate::graphics::vulkan::queue::{DeviceQueue, FmtQueueCapabilities, QueueFamily, QueueCapabilities, CapabilitiesSupport, DeviceQueueRef};
use crate::util::format_bool;
use crate::graphics::vulkan::VulkanError;

#[derive(Error, Debug)]
pub enum DeviceError {

    #[error("There is no queue family providing the requested capabilities: {requested_queue_flags:?} or the number of queues: {requested_queue_count}")]
    NoSatisfyingQueueFamilyFound {
        requested_queue_flags: FmtQueueCapabilities,
        requested_queue_count: u32,
        found: Vec<QueueFamily>,
    },

    #[error("Failed to create device: {source}")]
    DeviceInstantiationError {
        #[from]
        source: VulkanError,
    }
}

pub type DeviceRef = Rc<RefCell<Device>>;

pub struct Device {
    instance: InstanceRef,
    device: PhysicalDeviceRef,
    handle: ash::Device,
    queues: Vec<DeviceQueueRef>
}

impl fmt::Debug for Device {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut formatter = f.debug_struct("Device");
        formatter.field("name", &self.device.name);
        formatter.field("queues", &self.queues);
        formatter.finish()
    }
}

impl Device {

    pub fn new(physical_device: Rc<PhysicalDevice>, queue_flags: QueueCapabilities, queue_count: u32) -> Result<Device, DeviceError> {

        let _instance = physical_device.instance.upgrade().expect("Valid Instance");
        let _instance = (*_instance).borrow();

        let queue_families = physical_device.queue_families(queue_flags, queue_count);
        let queue_family = queue_families.first()
            .ok_or(NoSatisfyingQueueFamilyFound {
                requested_queue_flags: FmtQueueCapabilities::from(queue_flags),
                requested_queue_count: queue_count,
                found: physical_device.queue_families(QueueCapabilities::ANY, 0)
            })?;

        let queue_priorities = [1.0];
        let queue_create_infos = (0..queue_count).map(|index| {
            ash::vk::DeviceQueueCreateInfo::builder()
                .queue_family_index(queue_family.index())
                .queue_priorities(&queue_priorities)
                .build()
        }).collect::<Vec<_>>();

        let extension_names = [
            ash::extensions::khr::Swapchain::name().as_ptr(),
        ];

        let device_create_info = ash::vk::DeviceCreateInfo::builder()
            .queue_create_infos(&queue_create_infos)
            .enabled_extension_names(&extension_names)
            // .enabled_features(&features)
            .build();

        let device = match unsafe {
            _instance.handle().create_device(physical_device.handle, &device_create_info, None)
        } {
            Ok(device) => Ok(device),
            Err(result) => Err(VulkanError::from(result))
        }?;

        let queues: Vec<Rc<DeviceQueue>> = (0..queue_count)
            .map(|queue_index| unsafe {
                (queue_index, device.get_device_queue(queue_family.index(), queue_index))
            })
            .map(|(index, handle)| Rc::new(DeviceQueue::new(handle, index, *queue_family)))
            .collect();

        let device = Device {
            instance: physical_device.instance.upgrade().expect("Valid instance."),
            device: physical_device,
            handle: device,
            queues
        };

        info!("Vulkan device created.");
        debug!("\n{:#?}", device);

        Ok(device)
    }

    pub fn instance(&self) -> InstanceRef {
        Rc::clone(&self.instance)
    }

    pub fn handle(&self) -> &ash::Device {
        &self.handle
    }

    pub fn physical_device(&self) -> Rc<PhysicalDevice> {
        Rc::clone(&self.device)
    }

    pub fn queues(&self) -> &Vec<DeviceQueueRef> {
        &self.queues
    }
}

impl Drop for Device {
    fn drop(&mut self) {
        unsafe { self.handle.destroy_device(None); }
        info!("Vulkan device destroyed.")
    }
}

pub type PhysicalDeviceRef = Rc<PhysicalDevice>;

pub struct PhysicalDevice {
    instance: Weak<RefCell<Instance>>,
    handle: ash::vk::PhysicalDevice,
    id: u32,
    name: String,
    queue_families: Vec<QueueFamily>,
}

impl PhysicalDevice {

    pub fn new(
        instance: Rc<RefCell<Instance>>,
        handle: ash::vk::PhysicalDevice,
    ) -> PhysicalDevice {

        let _instance = (*instance).borrow();

        let (id, name, properties) = unsafe {
            let properties = _instance.handle().get_physical_device_properties(handle);
            (
                properties.device_id,
                CStr::from_ptr(properties.device_name.as_ptr()).to_str().unwrap().to_owned(),
                properties
            )
        };

        let queue_families: Vec<QueueFamily> = unsafe {
            _instance.handle().get_physical_device_queue_family_properties(handle)
        }.iter().enumerate().map(|(index, familiy)| {
            QueueFamily::new (
                index.to_u32().unwrap(),
                familiy.queue_flags.as_raw(),
                familiy.queue_count
            )
        }).collect();

        PhysicalDevice {
            instance: Rc::downgrade(&instance),
            handle,
            id,
            name,
            queue_families
        }
    }

    pub fn handle(&self) -> &ash::vk::PhysicalDevice {
        &self.handle
    }

    pub fn queue_families(&self, capabilities: QueueCapabilities, queue_count: u32) -> Vec<QueueFamily> {
        self.queue_families.iter()
            .filter(|family| family.supports(capabilities) && family.queues() >= queue_count)
            .cloned()
            .collect::<Vec<_>>()
    }
}

impl fmt::Debug for PhysicalDevice {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let mut formatter = f.debug_struct("PhysicalDevice");
        formatter.field("id", &self.id);
        formatter.field("name", &self.name);
        formatter.field("queue_families", &self.queue_families);
        formatter.finish()
    }
}
