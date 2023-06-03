use crate::fb::Framebuffer;
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

    pub fn draw(&mut self) -> bool {
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
            // TODO: Actually use a "Shader" object here for programmable shaders. It will take in a Vec3 and
            // output a Vec4 (homogenous with w component)
            // Right now, all the "vertex shader" does is is extend the vertex to a Vec4.
            let mut triangle_hom = [
                triangle_pos[0].extend(1.0),
                triangle_pos[1].extend(1.0),
                triangle_pos[2].extend(1.0),
            ];

            // After the vertex shader is run, our vertices now exist in clip space.
            // TODO: Clip vertices here.

            // After clipping the vertices, we can now perform a perspective divide
            triangle_hom[0] = (triangle_hom[0].xyz() / triangle_hom[0].w).extend(triangle_hom[0].w);
            triangle_hom[1] = (triangle_hom[1].xyz() / triangle_hom[1].w).extend(triangle_hom[1].w);
            triangle_hom[2] = (triangle_hom[2].xyz() / triangle_hom[2].w).extend(triangle_hom[2].w);

            // Finally, convert from ndc to screenspace
            let triangle_screen = [
                self.screenspace_matrix * triangle_hom[0],
                self.screenspace_matrix * triangle_hom[1],
                self.screenspace_matrix * triangle_hom[2],
            ];

            // TODO: Depth buffer

            // TODO: Fragment shader
            match self.draw_mode {
                DrawMode::REGULAR => todo!(),
                DrawMode::WIREFRAME => {
                    // Three lines per triangle
                    for j in 0..3 {
                        let ssp1 =
                            IVec2::new(triangle_screen[j].x as i32, triangle_screen[j].y as i32);
                        let ssp2 = IVec2::new(
                            triangle_screen[(j + 1) % 3].x as i32, // mod so we wrap back to the zero element
                            triangle_screen[(j + 1) % 3].y as i32,
                        );
                        // Plot to color buffer
                        self.plot_line(ssp1, ssp2, 0);
                    }
                }
            }
        }

        // Flush to screen
        self.fb.flush();
        true
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
