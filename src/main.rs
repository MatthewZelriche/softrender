use winit::dpi::LogicalSize;
use winit::event::{Event, WindowEvent};
use winit::event_loop::EventLoop;
use winit::platform::run_return::EventLoopExtRunReturn;
use winit::window::WindowBuilder;

mod fb;
mod fb_winit;
mod renderer;
use fb_winit::WinitFB;
use renderer::Renderer;

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

    let (models, _) = tobj::load_obj("african_head.obj", &tobj::LoadOptions::default())
        .expect("Could not load model.");

    let mut renderer = Renderer::new(fb);
    renderer.bind_vertex_data(&models[0].mesh.positions, &models[0].mesh.indices);

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
                renderer.clear_color(95 | 95 << 8 | 95 << 16);
                renderer.draw();
            }
            _ => (),
        }
    });
}
