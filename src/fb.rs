pub trait Framebuffer {
    fn fill(&mut self, color: u32);
    fn plot_pixel(&mut self, x: u16, y: u16, color: u32);
    fn flush(&mut self);
    fn resize(&mut self, x: u16, y: u16);
    fn get_width(&self) -> u16;
    fn get_height(&self) -> u16;
}
