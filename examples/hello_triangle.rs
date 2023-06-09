#[macro_use]
extern crate softrender_derive;

use glam::Vec3;
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

// This struct defines the results of our vertex shader that will then be passed in as inputs to the
// fragment shader. Vertex outputs will be interpolated on a per-fragment basis by the rendering pipeline,
// using barycentric coordinates.
// These structs MUST derive the Barycentric trait.
#[derive(Barycentric)]
struct VertexOut {
    color: glam::Vec3,
}

// This struct defined our input vertices, also known as vertex attributes.
struct Vertex {
    pos: glam::Vec3,
    color: glam::Vec3,
}

// We define and implement a shader program here.
// It requires two functions, one for each type of mandatory shader.
// The vertex shader takes in one set of vertex attributes, and outputs at least
// one vec4, and optionally any number of output parameters.
// The fragment shader takes in the interpolated result from the vertex shader, and outputs a single color.
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
    // Construct winit window
    let mut event_loop = EventLoop::new();
    let window = WindowBuilder::new()
        .with_inner_size(LogicalSize::new(800, 800))
        .with_resizable(false)
        .build(&event_loop)
        .expect("Failed to initialize window");
    let mut gc = unsafe { GraphicsContext::new(&window, &window).expect("Failed to create GC") };

    // Create our renderer, as well as an example shader.
    let mut renderer = Renderer::new(800, 800);
    let mut shader = MyShader {};

    // Build the buffer data for our triangle
    let mut vertices = Vec::new();
    vertices.push(Vertex {
        pos: Vec3::new(0.5, -0.5, 0.0),
        color: Vec3::new(255.0, 0.0, 0.0),
    });
    vertices.push(Vertex {
        pos: Vec3::new(0.0, 0.5, 0.0),
        color: Vec3::new(0.0, 255.0, 0.0),
    });
    vertices.push(Vertex {
        pos: Vec3::new(-0.5, -0.5, 0.0),
        color: Vec3::new(0.0, 0.0, 255.0),
    });
    let indices = [0, 1, 2];

    // Winit loop
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
                // Render a frame.
                renderer.clear_framebuffer(95 | 95 << 8 | 95 << 16);
                // Each call to draw represents one invocation of the render pipeline.
                // You can perform many calls per frame, with any combination of shaders, vertices, and indices.
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
