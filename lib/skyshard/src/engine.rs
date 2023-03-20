extern crate ash;

use std::borrow::Borrow;
use std::cell::{Ref, RefCell};
use std::convert::TryInto;
use std::ffi::CStr;
use std::io::Write;
use std::rc::Rc;

use ash::vk;
use ash::vk::{CommandBufferResetFlags, ImageView, Offset3D};
use log::info;
use nalgebra::Matrix4;
use raw_window_handle::{HasRawDisplayHandle, HasRawWindowHandle};
use winit::window::Window;

use crate::assets::AssetsManager;
use crate::entity::World;
use crate::graphics::{Extent, Geometry, Material};
use crate::graphics::Camera;
use crate::graphics::vulkan::DebugLevel;
use crate::graphics::vulkan::device::{Device, DeviceRef};
use crate::graphics::vulkan::instance::{Instance, InstanceRef};
use crate::graphics::vulkan::queue::QueueCapabilities;
use crate::graphics::vulkan::renderpass::create_render_pass;
use crate::graphics::vulkan::resources::{Buffer, CopyDestination, Image, ImageAllocationDescriptor, ImageFormat, ImageTiling, ImageUsage, Resource, ResourceManager};
use crate::graphics::vulkan::resources::{BufferAllocationDescriptor, BufferUsage, MemoryLocation};
use crate::graphics::vulkan::shaders::{FragmentShaderBinary, ShaderModule, VertexShaderBinary};
use crate::graphics::vulkan::surface::{Surface, SurfaceRef};
use crate::graphics::vulkan::swapchain::{Swapchain, SwapchainRef};
use crate::graphics::vulkan::VulkanObject;
use crate::util::HasBuilder;

#[repr(C, align(16))]
#[derive(Clone, Debug, Copy)]
pub struct Vertex {
    pub position: [f32; 3],
    pub color: [f32; 3],
    pub uv: [f32; 2]
}

#[repr(C, align(16))]
#[derive(Clone, Debug, Copy)]
pub struct InstanceData {
    pub id: u32,
    pub transformation: [f32; 16],
}

#[repr(C, align(16))]
#[derive(Clone, Debug, Copy)]
struct UniformBufferObject {
    mvp: Matrix4<f32>,
}

#[derive(Debug)]
pub struct EngineError {
    message: String
}

pub struct Engine {
    instance: InstanceRef,
    device: DeviceRef,
    resource_manager: ResourceManager,
    assets_manager: AssetsManager,
    surface: SurfaceRef,
    swapchain: SwapchainRef,
    renderpass: ash::vk::RenderPass,
    viewports: [ash::vk::Viewport; 1],
    scissors: [ash::vk::Rect2D; 1],
    pipelines: Vec<ash::vk::Pipeline>,
    pipeline_layout: ash::vk::PipelineLayout,
    frame_buffers: Vec<ash::vk::Framebuffer>,
    command_buffers: Vec<ash::vk::CommandBuffer>,
    descriptor_pool: ::ash::vk::DescriptorPool,
    global_descriptor_sets: Vec<ash::vk::DescriptorSet>,
    material_descriptor_set_layout: ::ash::vk::DescriptorSetLayout,
    texture_sampler: ::ash::vk::Sampler,
    ubo_buffer: Buffer<UniformBufferObject>,
    object_id_lookup_images: Vec<Image>,
    object_id_lookup_images_views: Vec<ImageView>,
    object_id_lookup_buffer: Buffer<u32>,
    image_available_semaphore: ash::vk::Semaphore,
    render_finished_semaphore: ash::vk::Semaphore,
    command_buffers_completed_fence: ash::vk::Fence,
    timings_query_pool: ash::vk::QueryPool,
    vertices_query_pool: ash::vk::QueryPool,
    last_timing_value: [u64; 1],
    last_swapchain_image_index: u32,
}

impl Engine {

    pub fn reference_counts(&self) {
        info!("instance references: {} / {}", Rc::strong_count(&self.instance), Rc::weak_count(&self.instance));
        info!("device references: {} / {}", Rc::strong_count(&self.device), Rc::weak_count(&self.device));
        info!("surface references: {} / {} ", Rc::strong_count(&self.surface), Rc::weak_count(&self.surface));
        info!("swapchain references: {} / {}", Rc::strong_count(&self.swapchain), Rc::weak_count(&self.swapchain));
    }

    pub fn asset_manager(&self) -> &AssetsManager {
        &self.assets_manager
    }
}

impl Drop for Engine {
    fn drop(&mut self) {
        unsafe {
            // self.instance.destroy_instance(None);
        }
    }
}

pub fn create(
    app_name: &str,
    window: &Window,
    vertex_shader: VertexShaderBinary,
    fragment_shader: FragmentShaderBinary,
) -> Result<Engine, EngineError> {

    let instance = Instance::builder()
        .application_name(app_name)
        .application_version(&"0.1.0".try_into().unwrap())
        .extensions(&ash_window::enumerate_required_extensions(window.raw_display_handle())
            .expect("Failed to enumerate required vulkan extensions to create a surface!")
            .iter()
            .map(|ptr| unsafe { String::from(CStr::from_ptr(*ptr).to_string_lossy()) })
            .collect::<Vec<_>>())
        .debug(true,DebugLevel::DEBUG)
        .build()
        .expect("Failed to create vulkan instance");

    let device;
    let mut resource_manager;
    let assets_manager;
    let surface;
    let swapchain;
    let frame_buffers: Vec<ash::vk::Framebuffer>;
    let command_buffers: Vec<ash::vk::CommandBuffer>;
    let renderpass: ash::vk::RenderPass;
    let viewports: [ash::vk::Viewport; 1];
    let scissors: [ash::vk::Rect2D; 1];
    let pipelines: Vec<ash::vk::Pipeline>;
    let pipeline_layout: ::ash::vk::PipelineLayout;
    let descriptor_pool: ::ash::vk::DescriptorPool;
    let global_descriptor_sets: Vec<ash::vk::DescriptorSet>;
    let material_descriptor_set_layout: ::ash::vk::DescriptorSetLayout;
    let texture_sampler: ::ash::vk::Sampler;
    let ubo_buffer: Buffer<UniformBufferObject>;
    let index_buffer: ash::vk::Buffer;
    let vertex_buffer: ash::vk::Buffer;
    let object_id_lookup_images: Vec<Image>;
    let object_id_lookup_images_views: Vec<ImageView>;
    let object_id_lookup_buffer: Buffer<u32>;
    let image_available_semaphore: ash::vk::Semaphore;
    let render_finished_semaphore: ash::vk::Semaphore;
    let command_buffers_completed_fence: ash::vk::Fence;
    let timings_query_pool: ash::vk::QueryPool;
    let vertices_query_pool: ash::vk::QueryPool;

    {
        let _instance = (*instance).borrow();

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

        resource_manager = ResourceManager::new(
            (*instance).borrow().handle(),
            (*device).borrow().handle(),
            (*physical_device).handle()
        ).expect("Failed to create ResourceManager");

        assets_manager = AssetsManager::new(&String::from("./assets"))
            .expect("Failed to create AssetsManager");

        swapchain = {
            let _device = (*device).borrow();
            let queue = _device.queues().first().unwrap();
            Swapchain::new(
                Rc::clone(&device),
                Rc::clone(queue),
                Rc::clone(&surface),
                &mut resource_manager,
            ).unwrap()
        };

        {
            let _device = (*device).borrow();

            let create_info = ash::vk::QueryPoolCreateInfo::builder()
                .query_type(ash::vk::QueryType::TIMESTAMP)
                .query_count(2);

            timings_query_pool = unsafe {
                _device.handle().create_query_pool(&create_info, None)
            }.expect("QueryPool creation failed.");

            let create_info = ash::vk::QueryPoolCreateInfo::builder()
                .query_type(ash::vk::QueryType::PIPELINE_STATISTICS)
                .query_count(1)
                .pipeline_statistics(ash::vk::QueryPipelineStatisticFlags::VERTEX_SHADER_INVOCATIONS);

            vertices_query_pool = unsafe {
                _device.handle().create_query_pool(&create_info, None)
            }.expect("QueryPool creation failed.");
        }

        let _device = (*device).borrow();

        let vertex_shader_module = ShaderModule::create(vertex_shader, "main")
            .and_then(|module| module.load(&_device))
            .unwrap();

        let fragment_shader_module = ShaderModule::create(fragment_shader, "main")
            .and_then(|module| module.load(&_device))
            .unwrap();

        let shader_stage_create_infos = [
            vertex_shader_module.create_pipeline_shader_stage_create_info(),
            fragment_shader_module.create_pipeline_shader_stage_create_info(),
        ];

        let vertex_input_binding_descriptors = [
            ash::vk::VertexInputBindingDescription::builder()
                .binding(0)
                .stride(std::mem::size_of::<Vertex>() as u32)
                .input_rate(ash::vk::VertexInputRate::VERTEX)
                .build(),
            ash::vk::VertexInputBindingDescription::builder()
                .binding(1)
                .stride(std::mem::size_of::<InstanceData>() as u32)
                .input_rate(ash::vk::VertexInputRate::INSTANCE)
                .build(),
        ];

        let vertex_input_attribute_descriptors = [
            ash::vk::VertexInputAttributeDescription::builder()
                .binding(0)
                .location(0)
                .format(ash::vk::Format::R32G32B32_SFLOAT)
                .offset(offset_of!(Vertex, position) as u32)
                .build(),
            ash::vk::VertexInputAttributeDescription::builder()
                .binding(0)
                .location(1)
                .format(ash::vk::Format::R32G32B32_SFLOAT)
                .offset(offset_of!(Vertex, color) as u32)
                .build(),
            ash::vk::VertexInputAttributeDescription::builder()
                .binding(0)
                .location(2)
                .format(ash::vk::Format::R32G32_SFLOAT)
                .offset(offset_of!(Vertex, uv) as u32)
                .build(),
            ash::vk::VertexInputAttributeDescription::builder()
                .binding(1)
                .location(3)
                .format(ash::vk::Format::R32_UINT)
                .offset(offset_of!(InstanceData, id) as u32)
                .build(),
            ash::vk::VertexInputAttributeDescription::builder()
                .binding(1)
                .location(4)
                .format(ash::vk::Format::R32G32B32A32_SFLOAT)
                .offset((offset_of!(InstanceData, transformation) + 0) as u32)
                .build(),
            ash::vk::VertexInputAttributeDescription::builder()
                .binding(1)
                .location(5)
                .format(ash::vk::Format::R32G32B32A32_SFLOAT)
                .offset((offset_of!(InstanceData, transformation) + 16) as u32)
                .build(),
            ash::vk::VertexInputAttributeDescription::builder()
                .binding(1)
                .location(6)
                .format(ash::vk::Format::R32G32B32A32_SFLOAT)
                .offset((offset_of!(InstanceData, transformation) + 32) as u32)
                .build(),
            ash::vk::VertexInputAttributeDescription::builder()
                .binding(1)
                .location(7)
                .format(ash::vk::Format::R32G32B32A32_SFLOAT)
                .offset((offset_of!(InstanceData, transformation) + 48) as u32)
                .build(),
        ];

        let vertex_input_state_info = ash::vk::PipelineVertexInputStateCreateInfo::builder()
            .vertex_binding_descriptions(&vertex_input_binding_descriptors)
            .vertex_attribute_descriptions(&vertex_input_attribute_descriptors);

        let vertex_input_assembly_state_info = ash::vk::PipelineInputAssemblyStateCreateInfo::builder()
            .topology(vk::PrimitiveTopology::TRIANGLE_LIST)
            .primitive_restart_enable(false);

        viewports = [vk::Viewport {
            x: 0.0,
            y: 0.0,
            width: window.inner_size().width as f32,
            height: window.inner_size().height as f32,
            min_depth: 0.0,
            max_depth: 1.0,
        }];

        scissors = [ash::vk::Rect2D {
            offset: ash::vk::Offset2D { x: 0, y: 0 },
            extent: ash::vk::Extent2D {
                width: window.inner_size().width,
                height: window.inner_size().height
            },
        }];

        let viewport_state_info = vk::PipelineViewportStateCreateInfo::builder()
            .scissors(&scissors)
            .viewports(&viewports);

        let rasterization_info_fill_mode = ash::vk::PipelineRasterizationStateCreateInfo::builder()
            .depth_clamp_enable(false)
            .rasterizer_discard_enable(false)
            .polygon_mode(ash::vk::PolygonMode::FILL)
            .line_width(1.0)
            .cull_mode(ash::vk::CullModeFlags::BACK)
            .front_face(ash::vk::FrontFace::COUNTER_CLOCKWISE)
            .depth_bias_enable(false)
            .depth_bias_constant_factor(0.0)
            .depth_bias_clamp(0.0)
            .depth_bias_slope_factor(0.0)
            .build();

        let rasterization_info_line_mode = ash::vk::PipelineRasterizationStateCreateInfo::builder()
            .depth_clamp_enable(false)
            .rasterizer_discard_enable(false)
            .polygon_mode(ash::vk::PolygonMode::LINE)
            .line_width(3.0)
            .cull_mode(ash::vk::CullModeFlags::BACK)
            .front_face(ash::vk::FrontFace::COUNTER_CLOCKWISE)
            .depth_bias_enable(false)
            .depth_bias_constant_factor(0.0)
            .depth_bias_clamp(0.0)
            .depth_bias_slope_factor(0.0)
            .build();

        let multisample_state_info = vk::PipelineMultisampleStateCreateInfo::builder()
            .sample_shading_enable(false)
            .rasterization_samples(vk::SampleCountFlags::TYPE_1)
            .min_sample_shading(1.0)
            .alpha_to_coverage_enable(false)
            .alpha_to_one_enable(false)
            .build();

        let noop_stencil_state = vk::StencilOpState {
            fail_op: vk::StencilOp::KEEP,
            pass_op: vk::StencilOp::KEEP,
            depth_fail_op: vk::StencilOp::KEEP,
            compare_op: vk::CompareOp::ALWAYS,
            ..Default::default()
        };

        let depth_state_info = vk::PipelineDepthStencilStateCreateInfo::builder()
            .depth_test_enable(true)
            .depth_write_enable(true)
            .depth_compare_op(vk::CompareOp::LESS)
            .depth_bounds_test_enable(false)
            .stencil_test_enable(false)
            .build();

        // per frame buffer
        let color_blend_attachment_states = [
            vk::PipelineColorBlendAttachmentState::builder()
                .color_write_mask(
                    vk::ColorComponentFlags::R
                        | vk::ColorComponentFlags::G
                        | vk::ColorComponentFlags::B
                        | vk::ColorComponentFlags::A
                )
                .blend_enable(false)
                .src_color_blend_factor(ash::vk::BlendFactor::ONE)
                .dst_color_blend_factor(ash::vk::BlendFactor::ZERO)
                .color_blend_op(ash::vk::BlendOp::ADD)
                .src_color_blend_factor(ash::vk::BlendFactor::ONE)
                .dst_alpha_blend_factor(ash::vk::BlendFactor::ZERO)
                .alpha_blend_op(ash::vk::BlendOp::ADD)
                .build(),
            vk::PipelineColorBlendAttachmentState::builder()
                .color_write_mask(
                    vk::ColorComponentFlags::R
                        | vk::ColorComponentFlags::G
                        | vk::ColorComponentFlags::B
                        | vk::ColorComponentFlags::A
                )
                .blend_enable(false)
                .src_color_blend_factor(ash::vk::BlendFactor::ONE)
                .dst_color_blend_factor(ash::vk::BlendFactor::ZERO)
                .color_blend_op(ash::vk::BlendOp::ADD)
                .src_color_blend_factor(ash::vk::BlendFactor::ONE)
                .dst_alpha_blend_factor(ash::vk::BlendFactor::ZERO)
                .alpha_blend_op(ash::vk::BlendOp::ADD)
                .build(),
        ];

        // for all frame buffers - global
        let color_blend_state = vk::PipelineColorBlendStateCreateInfo::builder()
            .logic_op_enable(false)
            .logic_op(ash::vk::LogicOp::COPY)
            .attachments(&color_blend_attachment_states)
            .blend_constants([0.0, 0.0, 0.0, 0.0]);

        let dynamic_state = [
            ash::vk::DynamicState::VIEWPORT,
            ash::vk::DynamicState::SCISSOR
        ];

        let dynamic_state_info = ash::vk::PipelineDynamicStateCreateInfo::builder()
            .dynamic_states(&dynamic_state);

        descriptor_pool = {
            let pool_sizes = {
                let count = (*swapchain).views().len() as u32;
                [
                    ::ash::vk::DescriptorPoolSize::builder()
                        .ty(ash::vk::DescriptorType::UNIFORM_BUFFER)
                        .descriptor_count(count)
                        .build(),
                    ::ash::vk::DescriptorPoolSize::builder()
                        .ty(::ash::vk::DescriptorType::SAMPLER)
                        .descriptor_count(count * 16) // TODO: how many sampler descriptors do we need?
                        .build(),
                ]
            };

            let pool_create_info = ash::vk::DescriptorPoolCreateInfo::builder()
                .max_sets(1024) // TODO: how many descriptor sets do we need?
                .pool_sizes(&pool_sizes)
                .build();

            unsafe {
                _device.handle().create_descriptor_pool(&pool_create_info, None)
                    .expect("Failed to create descriptor pool")
            }
        };

        let global_descriptor_set_layout = {
            let bindings = [
                ::ash::vk::DescriptorSetLayoutBinding::builder()
                    .binding(0)
                    .descriptor_count(1)
                    .stage_flags(::ash::vk::ShaderStageFlags::VERTEX)
                    .descriptor_type(ash::vk::DescriptorType::UNIFORM_BUFFER)
                    .build(),
            ];

            let create_info = ::ash::vk::DescriptorSetLayoutCreateInfo::builder()
                .bindings(&bindings)
                .build();

            unsafe {
                _device.handle().create_descriptor_set_layout(&create_info, None)
                    .expect("Failed to create descriptor set!")
            }
        };

        material_descriptor_set_layout = {
            let bindings = [
                ::ash::vk::DescriptorSetLayoutBinding::builder()
                    .binding(0)
                    .descriptor_count(1)
                    .stage_flags(::ash::vk::ShaderStageFlags::FRAGMENT)
                    .descriptor_type(::ash::vk::DescriptorType::COMBINED_IMAGE_SAMPLER)
                    .build()
            ];

            let descriptor_set_layout_create_info = ::ash::vk::DescriptorSetLayoutCreateInfo::builder()
                .bindings(&bindings)
                .build();

            unsafe {
                _device.handle().create_descriptor_set_layout(&descriptor_set_layout_create_info, None)
                    .expect("Failed to create descriptor set layout!")
            }
        };

        global_descriptor_sets = swapchain.views().iter().map(|_| {
            let descriptor_set_layouts = [
                global_descriptor_set_layout
            ];
            let create_info = ash::vk::DescriptorSetAllocateInfo::builder()
                .descriptor_pool(descriptor_pool)
                .set_layouts(&descriptor_set_layouts);
            unsafe {
                _device.handle().allocate_descriptor_sets(&create_info)
                    .expect("Failed to allocate descriptor set")[0]
            }
        }).collect::<Vec<_>>();

        pipeline_layout = {

            let descriptor_set_layouts = [
                global_descriptor_set_layout,
                material_descriptor_set_layout,
            ];

            let pipeline_layout_create_info = ::ash::vk::PipelineLayoutCreateInfo::builder()
                .set_layouts(&descriptor_set_layouts)
                .build();

            unsafe {
                _device.handle().create_pipeline_layout(&pipeline_layout_create_info, None)
                    .expect("Failed to create pipeline layout!")
            }
        };

        renderpass = create_render_pass(device.clone(), surface.clone());

        let default_graphic_pipeline_info = ::ash::vk::GraphicsPipelineCreateInfo::builder()
            .flags(::ash::vk::PipelineCreateFlags::ALLOW_DERIVATIVES)
            .stages(&shader_stage_create_infos)
            .vertex_input_state(&vertex_input_state_info)
            .input_assembly_state(&vertex_input_assembly_state_info)
            .viewport_state(&viewport_state_info)
            .rasterization_state(&rasterization_info_fill_mode)
            .multisample_state(&multisample_state_info)
            .depth_stencil_state(&depth_state_info)
            .color_blend_state(&color_blend_state)
            .dynamic_state(&dynamic_state_info)
            .layout(pipeline_layout)
            .render_pass(renderpass)
            .subpass(0)
            .build();

        let line_mode_graphic_pipeline_info = ::ash::vk::GraphicsPipelineCreateInfo::builder()
            .flags(::ash::vk::PipelineCreateFlags::DERIVATIVE)
            .base_pipeline_index(0)
            .stages(&shader_stage_create_infos)
            .vertex_input_state(&vertex_input_state_info)
            .input_assembly_state(&vertex_input_assembly_state_info)
            .viewport_state(&viewport_state_info)
            .rasterization_state(&rasterization_info_line_mode)
            .multisample_state(&multisample_state_info)
            .depth_stencil_state(&depth_state_info)
            .color_blend_state(&color_blend_state)
            .dynamic_state(&dynamic_state_info)
            .layout(pipeline_layout)
            .render_pass(renderpass)
            .subpass(0)
            .build();

        pipelines = unsafe {
            _device.handle().create_graphics_pipelines(
                ::ash::vk::PipelineCache::null(),
                &[
                    default_graphic_pipeline_info,
                    line_mode_graphic_pipeline_info,
                ],
                None,
            )
        }.expect("Failed to create graphic pipelines");

        object_id_lookup_images = swapchain.views().iter().enumerate().map(|(index, _)| {
            resource_manager.create_image(format!("object-id-lookup-{index:?}"), &ImageAllocationDescriptor {
                usage: [ImageUsage::ColorAttachment, ImageUsage::TransferSource],
                extent: Extent::from(window.inner_size().width, window.inner_size().height, 1),
                format: ImageFormat::UInt32,
                tiling: ImageTiling::Linear,
                memory: MemoryLocation::GpuOnly
            }).expect("Failed to create object-id-lookup image")
        }).collect::<Vec<_>>();

        object_id_lookup_images_views = object_id_lookup_images.iter().map(|image| {

            let create_image_view_info = vk::ImageViewCreateInfo::builder()
                .view_type(vk::ImageViewType::TYPE_2D)
                .format(ash::vk::Format::R32_UINT)
                .components(vk::ComponentMapping {
                    r: vk::ComponentSwizzle::R,
                    g: vk::ComponentSwizzle::IDENTITY,
                    b: vk::ComponentSwizzle::IDENTITY,
                    a: vk::ComponentSwizzle::IDENTITY,
                })
                .subresource_range(vk::ImageSubresourceRange {
                    aspect_mask: vk::ImageAspectFlags::COLOR,
                    base_mip_level: 0,
                    level_count: 1,
                    base_array_layer: 0,
                    layer_count: 1,
                })
                .image(*image.handle());

            unsafe {
                _device.handle().create_image_view(&create_image_view_info, None)
            }.unwrap()
        }).collect::<Vec<_>>();

        frame_buffers = swapchain.views().iter().zip(&object_id_lookup_images_views).map(|(view, object_id_lookup)| {

            let attachments = [*view, *swapchain.depth_image_view(), *object_id_lookup];

            let create_info = ash::vk::FramebufferCreateInfo::builder()
                .render_pass(renderpass)
                .attachments(&attachments)
                .width(window.inner_size().width)
                .height(window.inner_size().height)
                .layers(1);

            unsafe {
                _device.handle().create_framebuffer(&create_info, None)
            }.unwrap()
        }).collect::<Vec<_>>();

        command_buffers = (0..swapchain.views().len()).map(|_| {
            let create_info = ash::vk::CommandBufferAllocateInfo::builder()
                .command_pool(_device.command_pool().handle())
                .command_buffer_count(1)
                .level(ash::vk::CommandBufferLevel::PRIMARY)
                .build();

            unsafe {
                _device.handle().allocate_command_buffers(&create_info)
            }.unwrap()[0]
        }).collect::<Vec<_>>();

        let semaphore_create_info = ash::vk::SemaphoreCreateInfo::builder()
            .flags(ash::vk::SemaphoreCreateFlags::default());

        image_available_semaphore = unsafe {
            _device.handle().create_semaphore(&semaphore_create_info, None)
        }.unwrap();

        render_finished_semaphore = unsafe {
            _device.handle().create_semaphore(&semaphore_create_info, None)
        }.unwrap();

        command_buffers_completed_fence = {
            let fence_create_info = ::ash::vk::FenceCreateInfo::builder()
                .build();

            unsafe {
                _device.handle().create_fence(&fence_create_info, None)
                    .expect("Expected successfull fence creation!")
            }
        };

        ubo_buffer = {
            let count = swapchain.views().len(); // one ubo per swapchain image
            let size: usize = count * std::mem::size_of::<UniformBufferObject>();

            let mut buffer = resource_manager.create_buffer(String::from("uniform-buffer"), &BufferAllocationDescriptor {
                usage: [BufferUsage::UniformBuffer],
                memory: MemoryLocation::CpuToGpu
            }, count).expect("Failed to create uniform buffer");

            let ubos = (0..count).map(|_| {
                UniformBufferObject {
                    mvp: Matrix4::identity(),
                }
            }).collect::<Vec<_>>();

            unsafe {
                resource_manager.copy(&ubos, &mut buffer, 0, count);
                resource_manager.flush(&mut buffer, 0, count);
            }

            buffer
        };

        object_id_lookup_buffer = {

            let count = {
                let extent = object_id_lookup_images[0].extent();
                (extent.width * extent.height * extent.depth) as usize
            };

            let mut buffer = resource_manager.create_buffer(String::from("object-lookup-buffer"), &BufferAllocationDescriptor {
                usage: [BufferUsage::TransferDestinationBuffer],
                memory: MemoryLocation::GpuToCpu
            }, count).expect("Failed to create object-lookup-buffer");

            unsafe {
                resource_manager.copy(&vec![0u32; count], &mut buffer, 0, count);
                resource_manager.flush(&mut buffer, 0, count);
            }

            buffer
        };

        texture_sampler = {
            let sampler_create_info = ::ash::vk::SamplerCreateInfo::builder()
                .mag_filter(::ash::vk::Filter::LINEAR)
                .min_filter(::ash::vk::Filter::LINEAR)
                .address_mode_u(::ash::vk::SamplerAddressMode::REPEAT)
                .address_mode_v(::ash::vk::SamplerAddressMode::REPEAT)
                .address_mode_w(::ash::vk::SamplerAddressMode::REPEAT)
                .border_color(::ash::vk::BorderColor::FLOAT_OPAQUE_WHITE);

            unsafe {
                _device.handle().create_sampler(&sampler_create_info, None)
                    .expect("Failed to create sampler")
            }
        };

        {
            let size: usize = std::mem::size_of::<UniformBufferObject>();
            global_descriptor_sets.iter().enumerate().for_each(|(index, descriptor_set)| {
                let buffer_info = [
                    ::ash::vk::DescriptorBufferInfo::builder()
                        .buffer(*ubo_buffer.handle())
                        .offset((index * size) as u64)
                        .range(size as u64)
                        .build()
                ];
                let descriptor_writes = [
                    ::ash::vk::WriteDescriptorSet::builder()
                        .descriptor_type(ash::vk::DescriptorType::UNIFORM_BUFFER)
                        .dst_set(*descriptor_set)
                        .dst_binding(0)
                        .buffer_info(&buffer_info)
                        .build(),
                ];
                let descriptor_copies: [ash::vk::CopyDescriptorSet; 0] = [];
                unsafe {
                    _device.handle().update_descriptor_sets(&descriptor_writes, &descriptor_copies)
                }
            });
        }
    }

    return Ok(Engine {
        instance,
        device,
        resource_manager,
        assets_manager,
        surface,
        swapchain,
        frame_buffers,
        command_buffers,
        renderpass,
        viewports,
        scissors,
        pipelines,
        pipeline_layout,
        descriptor_pool,
        global_descriptor_sets,
        material_descriptor_set_layout,
        texture_sampler,
        ubo_buffer,
        object_id_lookup_images,
        object_id_lookup_images_views,
        object_id_lookup_buffer,
        image_available_semaphore,
        render_finished_semaphore,
        command_buffers_completed_fence,
        timings_query_pool,
        vertices_query_pool,
        last_timing_value: [0],
        last_swapchain_image_index: 0,
    });
}

pub fn create_geometry(
    engine: &mut Engine,
    indices: &Vec<u32>,
    vertices: &Vec<Vertex>,
    texture_data: &Vec<u8>,
    texture_extent: Extent,
    instances: &Vec<InstanceData>,
) -> Geometry {

    let _device = (*engine.device).borrow();
    let mut resource_manager = &mut engine.resource_manager;

    let index_buffer = {

        let size: usize = (indices.len() * std::mem::size_of::<u32>());

        let mut buffer = resource_manager.create_buffer(String::from("geometry-index-buffer"), &BufferAllocationDescriptor {
            usage: [BufferUsage::IndexBuffer],
            memory: MemoryLocation::CpuToGpu
        }, indices.len()).expect("geometry index buffer");

        unsafe {
            resource_manager.copy(&indices, &mut buffer, 0, indices.len());
            resource_manager.flush(&mut buffer, 0, indices.len());
        }

        buffer
    };

    let vertex_buffer = {

        let size: usize = (vertices.len() * std::mem::size_of::<Vertex>());

        let mut buffer = resource_manager.create_buffer(String::from("geometry-vertex-buffer"), &BufferAllocationDescriptor {
            usage: [BufferUsage::VertexBuffer],
            memory: MemoryLocation::CpuToGpu
        }, vertices.len()).expect("geometry vertex buffer");

        unsafe {
            resource_manager.copy(&vertices, &mut buffer, 0, vertices.len());
            resource_manager.flush(&mut buffer, 0, vertices.len());
        }

        buffer
    };

    let texture_transfere_buffer = {

        let size: usize = texture_data.len() * std::mem::size_of::<u8>();

        let mut buffer = resource_manager.create_buffer(String::from("texture-transfer-buffer"), &BufferAllocationDescriptor {
            usage: [BufferUsage::TransferSourceBuffer],
            memory: MemoryLocation::CpuToGpu,
        }, size).expect("Failed to create texture data transfer buffer");

        unsafe {
            resource_manager.copy(&texture_data, &mut buffer, 0, texture_data.len());
            resource_manager.flush(&mut buffer, 0, texture_data.len());
        }

        buffer
    };

    let texture_image: Image = {

        resource_manager.create_image(String::from("texture-image"), &ImageAllocationDescriptor {
            usage: [ImageUsage::Sampled, ImageUsage::TransferDestination],
            extent: texture_extent,
            format: ImageFormat::RGBA8,
            tiling: ImageTiling::Optimal,
            memory: MemoryLocation::GpuOnly
        }).expect("Failed to create texture image")
    };

    let texture_image_view = {
        let image_view_create_info = ::ash::vk::ImageViewCreateInfo::builder()
            .image(*texture_image.handle())
            .view_type(::ash::vk::ImageViewType::TYPE_2D)
            .format(::ash::vk::Format::R8G8B8A8_UNORM)
            .subresource_range(::ash::vk::ImageSubresourceRange::builder()
                .aspect_mask(::ash::vk::ImageAspectFlags::COLOR)
                .base_mip_level(0)
                .level_count(1)
                .base_array_layer(0)
                .layer_count(1)
                .build()
            );

        unsafe {
            _device.handle().create_image_view(&image_view_create_info, None)
                .expect("Failed to create image view")
        }
    };

    let instances_buffer = {

        let size: usize = (instances.len() * std::mem::size_of::<InstanceData>());

        let mut buffer = resource_manager.create_buffer(String::from("geometry-instance-data-buffer"), &BufferAllocationDescriptor {
            usage: [BufferUsage::VertexBuffer],
            memory: MemoryLocation::CpuToGpu
        }, instances.len()).expect("geometry instance data buffer");

        unsafe {
            resource_manager.copy(&instances, &mut buffer, 0, instances.len());
            resource_manager.flush(&mut buffer, 0, instances.len());
        }

        buffer
    };

    let descriptor_set = {
        let pool = engine.descriptor_pool;
        let descriptor_set_layouts = [
            engine.material_descriptor_set_layout
        ];
        let create_info = ash::vk::DescriptorSetAllocateInfo::builder()
            .descriptor_pool(pool)
            .set_layouts(&descriptor_set_layouts);
        unsafe {
            _device.handle().allocate_descriptor_sets(&create_info)
                .expect("Failed to allocate descriptor set")[0]
        }
    };

    {
        let sampler = engine.texture_sampler;

        let descriptor_image_info = [
            ::ash::vk::DescriptorImageInfo::builder()
                .sampler(sampler)
                .image_layout(::ash::vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL)
                .image_view(texture_image_view)
                .build()
        ];

        let descriptor_writes = [
            ::ash::vk::WriteDescriptorSet::builder()
                .descriptor_type(::ash::vk::DescriptorType::COMBINED_IMAGE_SAMPLER)
                .dst_set(descriptor_set)
                .dst_binding(0)
                .image_info(&descriptor_image_info)
                .build()
        ];

        let descriptor_copies: [ash::vk::CopyDescriptorSet; 0] = [];

        unsafe {
            _device.handle().update_descriptor_sets(&descriptor_writes, &descriptor_copies)
        }
    }

    Geometry {
        index_buffer: index_buffer,
        vertex_buffer: vertex_buffer,
        instances_buffer: instances_buffer,
        material: Material {
            descriptor_set: descriptor_set,
            texture_buffer: texture_transfere_buffer,
            texture_image: texture_image,
            texture_image_view: texture_image_view
        },
    }
}

pub fn update_geometry(
    engine: &mut Engine,
    geometry: &mut Geometry,
    instances: &Vec<InstanceData>,
) {
    let mut resource_manager = &mut engine.resource_manager;
    let mut buffer = &mut geometry.instances_buffer;
    unsafe {
        resource_manager.copy(&instances, buffer, 0, instances.len());
        resource_manager.flush(buffer, 0, instances.len());
    }
}

pub fn pick_object(engine: &mut Engine, x: i32, y: i32) -> Option<u32> {

    let _device = (*engine.device).borrow();
    let queue = Rc::clone(&_device.queues()[0]);
    let mut resource_manager = &engine.resource_manager;
    let mut image = &mut engine.object_id_lookup_images[engine.last_swapchain_image_index as usize];
    let mut buffer = &mut engine.object_id_lookup_buffer;
    let mut dst = vec![0u32; buffer.capacity()];

    let command_buffer = {
        let create_info = ::ash::vk::CommandBufferAllocateInfo::builder()
            .command_pool(_device.command_pool().handle())
            .command_buffer_count(1)
            .level(ash::vk::CommandBufferLevel::PRIMARY)
            .build();

        unsafe {
            _device.handle().allocate_command_buffers(&create_info)
                .expect("Failed to create command buffer")[0]
        }
    };

    let completion_fence = {
        let fence_create_info = ::ash::vk::FenceCreateInfo::builder()
            .build();
        unsafe {
            _device.handle().create_fence(&fence_create_info, None)
                .expect("Expected successfull fence creation!")
        }
    };

    let begin_info = ash::vk::CommandBufferBeginInfo::builder()
        .flags(ash::vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT);

    unsafe {
        _device.handle().begin_command_buffer(command_buffer, &begin_info)
            .expect("Failed to begin command buffer");
    }

    // let barrier = ::ash::vk::ImageMemoryBarrier::builder()
    //     .src_queue_family_index(0)
    //     .dst_queue_family_index(0)
    //     .image(*image.handle())
    //     .old_layout(::ash::vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL)
    //     .new_layout(::ash::vk::ImageLayout::TRANSFER_SRC_OPTIMAL)
    //     .subresource_range(::ash::vk::ImageSubresourceRange::builder()
    //         .aspect_mask(::ash::vk::ImageAspectFlags::COLOR)
    //         .base_mip_level(0)
    //         .level_count(1)
    //         .base_array_layer(0)
    //         .layer_count(1)
    //         .build())
    //     .src_access_mask(::ash::vk::AccessFlags::NONE)
    //     .dst_access_mask(::ash::vk::AccessFlags::HOST_READ);
    //
    // unsafe {
    //     _device.handle().cmd_pipeline_barrier(
    //         command_buffer,
    //         ::ash::vk::PipelineStageFlags::FRAGMENT_SHADER,
    //         ::ash::vk::PipelineStageFlags::HOST,
    //         ::ash::vk::DependencyFlags::empty(),
    //         &[],
    //         &[],
    //         &[*barrier],
    //     )
    // }

    let image_to_buffer_copy = ::ash::vk::CopyImageToBufferInfo2::builder()
        .src_image(*image.handle())
        .src_image_layout(::ash::vk::ImageLayout::TRANSFER_SRC_OPTIMAL)
        .dst_buffer(*buffer.handle())
        .regions(&[
            ::ash::vk::BufferImageCopy2::builder()
                .buffer_offset(0)
                .buffer_row_length(image.extent().width)
                .buffer_image_height(image.extent().height)
                .image_offset(::ash::vk::Offset3D { x: 0, y: 0, z: 0})
                .image_extent(::ash::vk::Extent3D { width: image.extent().width, height: image.extent().height, depth: image.extent().depth })
                .image_subresource(::ash::vk::ImageSubresourceLayers::builder()
                   .aspect_mask(::ash::vk::ImageAspectFlags::COLOR)
                   .base_array_layer(0)
                   .layer_count(1)
                   .mip_level(0)
                   .build())
                .build()
        ])
        .build();

    unsafe {
        _device.handle().cmd_copy_image_to_buffer2(command_buffer, &image_to_buffer_copy)
    }

    // let barrier = ::ash::vk::BufferMemoryBarrier::builder()
    //     .buffer(*buffer.handle())
    //     .offset(0)
    //     .size((std::mem::size_of::<u32>() * buffer.capacity()) as u64)
    //     .src_access_mask(::ash::vk::AccessFlags::NONE)
    //     .dst_access_mask(::ash::vk::AccessFlags::HOST_READ)
    //     .src_queue_family_index(0)
    //     .dst_queue_family_index(0);
    //
    // unsafe {
    //     _device.handle().cmd_pipeline_barrier(
    //         command_buffer,
    //         ::ash::vk::PipelineStageFlags::BOTTOM_OF_PIPE,
    //         ::ash::vk::PipelineStageFlags::HOST,
    //         ::ash::vk::DependencyFlags::empty(),
    //         &[],
    //         &[*barrier],
    //         &[]
    //     )
    // }

    unsafe {
        _device.handle().end_command_buffer(command_buffer)
            .expect("Failed to end command buffer");
    }

    let command_buffers = [command_buffer];
    let submit_info = ash::vk::SubmitInfo::builder()
        .command_buffers(&command_buffers);

    unsafe {
        _device.handle().queue_submit(*queue.handle(), &[*submit_info], completion_fence)
            .expect("Failed to submit queue");
    }

    unsafe {
        let fences = [completion_fence];
        _device.handle().wait_for_fences(&fences, true, 5_000_000_000)
            .expect("Failed to wait for command buffer completion fence!");
    }

    unsafe {
        _device.handle().free_command_buffers(_device.command_pool().handle(), &command_buffers);
    }

    unsafe {
        resource_manager.flush(&buffer, 0, buffer.capacity());
        resource_manager.copy(buffer, &mut dst, 0, buffer.capacity());
    }

    let objectId = dst[(x as u32 + (y as u32 * image.extent().width)) as usize];

    if objectId > 0 {
        Some(objectId)
    }
    else {
        None
    }
}

pub fn prepare(engine: &mut Engine, world: &mut World) {

    let _device: Ref<Device> = (*engine.device).borrow();
    let queue = Rc::clone(&_device.queues()[0]);

    info!("Preparing resources.");

    let command_buffer = {
        let create_info = ::ash::vk::CommandBufferAllocateInfo::builder()
            .command_pool(_device.command_pool().handle())
            .command_buffer_count(1)
            .level(ash::vk::CommandBufferLevel::PRIMARY)
            .build();

        unsafe {
            _device.handle().allocate_command_buffers(&create_info)
                .expect("Failed to create command buffer")[0]
        }
    };

    let completion_fence = {
        let fence_create_info = ::ash::vk::FenceCreateInfo::builder()
            .build();

        unsafe {
            _device.handle().create_fence(&fence_create_info, None)
                .expect("Expected successfull fence creation!")
        }
    };

    unsafe {
        let begin_info = ash::vk::CommandBufferBeginInfo::builder()
            .flags(ash::vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT);

        _device.handle().begin_command_buffer(command_buffer, &begin_info)
            .expect("Failed to begin command buffer");
    }

    let range = ::ash::vk::ImageSubresourceRange::builder()
        .aspect_mask(::ash::vk::ImageAspectFlags::COLOR)
        .base_mip_level(0)
        .level_count(1)
        .base_array_layer(0)
        .layer_count(1)
        .build();

    fn layout_transition(
        device: &Ref<Device>,
        command_buffer: ::ash::vk::CommandBuffer,
        image: ::ash::vk::Image,
        range: ::ash::vk::ImageSubresourceRange,
        from: ::ash::vk::ImageLayout,
        to: ::ash::vk::ImageLayout,
    ) {

        let (source_stage, source_access_mask, destination_stage, destination_access_mask) =
            if from == ::ash::vk::ImageLayout::UNDEFINED && to == ::ash::vk::ImageLayout::TRANSFER_DST_OPTIMAL {
                (
                    ::ash::vk::PipelineStageFlags::TOP_OF_PIPE,
                    ::ash::vk::AccessFlags::NONE,
                    ::ash::vk::PipelineStageFlags::TRANSFER,
                    ::ash::vk::AccessFlags::TRANSFER_WRITE
                )
            }
            else if from == ::ash::vk::ImageLayout::TRANSFER_DST_OPTIMAL && to == ::ash::vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL {
                (
                    ::ash::vk::PipelineStageFlags::TRANSFER,
                    ::ash::vk::AccessFlags::TRANSFER_WRITE,
                    ::ash::vk::PipelineStageFlags::FRAGMENT_SHADER,
                    ::ash::vk::AccessFlags::SHADER_READ,
                )
            }
            else {
                todo!()
            };

        let barrier = ::ash::vk::ImageMemoryBarrier::builder()
            .image(image)
            .old_layout(from)
            .new_layout(to)
            .subresource_range(range)
            .src_access_mask(source_access_mask)
            .dst_access_mask(destination_access_mask);

        unsafe {
            device.handle().cmd_pipeline_barrier(
                command_buffer,
                source_stage,
                destination_stage,
                ::ash::vk::DependencyFlags::empty(),
                &[],
                &[],
                &[*barrier]
            )
        }
    }

    world.geometries.iter().for_each(|geometry| {

        let material = &geometry.material;

        layout_transition(
            _device.borrow(),
            command_buffer,
            *material.texture_image.handle(),
            range,
            ::ash::vk::ImageLayout::UNDEFINED,
            ::ash::vk::ImageLayout::TRANSFER_DST_OPTIMAL
        );

        let buffer_copy = ::ash::vk::BufferImageCopy::builder()
            .buffer_row_length(0)
            .buffer_image_height(0)
            .image_subresource(::ash::vk::ImageSubresourceLayers::builder()
                .aspect_mask(::ash::vk::ImageAspectFlags::COLOR)
                .mip_level(0)
                .base_array_layer(0)
                .layer_count(1)
                .build()
            )
            .image_offset(Offset3D { x: 0, y: 0, z: 0 })
            .image_extent(material.texture_image.extent().into());

        unsafe {
            _device.handle().cmd_copy_buffer_to_image(
                command_buffer,
                *material.texture_buffer.handle(),
                *material.texture_image.handle(),
                ::ash::vk::ImageLayout::TRANSFER_DST_OPTIMAL,
                &[*buffer_copy]
            )
        }

        layout_transition(
            _device.borrow(),
            command_buffer,
            *material.texture_image.handle(),
            range,
            ::ash::vk::ImageLayout::TRANSFER_DST_OPTIMAL,
            ::ash::vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL
        );
    });

    unsafe {
        _device.handle().end_command_buffer(command_buffer)
            .expect("Failed to end command buffer");
    }

    let command_buffers = [command_buffer];
    let submit_info = ash::vk::SubmitInfo::builder()
        .command_buffers(&command_buffers);

    unsafe {
        _device.handle().queue_submit(*queue.handle(), &[*submit_info], completion_fence)
            .expect("Failed to submit queue");
    }

    unsafe {
        let fences = [completion_fence];
        _device.handle().wait_for_fences(&fences, true, 5_000_000_000)
            .expect("Failed to wait for command buffer completion fence!");
    }

    unsafe {
        _device.handle().free_command_buffers(_device.command_pool().handle(), &command_buffers);
    }

    info!("Resources prepared.");
}

pub fn render(engine: &mut Engine, world: &mut World, camera: &Camera) {

    let _device = (*engine.device).borrow();
    let (index, suboptimal) = engine.swapchain.acquire_next_image(engine.image_available_semaphore);
    let queue = Rc::clone(&_device.queues()[0]);
    let command_buffer = [engine.command_buffers[index as usize]];
    let swapchains = [*engine.swapchain.handle()];
    let indices = [index];
    let mut resource_manager = &mut engine.resource_manager;

    update_ubo(
        index as usize,
        engine.device.clone(),
        &mut resource_manager,
        &mut engine.ubo_buffer,
        camera,
    );

    record_commands(
        engine.device.clone(),
        &engine.command_buffers[index as usize],
        &engine.global_descriptor_sets[index as usize],
        &engine.frame_buffers[index as usize],
        &engine.renderpass,
        &engine.viewports[0],
        &engine.scissors[0],
        &engine.pipelines,
        &engine.pipeline_layout,
        &engine.timings_query_pool,
        &engine.vertices_query_pool,
        &world.geometries,
    );

    let wait_semaphores = [
        engine.image_available_semaphore
    ];

    let signal_semaphores = [
        engine.render_finished_semaphore
    ];

    let submit_info = ash::vk::SubmitInfo::builder()
        .wait_semaphores(&wait_semaphores)
        .wait_dst_stage_mask(&[ash::vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT])
        .command_buffers(&command_buffer)
        .signal_semaphores(&signal_semaphores);

    unsafe {
        let fences = [engine.command_buffers_completed_fence];
        _device.handle().reset_fences(&fences)
            .expect("Failed to reset command buffers completed fence.");
    }

    unsafe {
        _device.handle().queue_submit(*queue.handle(), &[*submit_info], engine.command_buffers_completed_fence)
    }.expect("Failed to submit queue");

    let present_info = ash::vk::PresentInfoKHR::builder()
        .wait_semaphores(&signal_semaphores)
        .swapchains(&swapchains)
        .image_indices(&indices);

    engine.swapchain.queue_present(queue, &present_info);

    unsafe {
        let fences = [engine.command_buffers_completed_fence];

        _device.handle().wait_for_fences(&fences, true, 5_000_000_000)
            .expect("Failed to wait for command buffers completed fence!");

        _device.handle().device_wait_idle()
            .expect("Failed to wait for idle device");
    }

    let mut timing_data: [u64; 2] = [0, 0];
    let mut vertices_data: [u64; 1] = [0];

    unsafe {
        _device.handle().get_query_pool_results(engine.timings_query_pool, 0, 2, &mut timing_data, ash::vk::QueryResultFlags::WAIT)
            .expect("Failed to query pool results of the timings query pool");
        _device.handle().get_query_pool_results(engine.vertices_query_pool, 0, 1, &mut vertices_data, ash::vk::QueryResultFlags::WAIT)
            .expect("Failed to query pool results of the vertices query pool");
    }

    // let diff = Duration::nanoseconds((timing_data[1] - timing_data[0]) as i64);

    // println!("draw time: {} ns", timing_data[1] - timing_data[0]);
    // println!("vert. invocations: {}", vertices_data[0]);
    engine.last_swapchain_image_index = index
}

fn update_ubo(index: usize, device: DeviceRef, resource_manager: &mut ResourceManager, buffer: &mut Buffer<UniformBufferObject>, camera: &Camera) {

    let size = std::mem::size_of::<UniformBufferObject>();
    let mvp: Matrix4<f32> = *camera.as_matrix();

    let ubo = [
        UniformBufferObject {
            mvp: mvp,
        }
    ];

    unsafe {
        resource_manager.copy(&ubo, buffer, index, 1);
        resource_manager.flush(buffer, index, 1);
    }
}

fn record_commands(
    device: DeviceRef,
    command_buffer: &ash::vk::CommandBuffer,
    descriptor_set: &ash::vk::DescriptorSet,
    frame_buffer: &ash::vk::Framebuffer,
    renderpass: &ash::vk::RenderPass,
    viewport: &ash::vk::Viewport,
    scissor: &ash::vk::Rect2D,
    pipelines: &Vec<ash::vk::Pipeline>,
    pipeline_layout: &ash::vk::PipelineLayout,
    timings_query_pool: &ash::vk::QueryPool,
    vertices_query_pool: &ash::vk::QueryPool,
    geometries: &Vec<Geometry>,
) {

    let _device = (*device).borrow();

    unsafe {
        _device.handle().reset_command_buffer(*command_buffer, CommandBufferResetFlags::RELEASE_RESOURCES);
    }

    let begin_info = ash::vk::CommandBufferBeginInfo::builder()
        .flags(ash::vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT);

    unsafe { _device.handle().begin_command_buffer(*command_buffer, &begin_info) }
        .expect("Begin command buffer");

    unsafe {
        _device.handle().cmd_reset_query_pool(*command_buffer, *timings_query_pool, 0, 2);
        _device.handle().cmd_reset_query_pool(*command_buffer, *vertices_query_pool, 0, 1);
    }

    let renderpass_begin_info = ash::vk::RenderPassBeginInfo::builder()
        .render_pass(*renderpass)
        .framebuffer(*frame_buffer)
        .render_area(*scissor)
        .clear_values(&[
            ash::vk::ClearValue {
                color: ash::vk::ClearColorValue { float32: [0.1, 0.1, 0.1, 0.0] }
            },
            ash::vk::ClearValue {
                depth_stencil: ash::vk::ClearDepthStencilValue { depth: 1.0, stencil: 0 }
            },
            ash::vk::ClearValue {
                color: ash::vk::ClearColorValue { uint32: [0, 0, 0, 0] }
            }
        ]);

    unsafe {
        _device.handle().cmd_write_timestamp(*command_buffer, ash::vk::PipelineStageFlags::VERTEX_SHADER, *timings_query_pool, 0)
    }

    unsafe {
        _device.handle().cmd_begin_query(*command_buffer, *vertices_query_pool, 0, ash::vk::QueryControlFlags::empty())
    }

    let viewports = [*viewport];
    unsafe {
        _device.handle().cmd_set_viewport(*command_buffer, 0, &viewports);
    }

    let scissors = [*scissor];
    unsafe {
        _device.handle().cmd_set_scissor(*command_buffer, 0, &scissors);
    }

    unsafe {
        _device.handle().cmd_begin_render_pass(*command_buffer, &renderpass_begin_info, ash::vk::SubpassContents::INLINE);
    }

    pipelines.iter().for_each(| pipeline | {

        unsafe {
            _device.handle().cmd_bind_pipeline(*command_buffer, ash::vk::PipelineBindPoint::GRAPHICS, *pipeline);
        }

        geometries.iter().for_each(|geometry| {

            let vertex_buffers = [*geometry.vertex_buffer.handle()];
            let instance_data_buffers = [*geometry.instances_buffer.handle()];
            let buffer_offsets: [u64; 1] = [0];
            let descriptor_sets = [
                *descriptor_set,
                geometry.material.descriptor_set
            ];
            let descriptor_sets_offsets = [];

            unsafe {
                _device.handle().cmd_bind_vertex_buffers(*command_buffer, 0, &vertex_buffers, &buffer_offsets);
                _device.handle().cmd_bind_vertex_buffers(*command_buffer, 1, &instance_data_buffers, &buffer_offsets);
                _device.handle().cmd_bind_index_buffer(*command_buffer, *geometry.index_buffer.handle(), 0, ::ash::vk::IndexType::UINT32);
                _device.handle().cmd_bind_descriptor_sets(*command_buffer, ash::vk::PipelineBindPoint::GRAPHICS, *pipeline_layout, 0, &descriptor_sets, &descriptor_sets_offsets)
            }

            unsafe {
                _device.handle().cmd_draw_indexed(
                    *command_buffer,
                    geometry.index_buffer.capacity() as u32,
                    geometry.instances_buffer.capacity() as u32,
                    0,
                    0,
                    0,
                );
            }
        });
    });
    unsafe {
        _device.handle().cmd_end_render_pass(*command_buffer);
    }

    unsafe {
        _device.handle().cmd_end_query(*command_buffer, *vertices_query_pool, 0)
    }

    unsafe {
        _device.handle().cmd_write_timestamp(*command_buffer, ash::vk::PipelineStageFlags::VERTEX_SHADER, *timings_query_pool, 1)
    }

    unsafe {
        _device.handle().end_command_buffer(*command_buffer);
    }

}
