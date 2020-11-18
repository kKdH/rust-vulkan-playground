use std::cell::{Cell, RefCell};
use std::convert::TryInto;
use std::ffi::{CStr, CString};
use std::ops::Deref;
use std::rc::Rc;

use ash::extensions::ext::DebugUtils;
use ash::extensions::khr;
use ash::version::{EntryV1_0, InstanceV1_0};
use ash::vk;
use log::info;
use winit::window::Window;

use crate::{graphics::vulkan};
use crate::graphics::vulkan::DebugLevel;
use crate::graphics::vulkan::device::{Device, DeviceRef};
use crate::graphics::vulkan::instance::{Instance, InstanceRef};
use crate::graphics::vulkan::queue::QueueCapabilities;
use crate::graphics::vulkan::surface::{Surface, SurfaceRef};
use crate::graphics::vulkan::swapchain::{Swapchain, SwapchainRef};
use crate::util::Version;

#[derive(Debug)]
pub struct EngineError {
    message: String
}

pub struct Engine {
    instance: InstanceRef,
    device: DeviceRef,
    surface: SurfaceRef,
    swapchain: SwapchainRef,
}

impl Engine {

    pub fn reference_counts(&self) {
        info!("instance references: {} / {}", Rc::strong_count(&self.instance), Rc::weak_count(&self.instance));
        info!("device references: {} / {}", Rc::strong_count(&self.device), Rc::weak_count(&self.device));
        info!("surface references: {} / {} ", Rc::strong_count(&self.surface), Rc::weak_count(&self.surface));
        info!("swapchain references: {} / {}", Rc::strong_count(&self.swapchain), Rc::weak_count(&self.swapchain));
    }
}

impl Drop for Engine {
    fn drop(&mut self) {
        unsafe {
            // self.instance.destroy_instance(None);
        }
    }
}

pub fn create(app_name: &str, window: &Window) -> Result<Engine, EngineError> {

    let instance = Instance::builder()
        .application_name(app_name)
        .application_version(&"0.1.0".try_into().unwrap())
        .layers(&Vec::from([
            String::from("VK_LAYER_KHRONOS_validation"),
            // String::from("VK_LAYER_LUNARG_api_dump")
        ]))
        .extensions(&ash_window::enumerate_required_extensions(window)
            .expect("Failed to enumerate required vulkan extensions to create a surface!")
            .iter()
            .map(|ext| ext.to_string_lossy().into_owned())
            .collect::<Vec<_>>())
        .debug(true,DebugLevel::INFO)
        .build()
        .unwrap();

    let device;
    let surface;
    let swapchain;

    {
        let _instance = instance.borrow();

        let physical_device = _instance.physical_devices().first()
            .expect("At least one physical device.");

        surface = Rc::new(RefCell::new(Surface::new(
            Rc::clone(&instance),
            window
        )));

        device = Device::new(
            Rc::clone(physical_device),
            QueueCapabilities::GRAPHICS_OPERATIONS & QueueCapabilities::TRANSFER_OPERATIONS,
            1
        ).unwrap();

        swapchain = {
            let _device = (*device).borrow();
            let queue = _device.queues().first().unwrap();
            Swapchain::new(
                Rc::clone(&device),
                Rc::clone(queue),
                Rc::clone(&surface)
            ).unwrap()
        };

        let command_buffer = (*device).borrow_mut().allocate_command_buffer();

    }

    return Ok(Engine {
        instance,
        device,
        surface,
        swapchain,
    });
}
