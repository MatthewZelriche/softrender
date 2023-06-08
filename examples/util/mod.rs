use softbuffer::GraphicsContext;

use winit::window::Window;

use softrender::fb::{Flushable, Framebuffer};
use std::vec::Vec;

pub struct WinitFB<T: Clone> {
    width: u16,
    height: u16,
    gc: GraphicsContext,
    buf: Vec<T>,
}

impl<T: Clone> WinitFB<T> {
    pub fn new(width: u16, height: u16, handle: &Window, default: T) -> Result<Self, String> {
        let gc_res = unsafe { GraphicsContext::new(&handle, &handle) };
        match gc_res {
            Ok(gc) => Ok(WinitFB {
                width,
                height,
                gc,
                buf: vec![default; width as usize * height as usize],
            }),
            Err(err) => Err(std::format!(
                "Error constructing underlying software buffer: {}",
                err.to_string()
            )),
        }
    }
}

impl<T: Clone + Copy> Framebuffer<T> for WinitFB<T> {
    fn fill(&mut self, value: T) {
        for pixel in &mut self.buf {
            *pixel = value;
        }
    }

    fn get_width(&self) -> u16 {
        self.width
    }

    fn get_height(&self) -> u16 {
        self.height
    }

    fn plot_pixel(&mut self, x: u16, mut y: u16, value: T) {
        // Calculate the inverse of the y coordinate, because softbuffer has topleft as origin,
        // but we need bottom left.
        y = (self.width - 1) - y;
        let idx = y as usize * self.width as usize + x as usize;
        self.buf[idx] = value;
    }

    fn resize(&mut self, new_width: u16, new_height: u16, default: T) {
        self.width = new_width;
        self.height = new_height;
        let new_size = self.width as usize * self.height as usize;
        self.buf.resize(new_size, default);
    }
}

impl Flushable for WinitFB<u32> {
    fn flush(&mut self) {
        self.gc
            .set_buffer(self.buf.as_slice(), self.width, self.height);
    }
}
