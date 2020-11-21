use std::cell::{Cell, RefCell};
use std::convert::TryInto;
use std::ffi::{CStr, CString};
use std::ops::Deref;
use std::rc::Rc;

use ash::extensions::ext::DebugUtils;
use ash::extensions::khr;
use ash::version::{EntryV1_0, InstanceV1_0, DeviceV1_0};
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
use crate::graphics::vulkan::renderpass::create_render_pass;
use std::io::Cursor;
use ash::vk::{Rect2D, CommandBuffer, xcb_connection_t};
use std::borrow::Borrow;

#[derive(Debug)]
pub struct EngineError {
    message: String
}

pub struct Engine {
    instance: InstanceRef,
    device: DeviceRef,
    surface: SurfaceRef,
    swapchain: SwapchainRef,
    frame_buffers: Vec<ash::vk::Framebuffer>,
    command_buffers: Vec<ash::vk::CommandBuffer>,
    image_available_semaphore: ash::vk::Semaphore,
    render_finished_semaphore: ash::vk::Semaphore
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
            // String::from("VK_LAYER_LUNARG_api_dump"),
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
    let frame_buffers: Vec<ash::vk::Framebuffer>;
    let command_buffers: Vec<ash::vk::CommandBuffer>;
    let image_available_semaphore: ash::vk::Semaphore;
    let render_finished_semaphore: ash::vk::Semaphore;

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

        swapchain = {
            let _device = (*device).borrow();
            let queue = _device.queues().first().unwrap();
            Swapchain::new(
                Rc::clone(&device),
                Rc::clone(queue),
                Rc::clone(&surface)
            ).unwrap()
        };

        // let command_buffer = (*device).borrow_mut().allocate_command_buffer();

        let mut vertex_spv_file = Cursor::new(&include_bytes!("../vert.spv")[..]);
        let mut frag_spv_file = Cursor::new(&include_bytes!("../frag.spv")[..]);

        let vertex_code = ash::util::read_spv(&mut vertex_spv_file).expect("Failed to read vertex shader spv file");
        let vertex_shader_info = vk::ShaderModuleCreateInfo::builder().code(&vertex_code);

        let frag_code = ash::util::read_spv(&mut frag_spv_file).expect("Failed to read fragment shader spv file");
        let frag_shader_info = vk::ShaderModuleCreateInfo::builder().code(&frag_code);

        let _device = (*device).borrow();

        let vertex_shader_module = unsafe {
            _device.handle().create_shader_module(&vertex_shader_info, None)
        }.expect("Vertex shader module error");

        let fragment_shader_module = unsafe {
            _device.handle().create_shader_module(&frag_shader_info, None)
        }.expect("Fragment shader module error");

        let shader_entry_name = CString::new("main").unwrap();

        let shader_stage_create_infos = [
            ash::vk::PipelineShaderStageCreateInfo::builder()
                .stage(ash::vk::ShaderStageFlags::VERTEX)
                .module(vertex_shader_module)
                .name(shader_entry_name.as_c_str())
                .build(),
            ash::vk::PipelineShaderStageCreateInfo::builder()
                .stage(ash::vk::ShaderStageFlags::FRAGMENT)
                .module(fragment_shader_module)
                .name(shader_entry_name.as_c_str())
                .build(),
        ];

        let vertex_input_state_info = ash::vk::PipelineVertexInputStateCreateInfo {
            ..Default::default()
        };

        let vertex_input_assembly_state_info = ash::vk::PipelineInputAssemblyStateCreateInfo {
            topology: vk::PrimitiveTopology::TRIANGLE_LIST,
            primitive_restart_enable: ash::vk::FALSE,
            ..Default::default()
        };

        let viewports = [vk::Viewport {
            x: 0.0,
            y: 0.0,
            width: window.inner_size().width as f32,
            height: window.inner_size().height as f32,
            min_depth: 0.0,
            max_depth: 1.0,
        }];

        let scissors = [ash::vk::Rect2D {
            offset: ash::vk::Offset2D { x: 0, y: 0 },
            extent: ash::vk::Extent2D {
                width: window.inner_size().width,
                height: window.inner_size().height
            },
        }];

        let viewport_state_info = vk::PipelineViewportStateCreateInfo::builder()
            .scissors(&scissors)
            .viewports(&viewports);

        let rasterization_info = ash::vk::PipelineRasterizationStateCreateInfo::builder()
            .depth_clamp_enable(false)
            .rasterizer_discard_enable(false)
            .polygon_mode(ash::vk::PolygonMode::FILL)
            .line_width(1.0)
            .cull_mode(ash::vk::CullModeFlags::BACK)
            .front_face(ash::vk::FrontFace::CLOCKWISE)
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

        let depth_state_info = vk::PipelineDepthStencilStateCreateInfo {
            depth_test_enable: 1,
            depth_write_enable: 1,
            depth_compare_op: vk::CompareOp::LESS_OR_EQUAL,
            front: noop_stencil_state,
            back: noop_stencil_state,
            max_depth_bounds: 1.0,
            ..Default::default()
        };

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

        let pipeline_layout_create_info = vk::PipelineLayoutCreateInfo::default();

        let pipeline_layout = unsafe {
            _device.handle().create_pipeline_layout(&pipeline_layout_create_info, None)
        }.unwrap();

        let renderpass = create_render_pass(device.clone(), surface.clone());

        let graphic_pipeline_info = vk::GraphicsPipelineCreateInfo::builder()
            .stages(&shader_stage_create_infos)
            .vertex_input_state(&vertex_input_state_info)
            .input_assembly_state(&vertex_input_assembly_state_info)
            .viewport_state(&viewport_state_info)
            .rasterization_state(&rasterization_info)
            .multisample_state(&multisample_state_info)
            // .depth_stencil_state(&depth_state_info)
            .color_blend_state(&color_blend_state)
            .dynamic_state(&dynamic_state_info)
            .layout(pipeline_layout)
            .render_pass(renderpass)
            .subpass(0);

        let graphics_pipelines = unsafe {
            _device.handle().create_graphics_pipelines(
                    vk::PipelineCache::null(),
                    &[graphic_pipeline_info.build()],
                    None,
                )
        }.expect("Unable to create graphics pipeline");

        let pipeline = graphics_pipelines[0];

        frame_buffers = (*swapchain).borrow().views().iter().map(|view| {

            let attachments = [*view];

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

        command_buffers = (0..(*swapchain).borrow().views().len()).map(|_| {

            let create_info = ash::vk::CommandBufferAllocateInfo::builder()
                .command_pool(_device.command_pool().handle())
                .command_buffer_count(1)
                .level(ash::vk::CommandBufferLevel::PRIMARY)
                .build();

            unsafe {
                _device.handle().allocate_command_buffers(&create_info)
            }.unwrap()[0]

        }).collect::<Vec<_>>();

        command_buffers.iter().enumerate().for_each(|(index, command_buffer)| {

            let begin_info = ash::vk::CommandBufferBeginInfo::builder()
                .flags(ash::vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT);

            unsafe { _device.handle().begin_command_buffer(*command_buffer, &begin_info) }
                .expect("Begin command buffer");

            let renderpass_begin_info = ash::vk::RenderPassBeginInfo::builder()
                .render_pass(renderpass)
                .framebuffer(frame_buffers[index])
                .render_area(Rect2D {
                    offset: ash::vk::Offset2D { x: 0, y: 0 },
                    extent: ash::vk::Extent2D {
                        width: window.inner_size().width,
                        height: window.inner_size().height,
                    }
                })
                .clear_values(&[
                    ash::vk::ClearValue {
                        color: ash::vk::ClearColorValue { float32: [0.1, 0.1, 0.1, 0.0] }
                    }
                ]);

            unsafe {
                _device.handle().cmd_begin_render_pass(*command_buffer, &renderpass_begin_info, ash::vk::SubpassContents::INLINE);
            }

            unsafe {
                _device.handle().cmd_bind_pipeline(*command_buffer, ash::vk::PipelineBindPoint::GRAPHICS, pipeline);
            }

            unsafe {
                _device.handle().cmd_set_viewport(*command_buffer, 0, &viewports);
            }

            unsafe {
                _device.handle().cmd_set_scissor(*command_buffer, 0, &scissors);
            }

            unsafe {
                _device.handle().cmd_draw(*command_buffer, 3, 1, 0, 0);
            }

            unsafe {
                _device.handle().cmd_end_render_pass(*command_buffer);
            }

            unsafe {
                _device.handle().end_command_buffer(*command_buffer);
            }
        });

        let semaphore_create_info = ash::vk::SemaphoreCreateInfo::builder()
            .flags(ash::vk::SemaphoreCreateFlags::default());

        image_available_semaphore = unsafe {
            _device.handle().create_semaphore(&semaphore_create_info, None)
        }.unwrap();

        render_finished_semaphore = unsafe {
            _device.handle().create_semaphore(&semaphore_create_info, None)
        }.unwrap();
    }

    return Ok(Engine {
        instance,
        device,
        surface,
        swapchain,
        frame_buffers,
        command_buffers,
        image_available_semaphore,
        render_finished_semaphore,
    });
}

pub fn render(engine: &Engine) {

    let _device = (*engine.device).borrow();
    let (index, suboptimal) = engine.swapchain.acquire_next_image(engine.image_available_semaphore);
    let queue = Rc::clone(&_device.queues()[0]);
    let command_buffer = [engine.command_buffers[index as usize]];
    let swapchains = [*engine.swapchain.handle()];
    let indices = [index];

    info!("index: {}", index);

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
        _device.handle().queue_submit(*queue.handle(), &[*submit_info], vk::Fence::null())
    }.unwrap();

    let present_info = ash::vk::PresentInfoKHR::builder()
        .wait_semaphores(&signal_semaphores)
        .swapchains(&swapchains)
        .image_indices(&indices);

    engine.swapchain.queue_present(queue, &present_info);

    unsafe {
        _device.handle().device_wait_idle();
    }
}
