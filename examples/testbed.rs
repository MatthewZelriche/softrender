#[macro_use]
extern crate softrender_derive;

use glam::{Affine3A, Mat4, Vec3};
use softbuffer::GraphicsContext;
use std::iter::zip;
use winit::dpi::LogicalSize;
use winit::event::{Event, WindowEvent};
use winit::event_loop::EventLoop;
use winit::platform::run_return::EventLoopExtRunReturn;
use winit::window::WindowBuilder;

use softrender::{
    renderer::Renderer,
    shader::{Barycentric, Shader},
};

#[derive(Barycentric)]
struct VertexOut {
    color: glam::Vec3,
    normal: glam::Vec3,
    frag_pos: glam::Vec3,
}

struct Vertex {
    pos: glam::Vec3,
    color: glam::Vec3,
    normal: glam::Vec3,
}

struct MyShader {
    // Fields in your shader can act as bound uniforms
    proj_mat: Mat4,
    model_mat: Affine3A,
    light_pos: Vec3,
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
            },
        )
    }

    fn fragment(&self, inputs: VertexOut) -> glam::UVec3 {
        // Calculations are performed in a normalized range, then scaled to 0-255.
        // TODO: Consider returning a value between 0-1 instead of 0-255?
        let light_color = Vec3::new(1.0, 1.0, 1.0);

        let ambient_intensity = 0.1;
        let ambient_light = ambient_intensity * light_color;

        let light_dir = (self.light_pos - inputs.frag_pos).normalize();
        let diffuse_intensity = f32::max(inputs.normal.dot(light_dir), 0.0);
        let diffuse_light = diffuse_intensity * light_color;

        ((ambient_light + diffuse_light) * 255.0)
            .clamp(Vec3::ZERO, Vec3::new(255.0, 255.0, 255.0))
            .as_uvec3()
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

    for (pos, (normal, _uv)) in zip(
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
        });
    }

    let indices = &models[0].mesh.indices;

    let window_size = window.inner_size();
    let mut renderer = Renderer::new(window_size.width as u16, window_size.height as u16);

    let mut shader = MyShader {
        proj_mat: Mat4::perspective_rh(
            f32::to_radians(90.0),
            window_size.width as f32 / window_size.height as f32,
            0.1,
            5.0,
        ),
        model_mat: Affine3A::from_translation(Vec3::new(0.0, 0.0, -1.5))
            * Affine3A::from_rotation_y(f32::to_radians(25.0)),
        light_pos: Vec3::new(0.0, 0.0, 5.0),
    };

    // Performance counter vars
    let mut frames = 0;
    let mut total = 0.0;

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
                        f32::to_radians(90.0),
                        inner_size.width as f32 / inner_size.height as f32,
                        0.1,
                        5.0,
                    );
                }
                _ => (),
            },

            Event::MainEventsCleared => {
                let now = std::time::Instant::now();

                renderer.clear_color(95 | 95 << 8 | 95 << 16);
                let color_buf = renderer.draw(&mut shader, &vertices, &indices);
                gc.set_buffer(
                    color_buf.get_raw(),
                    color_buf.get_width(),
                    color_buf.get_height(),
                );

                // Calculate frametime.
                let elapsed_time = now.elapsed().as_secs_f32();
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
