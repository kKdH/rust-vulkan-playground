use std::cell::RefCell;
use std::rc::{Rc, Weak};

use ash::extensions::khr;
use ash::vk;
use ash::vk::Handle;
use log::info;
use raw_window_handle::{HasRawDisplayHandle, HasRawWindowHandle};

use crate::graphics::vulkan::{VulkanError, VulkanObject};
use crate::graphics::vulkan::device::PhysicalDeviceRef;
use crate::graphics::vulkan::instance::Instance;
use crate::graphics::vulkan::queue::DeviceQueueRef;
use crate::graphics::vulkan::swapchain::{Swapchain, SwapchainRef};

pub type SurfaceRef = Rc<RefCell<Surface>>;

pub struct Surface {
    instance: Rc<RefCell<Instance>>,
    loader: khr::Surface,
    handle: vk::SurfaceKHR,
    swapchain: Weak<Swapchain>,
    width: u32,
    height: u32,
}

impl Surface {

    pub fn new(instance: Rc<RefCell<Instance>>, window: &winit::window::Window) -> Surface {

        let loader;
        let handle;

        {
            let _instance = instance.borrow();

            loader = khr::Surface::new(_instance.loader(), _instance.handle());
            handle = unsafe {
                ash_window::create_surface(
                    _instance.loader(),
                    _instance.handle(),
                    window.raw_display_handle(),
                    window.raw_window_handle(),
                    None
                )
            }.unwrap();
        }

        let surface = Surface {
            instance,
            loader,
            handle,
            swapchain: Weak::new(),
            width: window.inner_size().width,
            height: window.inner_size().height
        };

        info!("Vulkan surface <{}> created.", surface.hex_id());

        surface
    }

    pub fn loader(&self) -> &khr::Surface {
        &self.loader
    }

    pub fn handle(&self) -> &vk::SurfaceKHR {
        &self.handle
    }

    pub fn attach_swapchain(&mut self, swapchain: SwapchainRef) {
        self.swapchain = Rc::downgrade(&swapchain);
    }

    pub fn get_width(&self) -> u32 {
        self.width
    }

    pub fn get_height(&self) -> u32 {
        self.height
    }

    /// Query if presentation is supported by the specified device and queue.
    ///
    pub fn get_surface_support(&self, device: PhysicalDeviceRef, queue: DeviceQueueRef) -> Result<bool, VulkanError> {

        match unsafe { self.loader.get_physical_device_surface_support(*device.handle(), queue.index(), self.handle) }
        {
            Ok(support) => Ok(support),
            Err(result) => Err(VulkanError::from(result))
        }
    }

    /// Query color formats supported by this surface.
    ///
    pub fn get_formats(&self, device: PhysicalDeviceRef) -> Result<Vec<vk::SurfaceFormatKHR>, VulkanError> {

        match unsafe { self.loader.get_physical_device_surface_formats(*device.handle(), self.handle) }
        {
            Ok(formats) => Ok(formats),
            Err(result) => Err(VulkanError::from(result))
        }
    }

    /// Query surface capabilities
    ///
    pub fn get_capabilities(&self, device: PhysicalDeviceRef) -> Result<vk::SurfaceCapabilitiesKHR, VulkanError> {

        match unsafe { self.loader.get_physical_device_surface_capabilities(*device.handle(), self.handle) }
        {
            Ok(capabilities) => Ok(capabilities),
            Err(result) => Err(VulkanError::from(result))
        }
    }

    /// Query supported presentation modes
    ///
    pub fn get_present_modes(&self, device: PhysicalDeviceRef) -> Result<Vec<vk::PresentModeKHR>, VulkanError> {

        match unsafe { self.loader.get_physical_device_surface_present_modes(*device.handle(), self.handle) }
        {
            Ok(modes) => Ok(modes),
            Err(result) => Err(VulkanError::from(result))
        }
    }
}

impl VulkanObject for Surface {

    type A = ::ash::vk::SurfaceKHR;

    fn handle(&self) -> &Self::A {
        &self.handle
    }

    fn hex_id(&self) -> String {
        format!("0x{:x?}", self.handle.as_raw())
    }
}

impl Drop for Surface {
    fn drop(&mut self) {
        unsafe {
            self.loader.destroy_surface(self.handle, None);
        }
        info!("Vulkan surface <{}> destroyed.", self.hex_id());
    }
}
