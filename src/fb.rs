pub trait Framebuffer<T> {
    fn fill(&mut self, value: T);
    fn plot_pixel(&mut self, x: u16, y: u16, value: T);
    fn resize(&mut self, new_width: u16, new_height: u16, default: T);
    fn get_width(&self) -> u16;
    fn get_height(&self) -> u16;
}

pub trait Flushable {
    fn flush(&mut self);
}
