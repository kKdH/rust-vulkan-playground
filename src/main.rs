mod shaders;
mod camera;

use vulkano::buffer::{BufferUsage, CpuAccessibleBuffer, CpuBufferPool};
use vulkano::command_buffer::{AutoCommandBufferBuilder, DynamicState, AutoCommandBuffer};
use vulkano::device::{Device, DeviceExtensions};
use vulkano::framebuffer::{Framebuffer, FramebufferAbstract, RenderPassAbstract, Subpass};
use vulkano::image::{ImageUsage, SwapchainImage, ImmutableImage, Dimensions};
use vulkano::instance::{Instance, PhysicalDevice};
use vulkano::pipeline::viewport::Viewport;
use vulkano::pipeline::GraphicsPipeline;
use vulkano::swapchain;
use vulkano::swapchain::{
    AcquireError, ColorSpace, FullscreenExclusive, PresentMode, SurfaceTransform, Swapchain,
    SwapchainCreationError,
};
use vulkano::sync;
use vulkano::sync::{FlushError, GpuFuture};
use vulkano::descriptor::descriptor_set::PersistentDescriptorSet;
use vulkano_win::VkSurfaceBuild;
use winit::event::{Event, WindowEvent};
use winit::event_loop::{ControlFlow, EventLoop};
use winit::window::{Window, WindowBuilder};

use std::sync::Arc;

use vulkano::instance::InstanceExtensions;
use vulkano::instance;
use vulkano::pipeline::vertex::{OneVertexOneInstanceDefinition};
use vulkano::format::Format;
use vulkano::sampler::{Sampler, Filter, MipmapMode, SamplerAddressMode};
use std::io::Cursor;
use cgmath::{Rad, Matrix4, Point3, Vector3, Matrix3};
use log::{info, LevelFilter};
use log4rs::append::console::ConsoleAppender;
use log4rs::append::file::FileAppender;
use log4rs::encode::pattern::PatternEncoder;
use log4rs::config::{Config, Appender, Logger, Root};
use std::f32::consts::FRAC_PI_2;

#[derive(Default, Debug, Clone)]
struct Vertex {
    position: [f32; 3],
    tex_coord: [f32; 2]
}
vulkano::impl_vertex!(Vertex, position, tex_coord);

#[derive(Default, Debug, Clone)]
struct InstanceData {
    position_offset: [f32; 3],
    scale: f32,
}
vulkano::impl_vertex!(InstanceData, position_offset, scale);

fn main() {

    let stdout = ConsoleAppender::builder()
        .encoder(Box::new(PatternEncoder::new("{d} - {m}{n}")))
        .build();

    let config = Config::builder()
        .appender(Appender::builder().build("stdout", Box::new(stdout)))
        .build(Root::builder().appender("stdout").build(LevelFilter::Warn))
        .unwrap();

    let handle = log4rs::init_config(config).unwrap();

    let extensions = InstanceExtensions {
        ext_debug_utils: true,
        ..
        vulkano_win::required_extensions()
    };

    let layers = vec![
        "VK_LAYER_KHRONOS_validation",
    ];

    println!("Available layers:");
    for layer in instance::layers_list().unwrap() {
        println!("\t{}", layer.name());
    }

    let instance = Instance::new(None, &extensions, layers)
        .expect("failed to create Vulkan instance");

    let physical_device = PhysicalDevice::enumerate(&instance).next().unwrap();

    println!(
        "Using device: {} (type: {:?})",
        physical_device.name(),
        physical_device.ty()
    );

    let event_loop = EventLoop::new();
    let surface = WindowBuilder::new()
        .build_vk_surface(&event_loop, instance.clone())
        .unwrap();

    let queue_family = physical_device
        .queue_families()
        .find(|&queue| {
            queue.supports_graphics() && surface.is_supported(queue).unwrap_or(false)
        })
        .unwrap();

    let device_extensions = DeviceExtensions {
        khr_swapchain: true,
        ..
        DeviceExtensions::none()
    };

    let (device, mut queues) = Device::new(
        physical_device,
        physical_device.supported_features(),
        &device_extensions,
        [(queue_family, 0.5)].iter().cloned(),
    )
    .unwrap();

    let queue = queues.next().unwrap();

    let (mut swapchain, images) = {
        let capabilities = surface.capabilities(physical_device).unwrap();
        let alpha = capabilities.supported_composite_alpha.iter().next().unwrap();
        let format = capabilities.supported_formats[0].0;
        let dimensions: [u32; 2] = surface.window().inner_size().into();
        Swapchain::new(
            device.clone(),
            surface.clone(),
            capabilities.min_image_count,
            format,
            dimensions,
            1,
            ImageUsage::color_attachment(),
            &queue,
            SurfaceTransform::Identity,
            alpha,
            PresentMode::Fifo,
            FullscreenExclusive::Default,
            true,
            ColorSpace::SrgbNonLinear,
        )
        .unwrap()
    };

    let vertex_buffer = {

        CpuAccessibleBuffer::from_iter(
            device.clone(),
            BufferUsage::all(),
            false,
            [
                Vertex {
                    position: [-0.75, 0.75, 0.0],
                    tex_coord: [0.0, 1.0]
                },
                Vertex {
                    position: [-0.75, -0.75, 0.0],
                    tex_coord: [0.0, 0.0]
                },
                Vertex {
                    position: [0.75, -0.75, 0.0],
                    tex_coord: [1.0, 0.0]
                },
                Vertex {
                    position: [0.75, 0.75, 0.0],
                    tex_coord: [1.0, 1.0]
                },
            ]
                .iter()
                .cloned(),
        )
        .unwrap()
    };

    let indices: [u16; 6] = [0, 1, 2, 2, 3, 0];
    let index_buffer = {
        CpuAccessibleBuffer::from_iter(
            device.clone(),
            BufferUsage::all(),
            false,
            indices.iter().cloned()
        ).unwrap()
    };

    let instance_buffer = {
        let rows = 5;
        let cols = 5;
        let n_instances = rows * cols;
        let mut data = Vec::new();
        for c in 0..cols - 1 {
            for r in 0..rows - 1 {
                let half_cell_w = 0.5 / cols as f32;
                let half_cell_h = 0.5 / rows as f32;
                let x = half_cell_w + (c as f32 / cols as f32) * 2.0 - 1.0;
                let y = half_cell_h + (r as f32 / rows as f32) * 2.0 - 1.0;
                let position_offset = [x, y, 0.0];
                let scale = (2.0 / rows as f32) * (c * rows + r) as f32 / n_instances as f32;
                data.push(InstanceData {
                    position_offset,
                    scale,
                });
            }
        }
        CpuAccessibleBuffer::from_iter(
            device.clone(),
            BufferUsage::all(),
            false,
            data.iter().cloned(),
        )
        .unwrap()
    };

    let dimensions: [u32; 2] = surface.window().inner_size().into();

    let camera = camera::builder()
        .with_near_plane(0.01)
        .with_far_plane(100.0)
        .with_aspect_ratio_of(dimensions[0] as f32, dimensions[1] as f32)
        .with_field_of_view(FRAC_PI_2)
        .move_to(Point3::new(0.5, 0.5, 1.5))
        .look_at(Point3::new(0.0, 0.0, 0.0))
        .build();

    let uniform_buffer = CpuBufferPool::<shaders::vs::ty::Data>::new(device.clone(), BufferUsage::all());
    let model_buffer = CpuBufferPool::<shaders::vs::ty::Model>::new(device.clone(), BufferUsage::all());

    let world = Matrix4::from(Matrix3::from_angle_y(Rad(1.0)));

    let model_sub_buffer = {
        let uniform_data = shaders::vs::ty::Model {
            translation: Matrix4::from_scale(0.0).into()
        };
        model_buffer.next(uniform_data).unwrap()
    };

    let vs = shaders::vs::Shader::load(device.clone()).unwrap();
    let fs = shaders::fs::Shader::load(device.clone()).unwrap();

    let render_pass = Arc::new(
        vulkano::single_pass_renderpass!(
            device.clone(),
            attachments: {
                color: {
                    load: Clear,
                    store: Store,
                    format: swapchain.format(),
                    samples: 1,
                }
            },
            pass: {
                color: [color],
                depth_stencil: {}
            }
        )
        .unwrap(),
    );

    let (texture, mut tex_future) = {
        let png_bytes = include_bytes!("texture.png").to_vec();
        let cursor = Cursor::new(png_bytes);
        let decoder = png::Decoder::new(cursor);
        let (info, mut reader) = decoder.read_info().unwrap();
        let dimensions = Dimensions::Dim2d {
            width: info.width,
            height: info.height,
        };
        let mut image_data = Vec::new();
        image_data.resize((info.width * info.height * 4) as usize, 0);
        reader.next_frame(&mut image_data).unwrap();

        ImmutableImage::from_iter(
            image_data.iter().cloned(),
            dimensions,
            Format::R8G8B8A8Srgb,
            queue.clone(),
        )
            .unwrap()
    };

    let sampler = Sampler::new(
        device.clone(),
        Filter::Linear,
        Filter::Linear,
        MipmapMode::Nearest,
        SamplerAddressMode::Repeat,
        SamplerAddressMode::Repeat,
        SamplerAddressMode::Repeat,
        0.0,
        1.0,
        0.0,
        0.0,
    )
    .unwrap();

    let pipeline = Arc::new(
        GraphicsPipeline::start()
            .vertex_input(OneVertexOneInstanceDefinition::<Vertex, InstanceData>::new())
            .vertex_shader(vs.main_entry_point(), ())
            .triangle_list()
            .viewports_dynamic_scissors_irrelevant(1)
            .fragment_shader(fs.main_entry_point(), ())
            .render_pass(Subpass::from(render_pass.clone(), 0).unwrap())
            .build(device.clone())
            .unwrap(),
    );

    let mut dynamic_state = DynamicState {
        line_width: None,
        viewports: None,
        scissors: None,
        compare_mask: None,
        write_mask: None,
        reference: None,
    };

    let mut frame_buffers = window_size_dependent_setup(&images, render_pass.clone(), &mut dynamic_state);

    let mut recreate_swapchain = false;

    std::mem::drop(tex_future); // await image has been loaded!

    let mut previous_frame_end = Some(sync::now(device.clone()).boxed());

    let clear_values = vec![[0.2, 0.2, 0.2, 1.0].into()];

    event_loop.run(move |event, _, control_flow| {
        match event {
            Event::WindowEvent {
                event: WindowEvent::CloseRequested,
                ..
            } => {
                *control_flow = ControlFlow::Exit;
                println!("Window Closed.");
            }
            Event::WindowEvent {
                event: WindowEvent::Resized(_),
                ..
            } => {
                recreate_swapchain = true;
            }
            Event::RedrawEventsCleared => {
                previous_frame_end.as_mut().unwrap().cleanup_finished();

                if recreate_swapchain {
                    let dimensions: [u32; 2] = surface.window().inner_size().into();
                    let (new_swapchain, new_images) =
                        match swapchain.recreate_with_dimensions(dimensions) {
                            Ok(r) => r,
                            Err(SwapchainCreationError::UnsupportedDimensions) => return,
                            Err(e) => panic!("Failed to recreate swapchain: {:?}", e),
                        };
                    swapchain = new_swapchain;
                    frame_buffers = window_size_dependent_setup(
                        &new_images,
                        render_pass.clone(),
                        &mut dynamic_state,
                    );
                    recreate_swapchain = false;
                }

                let (image_num, suboptimal, acquire_future) =
                    match swapchain::acquire_next_image(swapchain.clone(), None) {
                        Ok(r) => r,
                        Err(AcquireError::OutOfDate) => {
                            recreate_swapchain = true;
                            return;
                        }
                        Err(e) => panic!("Failed to acquire next image: {:?}", e),
                    };

                if suboptimal {
                    recreate_swapchain = true
                }

                let model_rotation = Matrix4::from_scale(1.0);
                let model1: Matrix4<f32> = Matrix4::from_translation(Vector3::new(0.0, 0.0, 0.5)) * model_rotation;
                let model2: Matrix4<f32> = Matrix4::from_translation(Vector3::new(0.0, 0.0, -0.5)) * model_rotation;

                let mut sets = Vec::new();
                for model in [model1, model2].iter() {

                    let uniform_sub_buffer = {
                        let uniform_data = shaders::vs::ty::Data {
                            mvp: (camera.projection * camera.view * world * model).into()
                        };
                        uniform_buffer.next(uniform_data).unwrap()
                    };

                    let layout = pipeline.layout().descriptor_set_layout(0).unwrap();
                    let set = Arc::new(
                        PersistentDescriptorSet::start(layout.clone())
                            .add_buffer(uniform_sub_buffer)
                            .unwrap()
                            .add_sampled_image(texture.clone(), sampler.clone())
                            .unwrap()
                            .build()
                            .unwrap(),
                    );

                    sets.push(set);
                }

                let mut builder = AutoCommandBufferBuilder::primary_one_time_submit(
                    device.clone(),
                    queue.family(),
                )
                .unwrap();

                builder
                    .begin_render_pass(frame_buffers[image_num].clone(), false, clear_values.clone())
                    .unwrap()
                    .draw_indexed(
                        pipeline.clone(),
                        &dynamic_state,
                        (vertex_buffer.clone(), instance_buffer.clone()),
                        index_buffer.clone(),
                        sets.pop().unwrap().clone(),
                        ()
                    )
                    .unwrap()
                    .draw_indexed(
                        pipeline.clone(),
                        &dynamic_state,
                        (vertex_buffer.clone(), instance_buffer.clone()),
                        index_buffer.clone(),
                        sets.pop().unwrap().clone(),
                        ()
                    )
                    .unwrap()
                    .end_render_pass()
                    .unwrap();

                let command = builder.build().unwrap();

                let future = previous_frame_end
                    .take()
                    .unwrap()
                    .join(acquire_future)
                    .then_execute(queue.clone(), command)
                    .unwrap()
                    // .then_execute(queue.clone(), commands.pop().unwrap())
                    // .unwrap()
                    .then_swapchain_present(queue.clone(), swapchain.clone(), image_num)
                    .then_signal_fence_and_flush();

                match future {
                    Ok(future) => {
                        previous_frame_end = Some(future.boxed());
                    }
                    Err(FlushError::OutOfDate) => {
                        recreate_swapchain = true;
                        previous_frame_end = Some(sync::now(device.clone()).boxed());
                    }
                    Err(e) => {
                        println!("Failed to flush future: {:?}", e);
                        previous_frame_end = Some(sync::now(device.clone()).boxed());
                    }
                }
            }

            _ => (),
        }
    });

    println!("Window closed");
}

fn window_size_dependent_setup(
    images: &[Arc<SwapchainImage<Window>>],
    render_pass: Arc<dyn RenderPassAbstract + Send + Sync>,
    dynamic_state: &mut DynamicState,
) -> Vec<Arc<dyn FramebufferAbstract + Send + Sync>> {
    let dimensions = images[0].dimensions();

    let viewport = Viewport {
        origin: [0.0, 0.0],
        dimensions: [dimensions[0] as f32, dimensions[1] as f32],
        depth_range: 0.0..1.0,
    };
    dynamic_state.viewports = Some(vec![viewport]);

    images
        .iter()
        .map(|image| {
            Arc::new(
                Framebuffer::start(render_pass.clone())
                    .add(image.clone())
                    .unwrap()
                    .build()
                    .unwrap(),
            ) as Arc<dyn FramebufferAbstract + Send + Sync>
        })
        .collect::<Vec<_>>()
}
