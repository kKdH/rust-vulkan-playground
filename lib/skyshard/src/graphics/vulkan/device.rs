use core::fmt;
use std::borrow::Borrow;
use std::cell::{Ref, RefCell};
use std::ffi::CStr;
use std::fmt::{Formatter, Write};
use std::ops::Deref;
use std::rc::{Rc, Weak};

use std::mem::MaybeUninit;

use ash::version::{DeviceV1_0, InstanceV1_0};
use ash::vk::{Handle, CommandPoolResetFlags};
use cgmath::num_traits::ToPrimitive;
use log::{debug, info};
use thiserror::Error;

use crate::graphics::vulkan::device::DeviceError::NoSatisfyingQueueFamilyFound;
use crate::graphics::vulkan::instance::{Instance, InstanceRef};
use crate::graphics::vulkan::queue::{DeviceQueue, FmtQueueCapabilities, QueueFamily, QueueCapabilities, CapabilitiesSupport, DeviceQueueRef};
use crate::util::format_bool;
use crate::graphics::vulkan::{VulkanError, VulkanObject};
use crate::graphics::vulkan::buffer::{CommandBuffer, InternalCommandBuffer};

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
    queues: Vec<DeviceQueueRef>,
    allocator: vk_mem::Allocator,
    command_pool: Box<dyn CommandPool>,
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

    pub fn new(physical_device: Rc<PhysicalDevice>, queue_flags: QueueCapabilities, queue_count: u32) -> Result<DeviceRef, DeviceError> {

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

        let device_features = ash::vk::PhysicalDeviceFeatures::builder()
            .wide_lines(true)
            .fill_mode_non_solid(true);

        let device_create_info = ash::vk::DeviceCreateInfo::builder()
            .queue_create_infos(&queue_create_infos)
            .enabled_extension_names(&extension_names)
            .enabled_features(&device_features);

        let device: ash::Device = match unsafe {
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

        let allocator: vk_mem::Allocator = {
            let create_info = vk_mem::AllocatorCreateInfo {
                physical_device: physical_device.handle,
                device: device.clone(),
                instance: _instance.handle().clone(),
                ..Default::default()
            };
            vk_mem::Allocator::new(&create_info).unwrap()
        };

        let device = Rc::new(RefCell::new(Device {
            instance: physical_device.instance.upgrade().expect("Valid instance."),
            device: physical_device,
            handle: device,
            queues,
            allocator,
            command_pool: Box::new(UninitializedCommandPool::new()),
        }));

        let command_pool = {
            (*device).borrow().command_pool.initialize(Rc::clone(&device), queue_family)
        };

        {
            let mut device = (*device).borrow_mut();
            device.command_pool = command_pool;

            info!("Vulkan device <{}> created.", device.hex_id());
            debug!("\n{:#?}", device);
        }

        Ok(device)
    }

    pub fn instance(&self) -> InstanceRef {
        Rc::clone(&self.instance)
    }

    pub fn handle(&self) -> &ash::Device {
        &self.handle
    }

    pub fn allocator(&self) -> &vk_mem::Allocator {
        &self.allocator
    }

    pub fn physical_device(&self) -> Rc<PhysicalDevice> {
        Rc::clone(&self.device)
    }

    pub fn queues(&self) -> &Vec<DeviceQueueRef> {
        &self.queues
    }

    pub fn command_pool(&self) -> &Box<dyn CommandPool> {
        &self.command_pool
    }

    pub fn allocate_command_buffer(&mut self) -> CommandBuffer {

        let allocate_info = ash::vk::CommandBufferAllocateInfo::builder()
            .command_buffer_count(2)
            .command_pool(self.command_pool.handle())
            .level(ash::vk::CommandBufferLevel::PRIMARY);

        let command_buffer = unsafe {
            self.handle.allocate_command_buffers(&allocate_info)
        }.unwrap()[0];

        InternalCommandBuffer::new(command_buffer)
    }
}

impl VulkanObject for Device {
    fn hex_id(&self) -> String {
        format!("0x{:x?}", self.handle.handle().as_raw())
    }
}

impl Drop for Device {
    fn drop(&mut self) {

        unsafe {
            self.handle.destroy_command_pool(self.command_pool.handle(), None);
            self.handle.destroy_device(None);
        }
        info!("Vulkan device <{}> destroyed.", self.hex_id())
    }
}

pub type PhysicalDeviceRef = Rc<PhysicalDevice>;

pub struct PhysicalDevice {
    instance: Weak<RefCell<Instance>>,
    handle: ash::vk::PhysicalDevice,
    id: u32,
    name: String,
    queue_families: Vec<QueueFamily>,
    memory_properties: ash::vk::PhysicalDeviceMemoryProperties,
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

        let memory_properties = unsafe {
            _instance.handle().get_physical_device_memory_properties(handle)
        };

        PhysicalDevice {
            instance: Rc::downgrade(&instance),
            handle,
            id,
            name,
            queue_families,
            memory_properties
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
        formatter.field("memory_heaps", &self.memory_properties.memory_heap_count);
        formatter.field("memory_types", &self.memory_properties.memory_type_count);
        formatter.finish()
    }
}

pub trait CommandPool {

    fn initialize(&self, device: DeviceRef, queue_family: &QueueFamily) -> Box<dyn CommandPool>;

    fn handle(&self) -> ash::vk::CommandPool;
}

struct UninitializedCommandPool {}

impl UninitializedCommandPool {
    fn new() -> UninitializedCommandPool {
        UninitializedCommandPool {}
    }
}

impl CommandPool for UninitializedCommandPool {

    fn initialize(&self, device: DeviceRef, queue_family: &QueueFamily) -> Box<dyn CommandPool> {

        let handle = {
            let _device = (*device).borrow();

            let command_pool_create_info = ash::vk::CommandPoolCreateInfo::builder()
                .flags(ash::vk::CommandPoolCreateFlags::RESET_COMMAND_BUFFER)
                .queue_family_index(queue_family.index());

            unsafe {
                _device.handle.create_command_pool(&command_pool_create_info, None).unwrap()
            }
        };

        Box::new(InitializedCommandPool {
            handle,
            device: Rc::downgrade(&device),
            buffers: Vec::new()
        })
    }

    fn handle(&self) -> ash::vk::CommandPool {
        unimplemented!()
    }
}

struct InitializedCommandPool {
    handle: ash::vk::CommandPool,
    device: Weak<RefCell<Device>>,
    buffers: Vec<CommandBuffer>
}

impl CommandPool for InitializedCommandPool {

    fn initialize(&self, device: DeviceRef, queue_family: &QueueFamily) -> Box<dyn CommandPool> {
        unimplemented!()
    }

    fn handle(&self) -> ash::vk::CommandPool {
        self.handle
    }
}
