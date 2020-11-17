use core::fmt;
use std::borrow::{Borrow, BorrowMut};
use std::cell::Cell;
use std::fmt::{Debug, Formatter};
use std::ops::Deref;
use std::rc::{Rc, Weak};
use std::result;
use std::result::Result;

use ash::{Instance, vk};
use ash::extensions::khr;
use ash::version::{DeviceV1_0, DeviceV1_2, EntryV1_0, InstanceV1_0};
use log::{debug, info};
use thiserror::Error;

use crate::graphics::vulkan::device::{Device, DeviceRef};
use crate::graphics::vulkan::surface::{Surface, SurfaceRef};
use crate::graphics::vulkan::queue::DeviceQueueRef;
use SwapchainError::{PresentationNotSupportedError, SwapchainInstantiationError};
use crate::graphics::vulkan::{VulkanError, VulkanObject};
use crate::graphics::vulkan::swapchain::SwapchainError::SwapchainVulkanError;
use ash::vk::Handle;
use crate::graphics::vulkan::instance::InstanceRef;

#[derive(Error, Debug)]
pub enum SwapchainError {

    #[error("Failed to instantiate swapchain.")]
    SwapchainInstantiationError,

    #[error("Failed to instantiate swapchain.")]
    SwapchainVulkanError(#[from] VulkanError),

    #[error("Failed to instantiate swapchain.")]
    PresentationNotSupportedError
}

pub type SwapchainRef = Rc<Swapchain>;

pub struct Swapchain {
    loader: khr::Swapchain,
    handle: vk::SwapchainKHR,
    device: DeviceRef,
    surface: SurfaceRef,
}

impl Swapchain {

    pub fn new(device: DeviceRef, queue: DeviceQueueRef, surface: SurfaceRef) -> Result<SwapchainRef, SwapchainError> {

        let _device = (*device).borrow();
        let instance = _device.instance();
        let _instance = (*instance).borrow();

        let swapchain: Rc<Swapchain> = {

            let _surface = (*surface).borrow();

            match _surface.get_surface_support(_device.physical_device(), queue) {
                Ok(true) => Ok(true),
                Ok(false) => Err(PresentationNotSupportedError),
                Err(cause) => Err(SwapchainVulkanError(cause))
            }?;

            let formats = _surface.get_formats(_device.physical_device())
                .unwrap();

            let format = formats.first().unwrap();

            let capabilities = _surface.get_capabilities(_device.physical_device())
                .unwrap();

            let mut desired_image_count = capabilities.min_image_count + 1;
            if capabilities.max_image_count > 0 && desired_image_count > capabilities.max_image_count
            {
                desired_image_count = capabilities.max_image_count;
            }

            let resolution: vk::Extent2D = match capabilities.current_extent.width {
                std::u32::MAX => vk::Extent2D {
                    width: _surface.get_width(),
                    height: _surface.get_height(),
                },
                _ => capabilities.current_extent,
            };

            let pre_transform =
                if capabilities.supported_transforms.contains(vk::SurfaceTransformFlagsKHR::IDENTITY)
                {
                    vk::SurfaceTransformFlagsKHR::IDENTITY
                } else {
                    capabilities.current_transform
                };

            let present_mode = _surface.get_present_modes(_device.physical_device())
                .unwrap()
                .iter()
                .cloned()
                .find(|&mode| mode == vk::PresentModeKHR::MAILBOX)
                .unwrap_or(vk::PresentModeKHR::FIFO);

            let swapchain_create_info = vk::SwapchainCreateInfoKHR::builder()
                .surface(*_surface.handle())
                .min_image_count(desired_image_count)
                .image_color_space(format.color_space)
                .image_format(format.format)
                .image_extent(resolution)
                .image_usage(vk::ImageUsageFlags::COLOR_ATTACHMENT)
                .image_sharing_mode(vk::SharingMode::EXCLUSIVE)
                .pre_transform(pre_transform)
                .composite_alpha(vk::CompositeAlphaFlagsKHR::OPAQUE)
                .present_mode(present_mode)
                .clipped(true)
                .image_array_layers(1)
                .build();

            let loader = ash::extensions::khr::Swapchain::new(
                _instance.handle(),
                _device.handle()
            );

            let handle = unsafe {
                loader.create_swapchain(&swapchain_create_info, None)
            }.unwrap();

            Rc::new(Swapchain {
                loader,
                handle,
                device: device.clone(),
                surface: surface.clone()
            })
        };

        (*surface).borrow_mut().attach_swapchain(swapchain.clone());

        info!("Vulkan swapchain <{}> created.", swapchain.hex_id());
        debug!("\n{:?}", swapchain);

        Ok(swapchain)
    }
}

impl VulkanObject for Swapchain {
    fn hex_id(&self) -> String {
        format!("0x{:x?}", self.handle.as_raw())
    }
}

impl Drop for Swapchain {
    fn drop(&mut self) {
        unsafe { self.loader.destroy_swapchain(self.handle, None); }
        info!("Vulkan swapchain <{}> destroyed.", self.hex_id());
    }
}

impl fmt::Debug for Swapchain {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut formatter = f.debug_struct("Swapchain");
        formatter.finish()
    }
}

struct SwapchainBuilder {

}

impl SwapchainBuilder {

}
