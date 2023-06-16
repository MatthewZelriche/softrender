#[macro_use]
extern crate softrender_derive;

use glam::{Affine3A, Mat4, UVec3, Vec2, Vec3};
use image::{open, RgbImage};
use softbuffer::GraphicsContext;
use std::iter::zip;
use util::camera::Camera;
use winit::dpi::LogicalSize;
use winit::event::{DeviceEvent, ElementState, Event, VirtualKeyCode, WindowEvent};
use winit::event_loop::EventLoop;
use winit::platform::run_return::EventLoopExtRunReturn;
use winit::window::{CursorGrabMode, WindowBuilder};

use softrender::{
    renderer::Renderer,
    shader::{Barycentric, Shader},
};

mod util;

#[derive(Clone, Barycentric)]
struct VertexOut {
    color: glam::Vec3,
    normal: glam::Vec3,
    frag_pos: glam::Vec3,
    uvs: glam::Vec2,
}

struct Vertex {
    pos: glam::Vec3,
    color: glam::Vec3,
    normal: glam::Vec3,
    uvs: glam::Vec2,
}

struct MyShader {
    // Fields in your shader can act as bound uniforms
    proj_mat: Mat4,
    model_mat: Affine3A,
    light_pos: Vec3,
    texture: RgbImage,
}
impl Shader<Vertex, VertexOut> for MyShader {
    fn vertex(&self, vertex: &Vertex) -> (glam::Vec4, VertexOut) {
        let original_vertex_pos = vertex.pos;
        (
            self.proj_mat * self.model_mat * vertex.pos.extend(1.0),
            VertexOut {
                color: vertex.color,
                normal: vertex.normal,
                frag_pos: original_vertex_pos,
                uvs: vertex.uvs,
            },
        )
    }

    fn fragment(&self, inputs: VertexOut) -> glam::UVec3 {
        // TODO: Create some kind of Sampler2D class
        let uvs = inputs.uvs;
        let pix = self.texture.get_pixel(
            ((1.0 - uvs.x) * (self.texture.width() - 1) as f32) as u32,
            ((1.0 - uvs.y) * (self.texture.height() - 1) as f32) as u32,
        );

        let obj_col = UVec3 {
            x: pix[0] as u32,
            y: pix[1] as u32,
            z: pix[2] as u32,
        };

        // Calculations are performed in a normalized range, then scaled to 0-255.
        // TODO: Consider returning a value between 0-1 instead of 0-255?
        let light_color = Vec3::new(1.0, 1.0, 1.0);

        let ambient_intensity = 0.1;
        let ambient_light = ambient_intensity * light_color;

        let light_dir = (self.light_pos - inputs.frag_pos).normalize();
        let diffuse_intensity = f32::max(inputs.normal.dot(light_dir), 0.0);
        let diffuse_light = diffuse_intensity * light_color;

        let final_lighting =
            (ambient_light + diffuse_light).clamp(Vec3::ZERO, Vec3::new(1.0, 1.0, 1.0));

        (final_lighting * obj_col.as_vec3()).as_uvec3()
    }
}

fn main() {
    let mut event_loop = EventLoop::new();
    let window = WindowBuilder::new()
        .with_inner_size(LogicalSize::new(1024, 768))
        .with_resizable(false)
        .build(&event_loop)
        .expect("Failed to initialize window");
    let mut gc = unsafe { GraphicsContext::new(&window, &window).expect("Failed to create GC") };

    let load_opt = tobj::GPU_LOAD_OPTIONS;
    let (models, _) = tobj::load_obj("african_head.obj", &load_opt).expect("Could not load model.");

    let mut vertices = Vec::new();

    let pos_data = &models[0].mesh.positions;
    let normal_data = &models[0].mesh.normals;
    let uv_data = &models[0].mesh.texcoords;

    for (pos, (normal, uvs)) in zip(
        pos_data.chunks(3),
        zip(normal_data.chunks(3), uv_data.chunks(2)),
    ) {
        vertices.push(Vertex {
            pos: glam::Vec3::from_slice(pos),
            color: Vec3::new(
                rand::random::<f32>() * 255.0,
                rand::random::<f32>() * 255.0,
                rand::random::<f32>() * 255.0,
            ),
            normal: Vec3::from_slice(normal),
            uvs: Vec2::from_slice(uvs),
        });
    }

    let indices = &models[0].mesh.indices;

    let window_size = window.inner_size();
    let mut renderer = Renderer::new(window_size.width as u16, window_size.height as u16);
    let fov = 60.0;

    let mut shader = MyShader {
        proj_mat: Mat4::perspective_rh(
            f32::to_radians(fov),
            window_size.width as f32 / window_size.height as f32,
            0.1,
            5.0,
        ),
        model_mat: Affine3A::from_translation(Vec3::new(0.0, 0.0, -3.5))
            * Affine3A::from_rotation_y(f32::to_radians(30.0)),
        light_pos: Vec3::new(0.0, 0.0, 5.0),
        texture: open("african_head_diffuse.tga").unwrap().into_rgb8(),
    };

    let mut cam = Camera::new(
        f32::to_radians(fov),
        window_size.width as f32 / window_size.height as f32,
        0.1,
        50.0,
        Vec3::new(0.0, 0.0, 0.0),
    );

    // Performance counter vars
    let mut frames = 0;
    let mut total = 0.0;

    let mut is_pressed = [false; 4];
    let mut frame_delta = 1.0 / 100.0;

    window.set_cursor_grab(CursorGrabMode::Confined).unwrap();

    event_loop.run_return(|event, _, cf| {
        cf.set_poll();

        match event {
            Event::WindowEvent {
                event: window_event,
                ..
            } => match window_event {
                WindowEvent::CloseRequested => cf.set_exit(),
                WindowEvent::Resized(inner_size) => {
                    renderer.set_fb_size(inner_size.width as u16, inner_size.height as u16);
                    shader.proj_mat = Mat4::perspective_rh(
                        f32::to_radians(fov),
                        inner_size.width as f32 / inner_size.height as f32,
                        0.1,
                        5.0,
                    );
                }
                WindowEvent::KeyboardInput { input, .. } => match input.virtual_keycode {
                    Some(keycode) => match keycode {
                        VirtualKeyCode::W => is_pressed[0] = input.state == ElementState::Pressed,
                        VirtualKeyCode::S => is_pressed[1] = input.state == ElementState::Pressed,
                        VirtualKeyCode::A => is_pressed[2] = input.state == ElementState::Pressed,
                        VirtualKeyCode::D => is_pressed[3] = input.state == ElementState::Pressed,
                        _ => (),
                    },
                    None => (),
                },
                _ => (),
            },

            Event::DeviceEvent {
                event: DeviceEvent::MouseMotion { delta },
                ..
            } => {
                let pitch_delta = delta.1 as f32 * frame_delta * 0.6;
                let yaw_delta = delta.0 as f32 * frame_delta * 0.6;
                cam.rotate(-pitch_delta as f32, -yaw_delta as f32);
            }

            Event::MainEventsCleared => {
                let mut move_amt = Vec2::ZERO;
                if is_pressed[0] {
                    move_amt.x += 1.0;
                }
                if is_pressed[1] {
                    move_amt.x -= 1.0;
                }
                if is_pressed[2] {
                    move_amt.y += 1.0;
                }
                if is_pressed[3] {
                    move_amt.y -= 1.0;
                }
                move_amt = move_amt.normalize_or_zero() * 4.0 * frame_delta;
                cam.move_cam(move_amt);
                cam.tick();
                shader.proj_mat = cam.view_projection_matrix();
                let now = std::time::Instant::now();

                renderer.clear_framebuffer(95 | 95 << 8 | 95 << 16);
                let color_buf = renderer.draw(&mut shader, &vertices, &indices);
                gc.set_buffer(
                    color_buf.get_raw(),
                    color_buf.get_width(),
                    color_buf.get_height(),
                );

                // Calculate frametime.
                let elapsed_time = now.elapsed().as_secs_f32();
                frame_delta = elapsed_time;
                total += elapsed_time;
                frames += 1;
                if total >= 5.0 {
                    let avg = total / frames as f32;
                    println!(
                        "Avg frametime: {:.4}s / {:.3}ms / {:.3}us ({} fps)",
                        avg,
                        avg * 1000.0,
                        avg * 1000.0 * 1000.0,
                        1.0 / avg
                    );
                    frames = 0;
                    total = total - 5.0;
                }
            }
            _ => (),
        }
    });
}
