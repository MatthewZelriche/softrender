use crate::fb::Framebuffer;

pub struct Renderer<T: Framebuffer> {
    bg_color: u32,
    fb: T,
}

impl<T: Framebuffer> Renderer<T> {
    pub fn new(default_fb: T) -> Self {
        Renderer {
            fb: default_fb,
            bg_color: 0,
        }
    }

    pub fn set_fb_size(&mut self, width: u16, height: u16) {
        self.fb.resize(width, height);
    }

    pub fn set_bg_color(&mut self, new_color: u32) {
        self.bg_color = new_color;
    }

    pub fn draw_frame(&mut self) {
        // Blank the screen
        self.fb.fill(self.bg_color);

        // TODO: Rendering

        // Flush to screen
        self.fb.flush();
    }
}
