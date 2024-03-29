extern crate skyshard;
extern crate winit;

use std::borrow::Borrow;
use std::ops::{Deref, Mul};
use std::time::{Duration, SystemTime};
use std::vec;

use log::{info, LevelFilter};
use log4rs;
use log4rs::append::console::ConsoleAppender;
use log4rs::append::file::FileAppender;
use log4rs::config::{Appender, Config, Root};
use log4rs::encode::pattern::PatternEncoder;
use nalgebra::{Matrix4, Vector2};
use nalgebra::Vector3;
use rand::Rng;
use winit::dpi::PhysicalPosition;
use winit::event::{ElementState, Event, KeyboardInput, MouseButton, VirtualKeyCode, WindowEvent};
use winit::event_loop::{ControlFlow, EventLoop};
use winit::window::{CursorGrabMode, WindowBuilder};

use blend_rs::blend::{NameLike, PointerLike, StringLike};
use blend_rs::blend::traverse::Named;
use blend_rs::blender3_3::{bNode, bNodeTree, DrawDataList, Image, Material, Mesh, MLoop, MLoopUV, MVert, Object};
use skyshard::{InstanceData, pick_object, Vertex};
use skyshard::entity::World;
use skyshard::graphics::{Camera, Extent};
use skyshard::graphics::Projection::PerspectiveProjection;
use crate::clock::Clock;

use crate::input::{KeyAction, MovementController, MovementControllerSettings};
use crate::movable::Movable;

mod clock;
mod input;
mod movable;
mod shaders;
mod noise;


fn main() {

    let stdout = ConsoleAppender::builder()
        .encoder(Box::new(PatternEncoder::new("[{d(%Y-%m-%d %H:%M:%S.%3f%Z)(utc)}] [{h({l})}] {T} {M} - {m}\n")))
        .build();

    let fileout = FileAppender::builder()
        .encoder(Box::new(PatternEncoder::new("[{d(%Y-%m-%d %H:%M:%S.%3f%Z)(utc)}] [{h({l})}] {T} {M} - {m}\n")))
        .append(false)
        .build("logs/application.log")
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

    let window_width: f32 = 800.0;
    let window_height: f32 = 600.0;

    let window_title_prefix = "rust vulkan example: ";
    let window = WindowBuilder::new()
        .with_title(window_title_prefix)
        .with_inner_size(winit::dpi::LogicalSize::new(window_width, window_height,))
        .build(&events_loop)
        .unwrap();

    {
        let mut engine = skyshard::create(
            "Rust Vulkan Example",
            &window,
            shaders::vs::shader(),
            shaders::fs::shader(),
        ).unwrap();

        let asset_manager = engine.asset_manager();
        let mut clock = Clock::new(0.01);
        let mut world = World::new();

        let cube = {

            let transformation1 = Matrix4::<f32>::identity()
                .append_translation(&Vector3::new(0.0, 0.0, 0.0))
                .transpose();

            let mut transformation2 = Matrix4::<f32>::identity()
                .append_translation(&Vector3::new(3.5, 0.0, 0.0))
                .transpose();

            transformation2 = transformation2 * Matrix4::<f32>::from_euler_angles(0.0, 0.5, -0.7);

            let mut  transformation3 = Matrix4::<f32>::identity()
                .append_translation(&Vector3::new(0.0, 3.0, 1.5))
                .transpose();

            transformation3 = transformation3 * Matrix4::<f32>::from_euler_angles(0.5, 0.3, 0.5);

            let file_path = "assets/cube.blend";
            let blend_data = std::fs::read(file_path).unwrap();
            let blend_reader = blend_rs::blend::read(&blend_data)
                    .expect(&format!("Failed to read: {}", file_path));

            let object_name = "cube";// "tetrahedron";

            let object: &Object = blend_reader.iter::<Object>()
                .unwrap()
                .find(|object| object.id.get_name() == object_name)
                .expect(format!("blend file should contain object '{}'", object_name).as_str());

            let mesh: &Mesh = blend_reader.deref_single(&object.data.as_instance_of::<Mesh>())
                .unwrap();

            let vertices: Vec<Vertex> = {

                let mesh_polygons = blend_reader.deref(&mesh.mpoly)
                    .expect(format!("mesh of object '{}' should have polygons", object_name).as_str());

                let mesh_loops: Vec<&MLoop> = blend_reader.deref(&mesh.mloop)
                    .expect(format!("mesh of object '{}' should have loops", object_name).as_str())
                    .collect();

                let mesh_uvs: Vec<&MLoopUV> = blend_reader.deref(&mesh.mloopuv)
                    .expect(format!("mesh of object '{}' should have UVs", object_name).as_str())
                    .collect();

                let mesh_vertices: Vec<&MVert> = blend_reader.deref(&mesh.mvert)
                    .expect(format!("mesh of object '{}' should have vertices", object_name).as_str())
                    .collect();

                let mk_vert = |index| {
                    let uv = mesh_uvs[index as usize].uv;
                    let position = mesh_vertices[mesh_loops[index as usize].v as usize].co;
                    Vertex {
                        position: [position[0], position[2] * -1.0, position[1]], // # blender's z-up to y-up: x,y,z -> x,z,-y
                        color: [0.0, 0.0, 0.0],
                        uv: [uv[0], (uv[1] * -1.0) + 1.0], // blender: u,v -> u,1-v
                    }
                };

                mesh_polygons.fold(Vec::new(), | mut vertices, polygon| {

                        vertices.push(mk_vert(polygon.loopstart));
                        vertices.push(mk_vert(polygon.loopstart + 1));
                        vertices.push(mk_vert(polygon.loopstart + 2));

                        if polygon.totloop == 4 {
                            vertices.push(mk_vert(polygon.loopstart));
                            vertices.push(mk_vert(polygon.loopstart + 2));
                            vertices.push(mk_vert(polygon.loopstart + 3));
                        }

                        vertices
                    })
            };

            let indices: Vec<u32> = (0u32..vertices.len() as u32).collect();

            let (texture_extent, texture_data) = {

                let material = blend_reader.deref_single(&mesh.mat.as_instance_of::<DrawDataList>())
                    .map(|list| {
                        blend_reader.deref_single(&list.first.as_instance_of::<Material>()).unwrap()
                    })
                    .expect("A material to load for the object");

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
                        id: 1,
                        transformation: transformation1.data
                            .as_slice()
                            .try_into()
                            .expect("slice with incorect length")
                    },
                    InstanceData {
                        id: 2,
                        transformation: transformation2.data
                            .as_slice()
                            .try_into()
                            .expect("slice with incorect length")
                    },
                    InstanceData {
                        id: 3,
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
            PerspectiveProjection {
                fovy: 50f32.to_radians(),
                aspect: window_width / window_height,
                near: 0.1,
                far: 100.0
            }
        );

        camera.view_direction(
            &Vector3::new(0.0, 0.0, -5.0),
            &Vector3::new(0.0, 0.0, 1.0),
            &Vector3::new(0.0, -1.0, 0.0)
        );

        let mut movable = Movable::new();
        movable.translate(&Vector3::new(0.0, 0.0, -5.0));

        let mut movement_controller = MovementController::new(
            MovementControllerSettings {
                rotation_speed: 0.25,
                translation_speed: 1.0,
                mouse_acceleration: Vector2::new(1.0, 1.0),
                fast_movement_multiplier: 4.0,
                reset_rotation: Vector3::new(0.0, 0.0, 0.0),
                reset_translation: Vector3::new(0.0, 0.0, -5.0),
            }
        );

        let mut last_cursor_x = 0i32;
        let mut last_cursor_y = 0i32;
        let mut frames_per_second_time: SystemTime = SystemTime::now();
        let mut frame_count: u32 = 0;
        let mut frames_per_second: u32 = 0;

        skyshard::prepare(&mut engine, &mut world);

        info!("Starting event loop");

        let mut grabbed_object_id: Option<u32> = None;
        let mut translations: [Vector3<f32>; 3] = [Vector3::new(0.0, 0.0, 0.0); 3];

        let mut sleep_time_millis: u64 = 10;

        events_loop.run(move |event, _, control_flow| {

            match event {
                Event::WindowEvent { event, .. } => {

                    movement_controller.handle(&event);

                    match event {
                        WindowEvent::CloseRequested => {
                            println!("Request close");
                            close_requested = true
                        }
                        WindowEvent::CursorMoved { position, .. } => {
                            if movement_controller.is_active(&KeyAction::MouseLook) {
                                window.set_cursor_position(PhysicalPosition::new((window.inner_size().width as f32) * 0.5, (window.inner_size().height as f32) * 0.5))
                                    .expect("center cursor postion");
                            }

                            if let Some(object_id) = grabbed_object_id {
                                let object_index = (object_id - 1) as usize;
                                let delta_x = position.x as f32 - last_cursor_x as f32;
                                let delta_y = position.y as f32 - last_cursor_y as f32;

                                translations[object_index].x = translations[object_index].x + 0.01 * delta_x;
                                translations[object_index].y = translations[object_index].y + 0.01 * delta_y;
                            }

                            last_cursor_x = position.x as i32;
                            last_cursor_y = position.y as i32;
                        }
                        WindowEvent::KeyboardInput { input, .. } => {
                            if let Some(VirtualKeyCode::Escape) = input.virtual_keycode {
                                println!("KeyboardInput: ESCAPE");
                                close_requested = true
                            }
                            else {
                                match input {
                                    KeyboardInput {
                                        state: ElementState::Pressed,
                                        virtual_keycode: Some(VirtualKeyCode::Key1),
                                        ..
                                    } => {
                                        sleep_time_millis = 0;
                                    }
                                    KeyboardInput {
                                        state: ElementState::Pressed,
                                        virtual_keycode: Some(VirtualKeyCode::Key2),
                                        ..
                                    } => {
                                        sleep_time_millis = 10;
                                    }
                                    KeyboardInput {
                                        state: ElementState::Pressed,
                                        virtual_keycode: Some(VirtualKeyCode::Key3),
                                        ..
                                    } => {
                                        sleep_time_millis = 80;
                                    }
                                    _ => {}
                                }
                            }
                        }
                        WindowEvent::MouseInput {
                            button: MouseButton::Left,
                            state: ElementState::Pressed,
                            ..
                        } => {
                            let object_id: Option<u32> = pick_object(&mut engine, last_cursor_x, last_cursor_y);
                            println!("Picked: {object_id:?} at {last_cursor_x}/{last_cursor_y}");
                            grabbed_object_id = object_id;

                            // let ray: Vector4<f32> = {

                            // let ray_clip_coordinates = Vector4::<f32>::new(
                            //     (2 * last_cursor_x) as f32 / window_width as f32 - 1.0,
                            //     1.0 - (2 * last_cursor_y) as f32 / window_height as f32,
                            //     -1.0,
                            //     1.0,
                            // );

                            // let inverse_projection = camera.projection.try_inverse().unwrap();
                            // let inverse_view = camera.view.try_inverse().unwrap();
                            //
                            // let ray_eye: Vector4<f32> = {
                            //     let vec: Vector4<f32> = inverse_projection.mul(ray_clip_coordinates);
                            //     Vector4::<f32>::new(vec.x, vec.y, -1.0, 0.0).normalize()
                            // };
                            //
                            // inverse_view.mul(ray_eye).normalize()
                            // };

                            // println!("Ray: {:?}", ray);
                        }
                        WindowEvent::MouseInput {
                            button: MouseButton::Left,
                            state: ElementState::Released,
                            ..
                        } => {
                            grabbed_object_id = None;
                        }
                        _ => (),
                    }
                }
                Event::MainEventsCleared => {
                    match (redraw_requested, close_requested) {
                        (false, false) => {}
                        (true, false) => {

                            clock.produce();

                            if movement_controller.is_active(&KeyAction::MouseLook) {
                                window.set_cursor_visible(false);
                                window.set_cursor_grab(CursorGrabMode::Confined)
                                    .or_else(|_| window.set_cursor_grab(CursorGrabMode::Locked))
                                    .unwrap();
                            } else {
                                window.set_cursor_visible(true);
                                window.set_cursor_grab(CursorGrabMode::None)
                                    .unwrap();
                            }

                            let mut cube = &mut world.geometries[0];

                            let mut transformations: [Matrix4::<f32>; 3] = [
                                Matrix4::<f32>::identity()
                                    .append_translation(&Vector3::new(0.0, 0.0, 0.0))
                                    .append_translation(&translations[0])
                                    .transpose(),
                                Matrix4::<f32>::identity()
                                    .append_translation(&Vector3::new(3.5, 0.0, 0.0))
                                    .append_translation(&translations[1])
                                    .transpose(),
                                Matrix4::<f32>::identity()
                                    .append_translation(&Vector3::new(0.0, 3.0, 1.5))
                                    .append_translation(&translations[2])
                                    .transpose(),
                            ];

                            let transformations = vec![
                                InstanceData {
                                    id: 1,
                                    transformation: transformations[0].data
                                        .as_slice()
                                        .try_into()
                                        .expect("slice with incorect length"),
                                },
                                InstanceData {
                                    id: 2,
                                    transformation: transformations[1].data
                                        .as_slice()
                                        .try_into()
                                        .expect("slice with incorect length"),
                                },
                                InstanceData {
                                    id: 3,
                                    transformation: transformations[2].data
                                        .as_slice()
                                        .try_into()
                                        .expect("slice with incorrect length")
                                },
                            ];

                            skyshard::update_geometry(&mut engine, &mut cube, &transformations);

                            while let Some(tick) = clock.consume() {
                                // update all state with Tick(t, dt)
                                movement_controller.apply(&tick, &mut movable);
                                camera.view_yxz(movable.translation(), movable.rotation());
                            };

                            skyshard::render(&mut engine, &mut world, &camera);
                            std::thread::sleep(Duration::from_millis(sleep_time_millis));

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

                            window.set_title(format!("{} {} ms, {} fps", window_title_prefix, clock.frame_time().as_millis(), frames_per_second).as_str())
                        }
                        (_, true) => {
                            println!("Closing");
                            *control_flow = ControlFlow::Exit;
                        }
                    }
                }
                _ => (),
            }
        });
    }
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

#[cfg(test)]
mod test {

}
