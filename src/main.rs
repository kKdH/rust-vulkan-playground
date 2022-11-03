extern crate ash;
extern crate skyshard;
extern crate winit;

use std::borrow::Borrow;
use std::ops::Deref;
use std::time::{Duration, SystemTime};
use std::vec;

use log::{info, LevelFilter};
use log4rs;
use log4rs::append::console::ConsoleAppender;
use log4rs::append::file::FileAppender;
use log4rs::config::{Appender, Config, Root};
use log4rs::encode::pattern::PatternEncoder;
use nalgebra::Matrix4;
use nalgebra::Vector3;
use rand::Rng;
use winit::dpi::PhysicalPosition;
use winit::event::{ElementState, Event, KeyboardInput, VirtualKeyCode, WindowEvent};
use winit::event_loop::{ControlFlow, EventLoop};
use winit::window::WindowBuilder;

use blend_rs::blend::{PointerLike, StringLike};
use blend_rs::blend::traverse::Named;
use blend_rs::blender3_3::{bNode, bNodeTree, DrawDataList, Image, Link, Material, Mesh, MLoop, MLoopUV, MVert, Object};
use skyshard::{InstanceData, Vertex};
use skyshard::entity::World;
use skyshard::graphics::{Camera, Extent};

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

    {
        let mut engine = skyshard::create("Rust Vulkan Example", &window).unwrap();
        let asset_manager = engine.asset_manager();
        let mut world = World::new();

        let cube = {

            let transformation1 = Matrix4::<f32>::identity()
                .append_translation(&Vector3::new(0.0, 0.0, 0.0));

            let mut transformation2 = Matrix4::<f32>::identity()
                .append_translation(&Vector3::new(3.5, 0.0, 0.0));

            transformation2 = transformation2 * Matrix4::<f32>::from_euler_angles(0.0, 0.5, -0.7);

            let mut  transformation3 = Matrix4::<f32>::identity()
                .append_translation(&Vector3::new(0.0, 3.0, 1.5));

            transformation3 = transformation3 * Matrix4::<f32>::from_euler_angles(0.5, 0.3, 0.5);

            let file_path = "assets/cube.blend";
            let blend_data = std::fs::read(file_path).unwrap();
            let blend_reader = blend_rs::blend::read(&blend_data)
                    .expect(&format!("Failed to read: {}", file_path));

            let object: &Object = blend_reader.iter::<Object>()
                .unwrap()
                .find(|object| object.id.get_name() == "Cube")
                .unwrap();

            let mesh: &Mesh = blend_reader.deref_single(&object.data.as_instance_of::<Mesh>())
                .unwrap();

            let vertices: Vec<Vertex> = {

                let mesh_polygons = blend_reader.deref(&mesh.mpoly)
                    .expect("mesh of object 'Cube' should have polygons");

                let mesh_loops: Vec<&MLoop> = blend_reader.deref(&mesh.mloop)
                    .expect("mesh of object 'Cube' should have loops")
                    .collect();

                let mesh_uvs: Vec<&MLoopUV> = blend_reader.deref(&mesh.mloopuv)
                    .expect("mesh of object 'Cube' should have UVs")
                    .collect();

                let mesh_vertices: Vec<&MVert> = blend_reader.deref(&mesh.mvert)
                    .expect("mesh of object 'Cube' should have vertices")
                    .collect();

                mesh_polygons
                    .map(|polygon| {
                        (polygon.loopstart..polygon.loopstart + polygon.totloop).into_iter()
                            .map(|loop_index| {
                                let uv = mesh_uvs[loop_index as usize].uv;
                                let position = mesh_vertices[mesh_loops[loop_index as usize].v as usize].co;
                                Vertex {
                                    position: [position[0], position[2] * -1.0, position[1]], // # blender's z-up to y-up: x,y,z -> x,z,-y
                                    color: [0.0, 0.0, 0.0],
                                    uv: [uv[0], (uv[1] * -1.0) + 1.0], // blender: u,v -> u,1-v
                                }
                            })
                            .collect::<Vec<Vertex>>()
                    })
                    .flatten()
                    .collect()
            };

            let indices = (0u32..vertices.len() as u32).collect();

            let (texture_extent, texture_data) = {

                let material = blend_reader.deref_single(&mesh.mat.as_instance_of::<DrawDataList>())
                    .map(|list| {
                        blend_reader.deref_single(&list.first.as_instance_of::<Material>()).unwrap()
                    })
                    .unwrap();

                let tree: &bNodeTree = blend_reader.deref_single(&material.nodetree)
                    .unwrap();

                let tex_image_node = blend_reader.traverse_double_linked(&tree.nodes.first.as_instance_of::<bNode>())
                    .unwrap()
                    .find(|node: &bNode| node.idname.to_str_unchecked() == "ShaderNodeTexImage")
                    .unwrap();

                let tex_image: &Image = blend_reader.deref_single(&tex_image_node.id.as_instance_of::<Image>())
                    .unwrap();

                let image_packed_file = blend_reader.deref_single(&tex_image.packedfile)
                    .unwrap();

                let data = blend_reader.deref_raw_range(&image_packed_file.data, 0..image_packed_file.size as usize)
                    .unwrap();

                let decoder = ::png::Decoder::new(data);
                let mut reader = decoder.read_info().unwrap();
                let mut buf = vec![0; reader.output_buffer_size()];
                let info = reader.next_frame(&mut buf).unwrap();
                let bytes = &buf[..info.buffer_size()];

                (Extent::from(info.width, info.width, 1), Vec::from(bytes))
            };

            skyshard::create_geometry(&mut engine,
                &indices,
                &vertices,
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
                ]
            )
        };

        world.geometries.push(cube);

        let mut redraw_requested = true;
        let mut close_requested = false;

        engine.reference_counts();

        let mut camera = Camera::new(
            window_width as f32 / window_height as f32,
            3.14 / 4.0,
            0.01,
            100.0
        );

        camera.eye(0f32, 0f32, 5f32);
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

        let mut translation: Vector3<f32> = Vector3::new(0.0, 0.0, 0.0);
        let mut rotation = 0.0f32;
        let mut move_speed = 1f32;
        let mut rotation_speed = 0.01f32;

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

                                    window.set_cursor_position(PhysicalPosition::new(
                                        (window.inner_size().width as f32) * 0.5,
                                        (window.inner_size().height as f32) * 0.5
                                    )).unwrap();
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

                            let mut cube = &mut world.geometries[0];

                            if translation.x < -1.0 {
                                move_speed = 0.05
                            }

                            if translation.x > 1.0 {
                                move_speed = -0.05
                            }

                            translation.x = translation.x + move_speed;

                            rotation = rotation + 0.01;

                            let transformation1 = Matrix4::<f32>::identity()
                                .append_translation(&translation);

                            let mut transformation2 = Matrix4::<f32>::identity()
                                .append_translation(&Vector3::new(3.5, 0.0, 0.0));

                            transformation2 = transformation2 * Matrix4::<f32>::from_euler_angles(0.0, 0.5, rotation);

                            let mut  transformation3 = Matrix4::<f32>::identity()
                                .append_translation(&Vector3::new(0.0, 3.0, 1.5));

                            transformation3 = transformation3 * Matrix4::<f32>::from_euler_angles(0.0, rotation, 0.5);

                            let transformations = vec![
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
                            ];

                            skyshard::update_geometry(&mut engine, &mut cube, &transformations);

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
