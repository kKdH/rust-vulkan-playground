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
use std::rc::{Rc, Weak};
use std::sync::Arc;

use ash::extensions::{
    ext::DebugUtils,
    khr::{Surface, Swapchain},
};
use ash::version::{DeviceV1_0, DeviceV1_2, EntryV1_0, InstanceV1_0};
use ash::vk;
use ash::vk::{Extent2D, PhysicalDevice, Queue, SurfaceCapabilitiesKHR, SurfaceFormatKHR, SurfaceKHR};
use cgmath::{Matrix3, Matrix4, Point3, Rad, Vector3};
use log::{error, info, debug, LevelFilter, warn};
use log4rs;
use log4rs::append::console::ConsoleAppender;
use log4rs::append::file::FileAppender;
use log4rs::config::{Appender, Config, Logger, Root};
use log4rs::encode::pattern::PatternEncoder;
use winit::event::{Event, VirtualKeyCode, WindowEvent};
use winit::event_loop::{ControlFlow, EventLoop};
use winit::window::{Window, WindowBuilder};
use std::time::Duration;
use winit::platform::desktop::EventLoopExtDesktop;

mod camera;


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
            .build(LevelFilter::Info))
        .unwrap();

    // use handle to change logger configuration at runtime
    let handle = log4rs::init_config(config).unwrap();

    let mut events_loop = EventLoop::new();

    let window_width = 800;
    let window_height = 600;

    let window = WindowBuilder::new()
        .with_title("rust vulkan example")
        .with_inner_size(winit::dpi::LogicalSize::new(
            f64::from(window_width),
            f64::from(window_height),
        ))
        .build(&events_loop)
        .unwrap();

    {
        let engine = skyshard::create("Rust Vulkan Example", &window).unwrap();

        let mut redraw_requested = false;
        let mut close_requested = false;

        info!("Starting event loop");
        engine.reference_counts();

        events_loop.run_return(move |event, _, control_flow| {
            match event {
                Event::WindowEvent { event, .. } => match event {
                    WindowEvent::CloseRequested => {
                        println!("Request close");
                        close_requested = true
                    }
                    WindowEvent::KeyboardInput { input, .. } => {
                        if let Some(VirtualKeyCode::Escape) = input.virtual_keycode {
                            println!("KeyboardInput: ESCAPE");
                            close_requested = true
                        }
                    }
                    _ => (),
                }
                Event::MainEventsCleared => {
                    match (redraw_requested, close_requested) {
                        (false, false) => {}
                        (true, false) => {}
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
