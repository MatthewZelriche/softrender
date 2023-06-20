#[macro_use]
extern crate softrender_derive;

use glam::{Vec2, Vec3};
use image::{open, RgbImage};
use softbuffer::GraphicsContext;
use softrender::{
    renderer::Renderer,
    shader::{Barycentric, Shader},
};
use winit::{
    dpi::LogicalSize,
    event::{Event, WindowEvent},
    event_loop::EventLoop,
    platform::run_return::EventLoopExtRunReturn,
    window::WindowBuilder,
};

#[derive(Clone, Barycentric)]
struct VertexOut {
    uv: glam::Vec2,
}

struct Vertex {
    pos: glam::Vec3,
    uv: glam::Vec2,
}

struct MyShader {
    texture: RgbImage,
}

impl Shader<Vertex, VertexOut> for MyShader {
    fn vertex(&self, vertex: &Vertex) -> (glam::Vec4, VertexOut) {
        (vertex.pos.extend(1.0), VertexOut { uv: vertex.uv })
    }

    fn fragment(&self, inputs: VertexOut) -> glam::UVec3 {
        let pix = self.texture.get_pixel(
            ((inputs.uv.x) * (self.texture.width() - 1) as f32) as u32,
            ((1.0 - inputs.uv.y) * (self.texture.height() - 1) as f32) as u32,
        );

        glam::UVec3 {
            x: pix[0] as u32,
            y: pix[1] as u32,
            z: pix[2] as u32,
        }
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

    let mut renderer = Renderer::new(800, 800);
    let mut shader = MyShader {
        texture: open("res/texture.png")
            .expect("Failed to load texture")
            .into_rgb8(),
    };

    let mut vertices = Vec::new();
    vertices.push(Vertex {
        pos: Vec3::new(-0.5, -0.5, 0.0),
        uv: Vec2::new(0.0, 0.0),
    });
    vertices.push(Vertex {
        pos: Vec3::new(0.5, -0.5, 0.0),
        uv: Vec2::new(1.0, 0.0),
    });
    vertices.push(Vertex {
        pos: Vec3::new(-0.5, 0.5, 0.0),
        uv: Vec2::new(0.0, 1.0),
    });
    vertices.push(Vertex {
        pos: Vec3::new(0.5, 0.5, 0.0),
        uv: Vec2::new(1.0, 1.0),
    });
    let indices = [0, 1, 2, 2, 1, 3];

    event_loop.run_return(|event, _, cf| {
        cf.set_poll();

        match event {
            Event::WindowEvent {
                event: window_event,
                ..
            } => match window_event {
                WindowEvent::CloseRequested => cf.set_exit(),
                _ => (),
            },

            Event::MainEventsCleared => {
                renderer.clear_framebuffer(95 | 95 << 8 | 95 << 16);
                let color_buf = renderer.draw(&mut shader, &vertices, &indices);
                gc.set_buffer(
                    color_buf.get_raw(),
                    color_buf.get_width(),
                    color_buf.get_height(),
                );
            }
            _ => (),
        }
    });
}
