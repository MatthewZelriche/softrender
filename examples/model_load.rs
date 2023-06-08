#[macro_use]
extern crate softrender_derive;

use glam::Vec3;
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
}

struct Vertex {
    pos: glam::Vec3,
    color: glam::Vec3,
}

struct MyShader;
impl Shader<Vertex, VertexOut> for MyShader {
    fn vertex(&self, vertex: &Vertex) -> (glam::Vec4, VertexOut) {
        (
            vertex.pos.extend(1.0),
            VertexOut {
                color: vertex.color,
            },
        )
    }

    fn fragment(&self, inputs: VertexOut) -> glam::UVec3 {
        inputs.color.as_uvec3()
    }
}

fn main() {
    let mut event_loop = EventLoop::new();
    let window = WindowBuilder::new()
        .with_inner_size(LogicalSize::new(800, 800))
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

    for (pos, (_normal, _uv)) in zip(
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
        });
    }

    let indices = &models[0].mesh.indices;

    let mut renderer = Renderer::new(800, 800);

    let mut shader = MyShader {};

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
