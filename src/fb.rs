pub struct Framebuffer<T> {
    width: u16,
    height: u16,
    buf: Vec<T>,
}

impl<T: Default + Copy> Framebuffer<T> {
    pub fn new(width: u16, height: u16) -> Self {
        let buf = vec![T::default(); width as usize * height as usize];
        Framebuffer { width, height, buf }
    }

    pub fn fill(&mut self, value: T) {
        for pixel in &mut self.buf {
            *pixel = value;
        }
    }

    pub fn plot_pixel(&mut self, x: u16, mut y: u16, value: T) {
        // Invert y so that the start coordinate of the buffer is bottom left.
        y = (self.height - 1) - y;
        let idx = y as usize * self.width as usize + x as usize;
        self.buf[idx] = value;
    }

    pub fn get_pixel(&self, x: u16, mut y: u16) -> T {
        y = (self.height - 1) - y;
        let idx = y as usize * self.width as usize + x as usize;
        self.buf[idx]
    }

    pub fn resize(&mut self, new_width: u16, new_height: u16, default: T) {
        self.width = new_width;
        self.height = new_height;
        let new_size = self.width as usize * self.height as usize;
        self.buf.resize(new_size, default);
    }

    pub fn get_width(&self) -> u16 {
        self.width
    }

    pub fn get_height(&self) -> u16 {
        self.height
    }

    pub fn get_raw(&self) -> &[T] {
        self.buf.as_slice()
    }
}
