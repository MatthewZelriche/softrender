use softbuffer::GraphicsContext;

use winit::window::Window;

use crate::fb::Framebuffer;
use std::vec::Vec;

pub struct WinitFB {
    width: u16,
    height: u16,
    gc: GraphicsContext,
    buf: Vec<u32>,
}

impl WinitFB {
    pub fn new(width: u16, height: u16, handle: &Window) -> Result<Self, String> {
        let gc_res = unsafe { GraphicsContext::new(&handle, &handle) };
        match gc_res {
            Ok(gc) => Ok(WinitFB {
                width,
                height,
                gc,
                buf: vec![0u32; width as usize * height as usize],
            }),
            Err(err) => Err(std::format!(
                "Error constructing underlying software buffer: {}",
                err.to_string()
            )),
        }
    }
}

impl Framebuffer for WinitFB {
    fn fill(&mut self, color: u32) {
        for pixel in &mut self.buf {
            *pixel = color;
        }
    }

    fn get_width(&self) -> u16 {
        self.width
    }

    fn get_height(&self) -> u16 {
        self.height
    }

        let idx = y as usize * self.width as usize + x as usize;
        self.buf[idx] = color;
    }

    fn flush(&mut self) {
        self.gc
            .set_buffer(self.buf.as_slice(), self.width, self.height);
    }

    fn resize(&mut self, new_width: u16, new_height: u16) {
        self.width = new_width;
        self.height = new_height;
        let new_size = self.width as usize * self.height as usize;
        self.buf.resize(new_size, 0);
    }
}
