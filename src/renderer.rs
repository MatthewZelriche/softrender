use crate::{fb::Framebuffer, shader::Shader};
use std::option::Option;

use glam::{vec4, IVec2, Mat4, Vec2Swizzles, Vec3, Vec4Swizzles};

pub enum DrawMode {
    REGULAR,
    WIREFRAME,
}

fn calculate_screenspace_matrix(width: f32, height: f32) -> Mat4 {
    let max_cols = width - 1.0;
    let max_rows = height - 1.0;
    Mat4::from_cols(
        vec4(max_cols / 2.0, 0.0, 0.0, 0.0),
        vec4(0.0, max_rows / 2.0, 0.0, 0.0),
        vec4(0.0, 0.0, 1.0, 0.0),
        vec4(max_cols / 2.0, max_rows / 2.0, 0.0, 1.0),
    )
}

pub struct Renderer<'a, T: Framebuffer> {
    fb: T,
    vertex_buf: Option<&'a [f32]>,
    index_buf: Option<&'a [u32]>,
    draw_mode: DrawMode,
    screenspace_matrix: Mat4,
}

// TODO: Determine how stateful this renderer should be. Store state, or pass as args to draw call?
impl<'a, T: Framebuffer> Renderer<'a, T> {
    pub fn new(default_fb: T) -> Self {
        let a = calculate_screenspace_matrix(
            default_fb.get_width() as f32,
            default_fb.get_height() as f32,
        );
        Renderer {
            fb: default_fb,
            vertex_buf: None,
            index_buf: None,
            draw_mode: DrawMode::REGULAR,
            screenspace_matrix: a,
        }
    }

    pub fn set_fb_size(&mut self, width: u16, height: u16) {
        self.fb.resize(width, height);
        self.screenspace_matrix = calculate_screenspace_matrix(width as f32, height as f32);
    }

    pub fn clear_color(&mut self, new_color: u32) {
        self.fb.fill(new_color);
    }

    pub fn set_draw_mode(&mut self, new_mode: DrawMode) {
        self.draw_mode = new_mode;
    }

    pub fn bind_vertex_data(&mut self, vertex_buf_in: &'a [f32], index_buf_in: &'a [u32]) {
        self.vertex_buf = Some(vertex_buf_in);
        self.index_buf = Some(index_buf_in);
    }

    pub fn unbind_vertex_data(&mut self) {
        self.vertex_buf = None;
        self.index_buf = None;
    }

    pub fn draw<S: Shader>(&mut self, shader: &S) -> bool {
        // Rough draft of the pipeline. Will likely change.
        // TODO: Multithreading
        if self.vertex_buf == None || self.index_buf == None {
            return false;
        }

        // Each triangle will always have 3 indices/vertices
        for i in (0..self.index_buf.unwrap().len()).step_by(3) {
            let v0_idx = self.index_buf.unwrap()[i] as usize;
            let v1_idx = self.index_buf.unwrap()[i + 1] as usize;
            let v2_idx = self.index_buf.unwrap()[i + 2] as usize;

            // TODO: Right now we assume vertex stride is 3 floats, but that will change.
            let stride = 3;
            let triangle_pos = [
                Vec3::from_slice(self.vertex_buf.unwrap().get(v0_idx * stride..).unwrap()),
                Vec3::from_slice(self.vertex_buf.unwrap().get(v1_idx * stride..).unwrap()),
                Vec3::from_slice(self.vertex_buf.unwrap().get(v2_idx * stride..).unwrap()),
            ];

            // Apply the vertex shader to each vertex in the primitive
            // TODO: Consider some way to not process a single vertex multiple times due to using indices?
            // Right now, all the "vertex shader" does is is extend the vertex to a Vec4.
            let mut triangle_hom = [
                shader.vertex(triangle_pos[0]),
                shader.vertex(triangle_pos[1]),
                shader.vertex(triangle_pos[2]),
            ];

            // After the vertex shader is run, our vertices now exist in clip space.
            // TODO: Clip vertices here.

            // After clipping the vertices, we can now perform a perspective divide
            triangle_hom[0] = (triangle_hom[0].xyz() / triangle_hom[0].w).extend(triangle_hom[0].w);
            triangle_hom[1] = (triangle_hom[1].xyz() / triangle_hom[1].w).extend(triangle_hom[1].w);
            triangle_hom[2] = (triangle_hom[2].xyz() / triangle_hom[2].w).extend(triangle_hom[2].w);

            // Finally, convert from ndc to screenspace
            let screen_p0 = (self.screenspace_matrix * triangle_hom[0]).xy().as_ivec2();
            let screen_p1 = (self.screenspace_matrix * triangle_hom[1]).xy().as_ivec2();
            let screen_p2 = (self.screenspace_matrix * triangle_hom[2]).xy().as_ivec2();

            // TODO: Depth buffer

            // TODO: Interpolate vertex shader outputs for fragment shader inputs

            match self.draw_mode {
                DrawMode::REGULAR => {
                    self.plot_triangle(screen_p0, screen_p1, screen_p2, shader);
                }
                DrawMode::WIREFRAME => {
                    self.plot_line(screen_p0, screen_p1, shader);
                    self.plot_line(screen_p1, screen_p2, shader);
                    self.plot_line(screen_p2, screen_p0, shader);
                }
            }
        }

        // Flush to screen
        self.fb.flush();
        true
    }

    fn tri_area_signed(&self, p0: IVec2, p1: IVec2, p2: IVec2) -> i32 {
        (p1 - p0).perp_dot(p2 - p0) / 2
    }

    fn plot_triangle<S: Shader>(&mut self, p0: IVec2, p1: IVec2, p2: IVec2, program: &S) {
        // Ignore colinear triangles
        // Why is this necessary? Why would a mesh ever have colinear/degenerate triangles?
        if self.tri_area_signed(p0, p1, p2) == 0 {
            return;
        }

        // TODO: Calculate bounding box, so we aren't naively checking the entire screen for every primitive
        for y in 0..self.fb.get_height() {
            for x in 0..self.fb.get_width() {
                let pix = IVec2 {
                    x: x as i32,
                    y: y as i32,
                };

                // TODO: Calc barycentric coords
                let a = self.tri_area_signed(p0, p1, pix) >= 0;
                let b = self.tri_area_signed(p1, p2, pix) >= 0;
                let c = self.tri_area_signed(p2, p0, pix) >= 0;

                if a && b && c {
                    let frag_output = program.fragment();
                    let fb_color = frag_output.z | (frag_output.y << 8) | (frag_output.x << 16);
                    self.fb.plot_pixel(x, y, fb_color);
                }
            }
        }
    }

    fn plot_line<S: Shader>(&mut self, mut p1: IVec2, mut p2: IVec2, program: &S) {
        // Special case for a "line" thats a single point
        if p1 == p2 {
            let frag_output = program.fragment();
            let fb_color = frag_output.z | (frag_output.y << 8) | (frag_output.x << 16);
            self.fb.plot_pixel(p1.x as u16, p1.y as u16, fb_color);
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
            let frag_output = program.fragment();
            let fb_color = frag_output.z | (frag_output.y << 8) | (frag_output.x << 16);
            if y_long {
                // Swap back to screen-space
                self.fb.plot_pixel(y as u16, x as u16, fb_color);
            } else {
                // x and y are already in screen-space
                self.fb.plot_pixel(x as u16, y as u16, fb_color);
            }

            if eps >= 0 {
                y += 1 * sign;

                eps -= dx;
            }
            eps += dy_abs;
        }
    }
}
