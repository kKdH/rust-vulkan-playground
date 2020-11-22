use std::cell::{Cell, RefCell};
use std::convert::TryInto;
use std::ffi::{CStr, CString};
use std::ops::Deref;
use std::rc::Rc;

use ash::extensions::ext::DebugUtils;
use ash::extensions::khr;
use ash::version::{EntryV1_0, InstanceV1_0, DeviceV1_0, DeviceV1_2};
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
use ash::vk::{Rect2D, CommandBuffer, xcb_connection_t, DrawIndirectCommand};
use std::borrow::Borrow;
use chrono::Duration;

#[derive(Debug)]
pub struct EngineError {
    message: String
}

pub struct Engine {
    instance: InstanceRef,
    device: DeviceRef,
    surface: SurfaceRef,
    swapchain: SwapchainRef,
    renderpass: ash::vk::RenderPass,
    viewports: [ash::vk::Viewport; 1],
    scissors: [ash::vk::Rect2D; 1],
    pipeline: ash::vk::Pipeline,
    frame_buffers: Vec<ash::vk::Framebuffer>,
    command_buffers: Vec<ash::vk::CommandBuffer>,
    draw_indirect_command_buffer: (ash::vk::Buffer, vk_mem::Allocation),
    image_available_semaphore: ash::vk::Semaphore,
    render_finished_semaphore: ash::vk::Semaphore,
    timings_query_pool: ash::vk::QueryPool,
    vertices_query_pool: ash::vk::QueryPool,
    last_timing_value: [u64; 1]
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
    let renderpass: ash::vk::RenderPass;
    let viewports: [ash::vk::Viewport; 1];
    let scissors: [ash::vk::Rect2D; 1];
    let pipeline: ash::vk::Pipeline;
    let draw_indirect_command_buffer: (ash::vk::Buffer, vk_mem::Allocation);
    let image_available_semaphore: ash::vk::Semaphore;
    let render_finished_semaphore: ash::vk::Semaphore;
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

        swapchain = {
            let _device = (*device).borrow();
            let queue = _device.queues().first().unwrap();
            Swapchain::new(
                Rc::clone(&device),
                Rc::clone(queue),
                Rc::clone(&surface)
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

        renderpass = create_render_pass(device.clone(), surface.clone());

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

        pipeline = graphics_pipelines[0];

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

        let semaphore_create_info = ash::vk::SemaphoreCreateInfo::builder()
            .flags(ash::vk::SemaphoreCreateFlags::default());

        image_available_semaphore = unsafe {
            _device.handle().create_semaphore(&semaphore_create_info, None)
        }.unwrap();

        render_finished_semaphore = unsafe {
            _device.handle().create_semaphore(&semaphore_create_info, None)
        }.unwrap();

        {
            let allocation_create_info = vk_mem::AllocationCreateInfo {
                usage: vk_mem::MemoryUsage::GpuOnly,
                flags: vk_mem::AllocationCreateFlags::NONE,
                required_flags: ash::vk::MemoryPropertyFlags::HOST_VISIBLE,
                preferred_flags: Default::default(),
                memory_type_bits: 0,
                pool: None,
                user_data: None
            };

            let count = 1 as usize;

            let buffer_create_info = ash::vk::BufferCreateInfo::builder()
                .usage(ash::vk::BufferUsageFlags::INDIRECT_BUFFER)
                .sharing_mode(ash::vk::SharingMode::EXCLUSIVE)
                .size((count * std::mem::size_of::<ash::vk::DrawIndirectCommand>()) as u64);

            let (buffer, allocation, ai) = _device.allocator()
                .create_buffer(&buffer_create_info, &allocation_create_info)
                .expect("Allocation for 'draw_indirect_command_buffer' failed");

            let data =  _device.allocator()
                .map_memory(&allocation)
                .expect("Map memory for 'draw_indirect_command_buffer' failed") as *mut DrawIndirectCommand;

            unsafe {
                let data = std::ptr::slice_from_raw_parts_mut(data, count);
                (*data)[0] = DrawIndirectCommand {
                    vertex_count: 3,
                    instance_count: 1,
                    first_vertex: 0,
                    first_instance: 0
                };
            }

            _device.allocator().unmap_memory(&allocation);

            draw_indirect_command_buffer = (buffer, allocation)
        }
    }

    return Ok(Engine {
        instance,
        device,
        surface,
        swapchain,
        frame_buffers,
        command_buffers,
        renderpass,
        viewports,
        scissors,
        pipeline,
        draw_indirect_command_buffer,
        image_available_semaphore,
        render_finished_semaphore,
        timings_query_pool,
        vertices_query_pool,
        last_timing_value: [0]
    });
}

pub fn render(engine: &mut Engine) {

    let _device = (*engine.device).borrow();
    let (index, suboptimal) = engine.swapchain.acquire_next_image(engine.image_available_semaphore);
    let queue = Rc::clone(&_device.queues()[0]);
    let command_buffer = [engine.command_buffers[index as usize]];
    let swapchains = [*engine.swapchain.handle()];
    let indices = [index];

    info!("================");

    record_commands(
        engine.device.clone(),
        &engine.command_buffers[index as usize],
        &engine.draw_indirect_command_buffer.0,
        &engine.frame_buffers[index as usize],
        &engine.renderpass,
        &engine.viewports[0],
        &engine.scissors[0],
        &engine.pipeline,
        &engine.timings_query_pool,
        &engine.vertices_query_pool,
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

    let mut timing_data: [u64; 2] = [0, 0];
    let mut vertices_data: [u64; 1] = [0];

    unsafe {
        _device.handle().get_query_pool_results(engine.timings_query_pool, 0, 2, &mut timing_data, ash::vk::QueryResultFlags::WAIT);
        _device.handle().get_query_pool_results(engine.vertices_query_pool, 0, 1, &mut vertices_data, ash::vk::QueryResultFlags::WAIT);
    }

    let diff = Duration::nanoseconds((timing_data[1] - timing_data[0]) as i64);

    println!("draw time: {} ns", timing_data[1] - timing_data[0]);
    println!("vert. invocations: {}", vertices_data[0]);
}

fn record_commands(
    device: DeviceRef,
    command_buffer: &ash::vk::CommandBuffer,
    draw_indirect_command_buffer: &ash::vk::Buffer,
    frame_buffer: &ash::vk::Framebuffer,
    renderpass: &ash::vk::RenderPass,
    viewport: &ash::vk::Viewport,
    scissor: &ash::vk::Rect2D,
    pipeline: &ash::vk::Pipeline,
    timings_query_pool: &ash::vk::QueryPool,
    vertices_query_pool: &ash::vk::QueryPool,
) {

    let _device = (*device).borrow();

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
            }
        ]);

    unsafe {
        _device.handle().cmd_write_timestamp(*command_buffer, ash::vk::PipelineStageFlags::VERTEX_SHADER, *timings_query_pool, 0)
    }

    unsafe {
        _device.handle().cmd_begin_query(*command_buffer, *vertices_query_pool, 0, ash::vk::QueryControlFlags::empty())
    }

    unsafe {
        _device.handle().cmd_begin_render_pass(*command_buffer, &renderpass_begin_info, ash::vk::SubpassContents::INLINE);
    }

    unsafe {
        _device.handle().cmd_bind_pipeline(*command_buffer, ash::vk::PipelineBindPoint::GRAPHICS, *pipeline);
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
        _device.handle().cmd_draw_indirect(*command_buffer, *draw_indirect_command_buffer, 0, 1, 0);
    }

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
