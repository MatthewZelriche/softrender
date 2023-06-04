use winit::dpi::LogicalSize;
use winit::event::{Event, WindowEvent};
use winit::event_loop::EventLoop;
use winit::platform::run_return::EventLoopExtRunReturn;
use winit::window::WindowBuilder;

mod fb;
mod fb_winit;
mod renderer;
mod shader;
use fb_winit::WinitFB;
use renderer::{DrawMode, Renderer};
use shader::Shader;

struct MyShader;

impl Shader for MyShader {
    fn vertex(&self, pos: glam::Vec3) -> glam::Vec4 {
        pos.extend(1.0)
    }

    fn fragment(&self) -> glam::UVec3 {
        glam::UVec3::new(0, 0, 0)
    }
}

fn main() {
    let mut event_loop = EventLoop::new();
    let window = WindowBuilder::new()
        .with_inner_size(LogicalSize::new(800, 800))
        .with_resizable(false)
        .build(&event_loop)
        .expect("Failed to initialize window");

    let start_size = window.inner_size();
    let fb = WinitFB::new(start_size.width as u16, start_size.height as u16, &window)
        .expect("Failed to initialize framebuffer");

    let load_opt = tobj::GPU_LOAD_OPTIONS;
    let (models, _) = tobj::load_obj("african_head.obj", &load_opt).expect("Could not load model.");

    let mut renderer = Renderer::new(fb);
    renderer.set_draw_mode(DrawMode::WIREFRAME);
    renderer.bind_vertex_data(&models[0].mesh.positions, &models[0].mesh.indices);

    let shader = MyShader {};

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
                renderer.draw(&shader);

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
