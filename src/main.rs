mod blend;

extern crate ash;
extern crate skyshard;
extern crate winit;

use std::borrow::{Borrow, BorrowMut};
use std::cell::{Cell, RefCell};
use std::error::Error;
use std::f32::consts::FRAC_PI_2;
use std::ffi::{CStr, CString};
use std::io::Cursor;
use std::iter::Copied;
use std::ops::Deref;
use std::process::exit;
use std::rc::{Rc, Weak};
use std::slice::Iter;
use std::sync::Arc;
use std::time::{Duration, SystemTime, SystemTimeError};
use std::vec;

use ash::extensions::{
    ext::DebugUtils,
    khr::{Surface, Swapchain},
};
use ash::vk;
use ash::vk::{Extent2D, PhysicalDevice, Queue, SurfaceCapabilitiesKHR, SurfaceFormatKHR, SurfaceKHR};
use log::{debug, error, info, LevelFilter, warn};
use log4rs;
use log4rs::append::console::ConsoleAppender;
use log4rs::append::file::FileAppender;
use log4rs::config::{Appender, Config, Logger, Root};
use log4rs::encode::pattern::PatternEncoder;
use nalgebra::Matrix4;
use nalgebra::Vector3;
use skyshard::entity::{World};
use skyshard::graphics::{Camera, Extent, Position};
use skyshard::{InstanceData, Vertex};
use winit::dpi::PhysicalPosition;
use winit::event::{ElementState, Event, KeyboardInput, VirtualKeyCode, WindowEvent};
use winit::event_loop::{ControlFlow, EventLoop};
use winit::window::{Window, WindowBuilder};
use rand::Rng;


fn main() {

    let stdout = ConsoleAppender::builder()
        .encoder(Box::new(PatternEncoder::new("[{d(%Y-%m-%d %H:%M:%S.%3f%Z)(utc)}] [{h({l})}] {T} {M} - {m}\n")))
        .build();

    let fileout = FileAppender::builder()
        .encoder(Box::new(PatternEncoder::new("[{d(%Y-%m-%d %H:%M:%S.%3f%Z)(utc)}] [{h({l})}] {T} {M} - {m}\n")))
        .append(false)
        .build("log/application.log")
        .unwrap();

    let config = Config::builder()
        .appender(Appender::builder().build("stdout", Box::new(stdout)))
        .appender(Appender::builder().build("fileout", Box::new(fileout)))
        .build(Root::builder()
            .appender("stdout")
            .appender("fileout")
            .build(LevelFilter::Debug))
        .unwrap();

    // use handle to change logger configuration at runtime
    let handle = log4rs::init_config(config).unwrap();

    let mut events_loop = EventLoop::new();

    let window_width = 960;
    let window_height = 540;

    let window_title_prefix = "rust vulkan example: ";
    let window = WindowBuilder::new()
        .with_title(window_title_prefix)
        .with_inner_size(winit::dpi::LogicalSize::new(
            f64::from(window_width),
            f64::from(window_height),
        ))
        .build(&events_loop)
        .unwrap();

    let mut rng = rand::thread_rng();

    {
        let mut engine = skyshard::create("Rust Vulkan Example", &window).unwrap();
        let asset_manager = engine.asset_manager();
        let mut world = World::new();

        let (texture_extent, texture_data) = load_image("src/texture-small.png");
        let cube_node = asset_manager.load_node(&String::from("Cube")).expect("Failed to load cube");
        let cube_mesh = cube_node.mesh();
        let cube_vertices: Vec<Vertex> = cube_mesh.positions.iter().map(|index| {
            Vertex {
                position: *index,
                color: [
                    rng.gen_range(0f32..1f32),
                    rng.gen_range(0f32..1f32),
                    rng.gen_range(0f32..1f32)
                ],
                uv: [0.0, 0.0] //(&cube_mesh.texture_coordinates)[*index as usize],
            }
        }).collect();

        let cube = {

            let transformation1 = Matrix4::<f32>::identity()
                .append_translation(&Vector3::new(0.0, 0.0, 0.0));

            let mut transformation2 = Matrix4::<f32>::identity()
                .append_translation(&Vector3::new(2.5, 0.0, 0.0));

            transformation2 = transformation2 * Matrix4::<f32>::from_euler_angles(0.0, 0.5, 0.7);

            let mut  transformation3 = Matrix4::<f32>::identity()
                .append_translation(&Vector3::new(0.0, 3.0, 1.5));

            transformation3 = transformation3 * Matrix4::<f32>::from_euler_angles(0.5, 0.3, 0.5);

            skyshard::create_geometry(&mut engine,
                &cube_mesh.indices,
                &cube_vertices,
                &texture_data,
                texture_extent,
                &vec![
                    InstanceData {
                        transformation: transformation1.data
                            .as_slice()
                            .try_into()
                            .expect("slice with incorect length")
                    },
                    InstanceData {
                        transformation: transformation2.data
                            .as_slice()
                            .try_into()
                            .expect("slice with incorect length")
                    },
                    InstanceData {
                        transformation: transformation3.data
                            .as_slice()
                            .try_into()
                            .expect("slice with incorect length")
                    },
                ])
        };

        world.geometries.push(cube);

        // {
        //     let transformation1 = Matrix4::<f32>::identity()
        //         .append_translation(&Vector3::new(0.0, 0.0, 0.0));
        //
        //     let mut transformation2 = Matrix4::<f32>::identity()
        //         .append_translation(&Vector3::new(1.5, 0.0, 0.0));
        //
        //     transformation2 = transformation2 * Matrix4::<f32>::from_euler_angles(0.0, 0.5, 0.7);
        //
        //     let mut  transformation3 = Matrix4::<f32>::identity()
        //         .append_translation(&Vector3::new(0.0, 2.0, 1.5));
        //
        //     transformation3 = transformation3 * Matrix4::<f32>::from_euler_angles(0.5, 0.3, 0.5);
        //
        //     world.geometries.push(skyshard::create_geometry(&mut engine,
        //         &vec![
        //             0, 1, 2, 2, 3, 0, // front
        //             0, 3, 4, 5, 0, 4, // left
        //             1, 7, 6, 2, 1, 6, // right
        //             0, 5, 1, 1, 5, 7, // top
        //             2, 4, 3, 6, 4, 2, // bottom
        //             5, 4, 6, 6, 7, 5, // rear
        //         ],
        //         &vec![
        //             Vertex {
        //                 position: [-0.5, -0.5, 0.0], // front top-left
        //                 color: [1.0, 0.0, 0.0],
        //                 uv: [0.0, 0.0],
        //             },
        //             Vertex {
        //                 position: [0.5, -0.5, 0.0], // front top-right
        //                 color: [0.0, 1.0, 0.0],
        //                 uv: [1.0, 0.0],
        //             },
        //             Vertex {
        //                 position: [0.5, 0.5, 0.0], // front bottom-right
        //                 color: [0.0, 0.0, 1.0],
        //                 uv: [1.0, 1.0],
        //             },
        //             Vertex {
        //                 position: [-0.5, 0.5, 0.0], // front bottom-left
        //                 color: [1.0, 1.0, 1.0],
        //                 uv: [0.0, 1.0],
        //             },
        //             Vertex {
        //                 position: [-0.5, 0.5, 1.0], // rear bottom-left
        //                 color: [1.0, 0.0, 1.0],
        //                 uv: [0.0, 1.0],
        //             },
        //             Vertex {
        //                 position: [-0.5, -0.5, 1.0], // rear top-left
        //                 color: [1.0, 1.0, 0.0],
        //                 uv: [0.0, 0.0],
        //             },
        //             Vertex {
        //                 position: [0.5, 0.5, 1.0], // rear bottom-right
        //                 color: [1.0, 0.0, 0.0],
        //                 uv: [1.0, 1.0],
        //             },
        //             Vertex {
        //                 position: [0.5, -0.5, 1.0], // rear top-right
        //                 color: [0.0, 0.0, 1.0],
        //                 uv: [1.0, 0.0],
        //             },
        //         ],
        //         &texture_data,
        //         texture_extent,
        //         &vec![
        //             InstanceData {
        //                 transformation: transformation1.data
        //                     .as_slice()
        //                     .try_into()
        //                     .expect("slice with incorect length")
        //             },
        //             InstanceData {
        //                 transformation: transformation2.data
        //                     .as_slice()
        //                     .try_into()
        //                     .expect("slice with incorect length")
        //             },
        //             InstanceData {
        //                 transformation: transformation3.data
        //                     .as_slice()
        //                     .try_into()
        //                     .expect("slice with incorect length")
        //             },
        //         ])
        //     );
        // }

        {
            let mut transformation1 = Matrix4::<f32>::identity()
                .append_translation(&Vector3::new(-1.5, 0.0, 0.0));

            transformation1 = transformation1 * Matrix4::<f32>::from_euler_angles(0.25, -0.75, -0.0);

            world.geometries.push(skyshard::create_geometry(&mut engine,
                &vec![
                    2, 0, 1, // front
                    4, 3, 5, // back
                    2, 3, 0, 5, 3, 2, // right
                    1, 0, 3, 3, 4, 1, // left
                    4, 2, 1, 5, 2, 4, // bottom
                ],
                &vec![
                    Vertex {
                        position: [0.0, -0.5, 0.0], // front top
                        color: [1.0, 0.0, 0.0],
                        uv: [0.0, 0.0],
                    },
                    Vertex {
                        position: [-0.5, 0.5, 0.0], // front left
                        color: [0.0, 1.0, 0.0],
                        uv: [0.0, 0.0],
                    },
                    Vertex {
                        position: [0.5, 0.5, 0.0], // front right
                        color: [0.0, 0.0, 1.0],
                        uv: [0.0, 0.0],
                    },
                    Vertex {
                        position: [0.0, -0.5, 1.0], // rear top
                        color: [1.0, 0.0, 0.0],
                        uv: [0.0, 0.0],
                    },
                    Vertex {
                        position: [-0.5, 0.5, 1.0], // rear left
                        color: [0.0, 1.0, 0.0],
                        uv: [0.0, 0.0],
                    },
                    Vertex {
                        position: [0.5, 0.5, 1.0], // rear right
                        color: [0.0, 0.0, 1.0],
                        uv: [0.0, 0.0],
                    },
                ],
                &texture_data,
                texture_extent,
                &vec![
                    InstanceData {
                        transformation: transformation1.data
                            .as_slice()
                            .try_into()
                            .expect("slice with incorect length")
                    },
                ]
            ));
        }

        let mut redraw_requested = true;
        let mut close_requested = false;

        engine.reference_counts();

        let mut camera = Camera::new(
            window_width as f32 / window_height as f32,
            3.14 / 4.0,
            0.01,
            100.0
        );

        camera.eye(0f32, 0f32, 3f32);
        camera.update();

        let mut is_cursor_grabbed = false;
        let mut last_cursor_x = 0.0;
        let mut last_cursor_y = 0.0;
        let mut roll = 0.0;
        let mut pitch = 0.0;
        let mut yaw = 0.0;
        let mut render_time: SystemTime = SystemTime::now();
        let mut frames_per_second_time: SystemTime = SystemTime::now();
        let mut frame_count: u32 = 0;
        let mut frames_per_second: u32 = 0;

        skyshard::prepare(&mut engine, &mut world);

        info!("Starting event loop");

        skyshard::render(&mut engine, &mut world, &camera);

        events_loop.run(move |event, _, control_flow| {
            match event {
                Event::WindowEvent { event, .. } => match event {
                    WindowEvent::CloseRequested => {
                        println!("Request close");
                        close_requested = true
                    }
                    WindowEvent::CursorMoved { position, .. } => {
                        if is_cursor_grabbed {
                            let delta_x = 0.01 * (window.inner_size().width as f64 * 0.5 - position.x) as f32;
                            let delta_y = -0.01 * (window.inner_size().height as f64 * 0.5 - position.y) as f32;

                            yaw += delta_x;
                            pitch += delta_y;

                            window.set_cursor_position(PhysicalPosition::new((window.inner_size().width as f32) * 0.5, (window.inner_size().height as f32) * 0.5));
                            camera.yaw(yaw);
                            camera.pitch(pitch);
                            camera.update()
                        }
                    }
                    WindowEvent::KeyboardInput { input, ..} => {
                        if let Some(VirtualKeyCode::Escape) = input.virtual_keycode {
                            println!("KeyboardInput: ESCAPE");
                            close_requested = true
                        }
                        else {
                            match input {
                                KeyboardInput {
                                    state: ElementState::Pressed,
                                    virtual_keycode: Some(VirtualKeyCode::Left),
                                    ..
                                } => {
                                    yaw += 5.0;
                                }
                                KeyboardInput {
                                    state: ElementState::Pressed,
                                    virtual_keycode: Some(VirtualKeyCode::Right),
                                    ..
                                } => {
                                    yaw -= 5.0;
                                }
                                KeyboardInput {
                                    state: ElementState::Pressed,
                                    virtual_keycode: Some(VirtualKeyCode::Right),
                                    ..
                                } => {
                                    yaw -= 5.0;
                                }
                                KeyboardInput {
                                    state: ElementState::Pressed,
                                    virtual_keycode: Some(VirtualKeyCode::Up),
                                    ..
                                } => {
                                    pitch -= 5.0;
                                }
                                KeyboardInput {
                                    state: ElementState::Pressed,
                                    virtual_keycode: Some(VirtualKeyCode::Down),
                                    ..
                                } => {
                                    pitch += 5.0;
                                }
                                KeyboardInput {
                                    state: ElementState::Pressed,
                                    virtual_keycode: Some(VirtualKeyCode::W),
                                    ..
                                } => {
                                    camera.forward();
                                }
                                KeyboardInput {
                                    state: ElementState::Pressed,
                                    virtual_keycode: Some(VirtualKeyCode::S),
                                    ..
                                } => {
                                    camera.backward();
                                }
                                KeyboardInput {
                                    state: ElementState::Pressed,
                                    virtual_keycode: Some(VirtualKeyCode::A),
                                    ..
                                } => {
                                    camera.strafe_left();
                                }
                                KeyboardInput {
                                    state: ElementState::Pressed,
                                    virtual_keycode: Some(VirtualKeyCode::D),
                                    ..
                                } => {
                                    camera.strafe_right();
                                }
                                KeyboardInput {
                                    state: ElementState::Pressed,
                                    virtual_keycode: Some(VirtualKeyCode::R),
                                    ..
                                } => {
                                    println!("Reset");
                                    yaw = 0.0;
                                    pitch = 0.0;
                                    camera.reset();
                                }
                                KeyboardInput {
                                    state: ElementState::Pressed,
                                    virtual_keycode: Some(VirtualKeyCode::Space),
                                    ..
                                } => {
                                    is_cursor_grabbed = !is_cursor_grabbed;

                                    window.set_cursor_position(PhysicalPosition::new((window.inner_size().width as f32) * 0.5, (window.inner_size().height as f32) * 0.5));
                                    window.set_cursor_grab(is_cursor_grabbed);
                                    window.set_cursor_visible(!is_cursor_grabbed);
                                }
                                _ => {}
                            }
                            camera.yaw(yaw);
                            camera.pitch(pitch);
                            camera.update()
                        }
                    }
                    _ => (),
                }
                Event::MainEventsCleared => {
                    match (redraw_requested, close_requested) {
                        (false, false) => {}
                        (true, false) => {
                            skyshard::render(&mut engine, &mut world, &camera);
                            frame_count += 1;
                            match frames_per_second_time.elapsed() {
                                Ok(elapsed) => {
                                    if elapsed >= Duration::new(1, 0) {
                                        frames_per_second = frame_count;
                                        frames_per_second_time = SystemTime::now();
                                        frame_count = 0;
                                    }
                                }
                                Err(_) => {}
                            };
                            match render_time.elapsed() {
                                Ok(elapsed) => {
                                    window.set_title(format!("{} {} ms, {} fps", window_title_prefix, elapsed.as_millis(), frames_per_second).as_str());
                                }
                                Err(_) => {}
                            };
                            render_time = SystemTime::now();
                            std::thread::sleep(Duration::from_millis(30))
                        }
                        (_, true) => {
                            println!("Closing");
                            *control_flow = ControlFlow::Exit;
                        }
                    }
                }
                Event::RedrawRequested(window_id) => {}
                _ => (),
            }
        });
    }

    println!("Window closed");
    // std::thread::sleep(Duration::from_millis(500))
}

fn load_image(filepath: &'static str) -> (Extent, Vec<u8>) {

    use std::fs::File;

    let decoder = png::Decoder::new(File::open(filepath).unwrap());
    let mut reader = decoder.read_info().unwrap();
    let mut buf = vec![0; reader.output_buffer_size()];
    let info = reader.next_frame(&mut buf).unwrap();
    let bytes = &buf[..info.buffer_size()];

    (Extent::from(info.width, info.width, 1), Vec::from(bytes))
}
