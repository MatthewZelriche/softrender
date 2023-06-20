#[macro_use]
extern crate softrender_derive;

use glam::{Affine3A, Mat4, UVec3, Vec3};
use softbuffer::GraphicsContext;
use std::iter::zip;
use util::camera::Camera;
use winit::dpi::LogicalSize;
use winit::event::{Event, WindowEvent};
use winit::event_loop::EventLoop;
use winit::platform::run_return::EventLoopExtRunReturn;
use winit::window::WindowBuilder;

use softrender::{
    renderer::Renderer,
    shader::{Barycentric, Shader},
};

mod util;

#[derive(Clone, Barycentric)]
struct VertexOut {
    normal: glam::Vec3,
    frag_pos: glam::Vec3,
}

struct Vertex {
    pos: glam::Vec3,
    normal: glam::Vec3,
}

struct MyShader {
    // Fields in your shader can act as bound uniforms
    view_proj: Mat4,
    model_mat: Affine3A,
    light_pos: Vec3,
}
impl Shader<Vertex, VertexOut> for MyShader {
    fn vertex(&self, vertex: &Vertex) -> (glam::Vec4, VertexOut) {
        let original_vertex_pos = vertex.pos;
        (
            self.view_proj * self.model_mat * vertex.pos.extend(1.0),
            VertexOut {
                normal: (self.model_mat.inverse().matrix3.transpose() * vertex.normal).normalize(),
                frag_pos: original_vertex_pos,
            },
        )
    }

    fn fragment(&self, inputs: VertexOut) -> glam::UVec3 {
        let obj_col = UVec3 {
            x: 200,
            y: 200,
            z: 200,
        };

        // Calculations are performed in a normalized range, then scaled to 0-255.
        // TODO: Consider returning a value between 0-1 instead of 0-255?
        let light_color = Vec3::new(1.0, 1.0, 1.0);

        let ambient_intensity = 0.2;
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
        .with_inner_size(LogicalSize::new(800, 800))
        .with_resizable(false)
        .build(&event_loop)
        .expect("Failed to initialize window");
    let mut gc = unsafe { GraphicsContext::new(&window, &window).expect("Failed to create GC") };

    let load_opt = tobj::GPU_LOAD_OPTIONS;
    let (models, _) = tobj::load_obj("res/teapot.obj", &load_opt).expect("Could not load model.");

    let mut vertices = Vec::new();

    let pos_data = &models[0].mesh.positions;
    let normal_data = &models[0].mesh.normals;

    for (pos, normal) in zip(pos_data.chunks(3), normal_data.chunks(3)) {
        vertices.push(Vertex {
            pos: glam::Vec3::from_slice(pos),
            normal: Vec3::from_slice(normal),
        });
    }

    let indices = &models[0].mesh.indices;

    let window_size = window.inner_size();
    let mut renderer = Renderer::new(window_size.width as u16, window_size.height as u16);
    let fov = 50.0;

    let cam = Camera::new(
        f32::to_radians(fov),
        window_size.width as f32 / window_size.height as f32,
        0.1,
        50.0,
        Vec3::new(0.0, 0.2, 3.5),
    );

    let mut shader = MyShader {
        view_proj: cam.view_projection_matrix(),
        model_mat: Affine3A::from_rotation_x(25.0f32.to_radians())
            * Affine3A::from_rotation_y(10.0f32.to_radians())
            * Affine3A::from_scale(Vec3::new(0.4, 0.4, 0.4)),
        light_pos: Vec3::new(0.0, 0.0, 5.0),
    };

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
                renderer.clear_framebuffer(50 | 50 << 8 | 50 << 16);
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
