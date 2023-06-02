use crate::fb::Framebuffer;
use std::option::Option;

use glam::{IVec2, Vec2Swizzles};

pub struct Renderer<'a, T: Framebuffer> {
    fb: T,
    vertex_buf: Option<&'a [f32]>,
    index_buf: Option<&'a [u32]>,
}

impl<'a, T: Framebuffer> Renderer<'a, T> {
    pub fn new(default_fb: T) -> Self {
        Renderer {
            fb: default_fb,
            vertex_buf: None,
            index_buf: None,
        }
    }

    pub fn set_fb_size(&mut self, width: u16, height: u16) {
        self.fb.resize(width, height);
    }

    pub fn clear_color(&mut self, new_color: u32) {
        self.fb.fill(new_color);
    }

    pub fn bind_vertex_data(&mut self, vertex_buf_in: &'a [f32], index_buf_in: &'a [u32]) {
        self.vertex_buf = Some(vertex_buf_in);
        self.index_buf = Some(index_buf_in);
    }

    pub fn unbind_vertex_data(&mut self) {
        self.vertex_buf = None;
        self.index_buf = None;
    }

    pub fn draw(&mut self) {
        // TODO: pipeline invocation

        self.plot_line(IVec2 { x: 0, y: 100 }, IVec2 { x: 799, y: 100 }, 0);
        self.plot_line(IVec2 { x: 100, y: 0 }, IVec2 { x: 100, y: 799 }, 0);
        self.plot_line(IVec2 { x: 0, y: 0 }, IVec2 { x: 799, y: 799 }, 0);
        self.plot_line(IVec2 { x: 0, y: 799 }, IVec2 { x: 799, y: 0 }, 0);
        self.plot_line(IVec2 { x: 400, y: 0 }, IVec2 { x: 450, y: 800 }, 0);
        self.plot_line(IVec2 { x: 400, y: 800 }, IVec2 { x: 450, y: 0 }, 0);
        self.plot_line(IVec2 { x: 0, y: 400 }, IVec2 { x: 800, y: 450 }, 0);
        self.plot_line(IVec2 { x: 800, y: 400 }, IVec2 { x: 0, y: 450 }, 0);

        // Flush to screen
        self.fb.flush();
    }

    fn plot_line(&mut self, mut p1: IVec2, mut p2: IVec2, color: u32) {
        // Special case for a "line" thats a single point
        if p1 == p2 {
            self.fb.plot_pixel(p1.x as u16, p1.y as u16, color);
            return;
        }

        // Determine the "Driving Axis", that is, whether the line is more vertical or horizontal
        // If driving axis is Y-axis, we need to flip so that we are iterating 1 per row instead of column
        let y_long = (p1.y - p2.y).abs() > (p1.x - p2.x).abs();
        if y_long {
            p1 = p1.yx();
            p2 = p2.yx();
        }
        // The X-coordinate in our points now acts as the coordinate of the driving axis, regardless of what
        // axis it is in ndc. We need to ensure p1 always comes "before" p2 on the driving axis, to ensure
        // our for loop runs independently of ordering of the two points, so we re-order the points if
        // necessary.
        if p1.x > p2.x {
            let temp = p1;
            p1 = p2;
            p2 = temp;
        }

        let dx = p2.x - p1.x;
        let dy = p2.y - p1.y;
        let dy_abs = dy.abs();
        let mut eps = dy_abs - dx.abs();
        let mut y = p1.y;

        // Whether we increment or decrement the screen-space y coordinate depends on the sign of
        // the line's dy. This is checked ahead of time to avoid an additional branch in the for loop.
        let sign = if dy >= 0 { 1 } else { -1 };

        for x in p1.x..p2.x {
            if y_long {
                // Swap back to screen-space
                self.fb.plot_pixel(y as u16, x as u16, color);
            } else {
                // x and y are already in screen-space
                self.fb.plot_pixel(x as u16, y as u16, color);
            }

            if eps >= 0 {
                y += 1 * sign;

                eps -= dx;
            }
            eps += dy_abs;
        }
    }
}
