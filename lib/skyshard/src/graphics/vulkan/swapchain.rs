use core::fmt;
use std::any::Any;
use std::borrow::{Borrow, BorrowMut};
use std::cell::Cell;
use std::fmt::{Debug, Formatter};
use std::ops::{BitAnd, Deref};
use std::rc::{Rc, Weak};
use std::result;
use std::result::Result;

use ash::{Instance, vk};
use ash::extensions::khr;
use ash::vk::{Extent3D, Handle, ImageView, MemoryAllocateInfo};
use log::{debug, info};
use SwapchainError::{PresentationNotSupportedError, SwapchainInstantiationError};
use thiserror::Error;

use crate::graphics::vulkan::{VulkanError, VulkanObject};
use crate::graphics::vulkan::device::{Device, DeviceRef};
use crate::graphics::vulkan::instance::InstanceRef;
use crate::graphics::vulkan::queue::DeviceQueueRef;
use crate::graphics::vulkan::surface::{Surface, SurfaceRef};
use crate::graphics::vulkan::swapchain::SwapchainError::SwapchainVulkanError;
use crate::util::HasBuilder;


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
    images: Vec<vk::Image>,
    views: Vec<vk::ImageView>,
    depth_image: vk::Image,
    depth_image_view: vk::ImageView,
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

            let images: Vec<vk::Image> = unsafe {
                loader.get_swapchain_images(handle)
            }.unwrap();

            let views: Vec<vk::ImageView> = images.iter()
                .cloned()
                .map(|image| {
                    let create_view_info = vk::ImageViewCreateInfo::builder()
                        .view_type(vk::ImageViewType::TYPE_2D)
                        .format(format.format)
                        .components(vk::ComponentMapping {
                            r: vk::ComponentSwizzle::R,
                            g: vk::ComponentSwizzle::G,
                            b: vk::ComponentSwizzle::B,
                            a: vk::ComponentSwizzle::A,
                        })
                        .subresource_range(vk::ImageSubresourceRange {
                            aspect_mask: vk::ImageAspectFlags::COLOR,
                            base_mip_level: 0,
                            level_count: 1,
                            base_array_layer: 0,
                            layer_count: 1,
                        })
                        .image(image);

                    unsafe {
                        _device.handle().create_image_view(&create_view_info, None)
                    }.unwrap()
                })
                .collect();

            let depth_image: vk::Image = unsafe {

                let image_create_info = vk::ImageCreateInfo::builder()
                    .image_type(vk::ImageType::TYPE_2D)
                    .extent(vk::Extent3D {
                        width: resolution.width,
                        height: resolution.height,
                        depth: 1,
                    })
                    .mip_levels(1)
                    .array_layers(1)
                    .format(ash::vk::Format::D32_SFLOAT_S8_UINT)
                    .tiling(ash::vk::ImageTiling::OPTIMAL)
                    .initial_layout(ash::vk::ImageLayout::UNDEFINED)
                    .usage(ash::vk::ImageUsageFlags::DEPTH_STENCIL_ATTACHMENT)
                    .samples(ash::vk::SampleCountFlags::TYPE_1)
                    .sharing_mode(ash::vk::SharingMode::EXCLUSIVE)
                    .build();

                let allocation_create_info = vk_mem::AllocationCreateInfo::builder()
                    .usage(vk_mem::MemoryUsage::GpuOnly)
                    .required_flags(ash::vk::MemoryPropertyFlags::DEVICE_LOCAL)
                    .build();

                let (image, _, _) = unsafe {
                    _device.allocator().create_image(&image_create_info, &allocation_create_info)
                        .expect("depth image")
                };

                // let memory_requirements = unsafe {
                //     _device.handle().get_image_memory_requirements(image)
                // };

                // let memory_allocation_info = MemoryAllocateInfo::builder()
                //     .allocation_size(memory_requirements.size)
                //     .memory_type_index(memory_requirements.memory_type_bits)
                //     .build();

                // let device_memory = unsafe {
                //     _device.allocator().create_image();
                //     _device.allocator().bind_image_memory()
                //     _device.handle().allocate_memory(&memory_allocation_info, None)
                //         .expect("memory allocation for depth image.")
                // };

                // unsafe {
                //     _device.handle().bind_image_memory(image, device_memory, 0)
                //         .expect("bind memory of depth image.");
                // }

                image
            };

            let depth_image_view: vk::ImageView = {
                let create_view_info = vk::ImageViewCreateInfo::builder()
                    .view_type(vk::ImageViewType::TYPE_2D)
                    .format(ash::vk::Format::D32_SFLOAT_S8_UINT)
                    .components(vk::ComponentMapping {
                        r: vk::ComponentSwizzle::R,
                        g: vk::ComponentSwizzle::G,
                        b: vk::ComponentSwizzle::B,
                        a: vk::ComponentSwizzle::A,
                    })
                    .subresource_range(vk::ImageSubresourceRange {
                        aspect_mask: vk::ImageAspectFlags::DEPTH, // TODO: Maybe set stencil bit.
                        base_mip_level: 0,
                        level_count: 1,
                        base_array_layer: 0,
                        layer_count: 1,
                    })
                    .image(depth_image);

                unsafe {
                    _device.handle().create_image_view(&create_view_info, None)
                }.unwrap()
            };

            Rc::new(Swapchain {
                loader,
                handle,
                images,
                views,
                depth_image,
                depth_image_view,
                device: device.clone(),
                surface: surface.clone()
            })
        };

        (*surface).borrow_mut().attach_swapchain(swapchain.clone());

        info!("Vulkan swapchain <{}> created.", swapchain.hex_id());
        debug!("\n{:#?}", swapchain);

        Ok(swapchain)
    }

    pub fn handle(&self) -> &ash::vk::SwapchainKHR {
        &self.handle
    }

    pub fn views(&self) -> &Vec<ash::vk::ImageView> {
        &self.views
    }

    pub fn depth_image_view(&self) -> &ash::vk::ImageView {
        &self.depth_image_view
    }

    /// Returns the next image's index and whether the swapchain is suboptimal for the surface.
    ///
    pub fn acquire_next_image(&self, semaphore: ash::vk::Semaphore) -> (u32, bool) {
        unsafe {
            self.loader.acquire_next_image(
                self.handle,
                std::u64::MAX,
                semaphore,
                ash::vk::Fence::null()
            )
        }.expect("Acquire next image")
    }

    pub fn queue_present(&self, queue: DeviceQueueRef, present_info: &ash::vk::PresentInfoKHR) {
        unsafe {
            self.loader.queue_present(*queue.handle(), present_info)
        }.expect("Queue Present");
    }
}

impl VulkanObject for Swapchain {
    fn hex_id(&self) -> String {
        format!("0x{:x?}", self.handle.as_raw())
    }
}

impl Drop for Swapchain {
    fn drop(&mut self) {
        let _device = (*self.device).borrow();
        unsafe {
            self.views.iter().for_each(|view| {
                _device.handle().destroy_image_view(*view, None)
            });
            self.loader.destroy_swapchain(self.handle, None);
        }
        info!("Vulkan swapchain <{}> destroyed.", self.hex_id());
    }
}

impl fmt::Debug for Swapchain {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut formatter = f.debug_struct("Swapchain");
        formatter.field("images", &self.images.len());
        formatter.field("views", &self.views.len());
        formatter.finish()
    }
}
