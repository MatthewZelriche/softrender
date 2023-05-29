use winit::dpi::LogicalSize;
use winit::event::{Event, WindowEvent};
use winit::event_loop::EventLoop;
use winit::window::WindowBuilder;

mod fb;
mod fb_winit;
use fb::Framebuffer;
use fb_winit::WinitFB;

fn main() {
    let event_loop = EventLoop::new();
    let window = WindowBuilder::new()
        .with_inner_size(LogicalSize::new(800, 800))
        .with_resizable(false)
        .build(&event_loop)
        .expect("Failed to initialize window");

    let start_size = window.inner_size();
    let mut fb = WinitFB::new(start_size.width as u16, start_size.height as u16, &window)
        .expect("Failed to initialize framebuffer");

    let bg_color = 255 | 0 << 8 | 0 << 16;

    event_loop.run(move |event, _, cf| {
        cf.set_poll();

        match event {
            Event::WindowEvent {
                event: window_event,
                ..
            } => match window_event {
                WindowEvent::CloseRequested => cf.set_exit(),
                WindowEvent::Resized(inner_size) => {
                    fb.resize(inner_size.width as u16, inner_size.height as u16)
                }
                _ => (),
            },

            Event::MainEventsCleared => {
                // TODO: Move this into renderer struct render()
                // Blank the screen
                fb.fill(bg_color);

                // TODO: Rendering

                // Flush to screen
                fb.flush();
            }
            _ => (),
        }
    });
}
