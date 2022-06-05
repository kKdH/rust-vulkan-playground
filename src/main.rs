extern crate ash;
extern crate skyshard;
extern crate winit;

use std::borrow::{Borrow, BorrowMut};
use std::cell::{Cell, RefCell};
use std::f32::consts::FRAC_PI_2;
use std::ffi::{CStr, CString};
use std::io::Cursor;
use std::iter::Copied;
use std::ops::Deref;
use std::process::exit;
use std::rc::{Rc, Weak};
use std::slice::Iter;
use std::sync::Arc;
use std::time::Duration;
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
use skyshard::entity::{World};
use skyshard::graphics::{Camera, Position};
use skyshard::Vertex;
use winit::dpi::PhysicalPosition;
use winit::event::{ElementState, Event, KeyboardInput, VirtualKeyCode, WindowEvent};
use winit::event_loop::{ControlFlow, EventLoop};
use winit::window::{Window, WindowBuilder};

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

    let window_width = 640;
    let window_height = 480;

    let window = WindowBuilder::new()
        .with_title("rust vulkan example")
        .with_inner_size(winit::dpi::LogicalSize::new(
            f64::from(window_width),
            f64::from(window_height),
        ))
        .build(&events_loop)
        .unwrap();

    {
        let mut engine = skyshard::create("Rust Vulkan Example", &window).unwrap();
        let mut world = World::new();

        world.geometries.push(skyshard::create_geometry(&engine,
            Position::new(0.0, 0.0, 0.0),
            Vec::from([0, 1, 2, 2, 3, 0]),
            Vec::from([
                Vertex {
                    position: [-0.5, -0.5, 0.0], // top-left
                    color: [1.0, 0.0, 0.0]
                },
                Vertex {
                    position: [0.5, -0.5, 0.0], // top-right
                    color: [0.0, 1.0, 0.0]
                },
                Vertex {
                    position: [0.5, 0.5, 0.0], // bottom-right
                    color: [0.0, 0.0, 1.0]
                },
                Vertex {
                    position: [-0.5, 0.5, 0.0], // bottom-left
                    color: [1.0, 1.0, 1.0]
                },
            ])
        ));

        let mut redraw_requested = true;
        let mut close_requested = false;

        info!("Starting event loop");
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

        skyshard::render(&mut engine, &world, &camera);

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
                            skyshard::render(&mut engine, &world, &camera);
                            std::thread::sleep(Duration::from_millis(50))
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

mod tests {

}
