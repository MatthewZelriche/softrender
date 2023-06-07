use crate::{
    fb::Framebuffer,
    shader::{Barycentric, Shader},
};

use glam::{vec4, IVec2, Mat4, Vec2Swizzles, Vec3, Vec4Swizzles};
use unzip_array_of_tuple::unzip_array_of_tuple;

struct BoundingBox2D {
    origin: IVec2,
    width: i32,
    height: i32,
}

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

pub struct Renderer<T: Framebuffer> {
    fb: T,
    draw_mode: DrawMode,
    screenspace_matrix: Mat4,
}

// TODO: Determine how stateful this renderer should be. Store state, or pass as args to draw call?
impl<T: Framebuffer> Renderer<T> {
    pub fn new(default_fb: T) -> Self {
        let a = calculate_screenspace_matrix(
            default_fb.get_width() as f32,
            default_fb.get_height() as f32,
        );
        Renderer {
            fb: default_fb,
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

    pub fn draw<S: Shader<Vertex, VI>, Vertex, VI: Barycentric>(
        &mut self,
        shader: &mut S,
        vbo: &[Vertex],
        ibo: &[u32],
    ) -> bool {
        // Rough draft of the pipeline. Will likely change.
        // TODO: Multithreading

        // Each triangle will always have 3 indices/vertices
        for i in (0..ibo.len()).step_by(3) {
            let v0_idx = ibo[i] as usize;
            let v1_idx = ibo[i + 1] as usize;
            let v2_idx = ibo[i + 2] as usize;

            let (mut clip_pos, varyings) = unzip_array_of_tuple([
                shader.vertex(&vbo[v0_idx]),
                shader.vertex(&vbo[v1_idx]),
                shader.vertex(&vbo[v2_idx]),
            ]);

            // After the vertex shader is run, our vertices now exist in clip space.
            // TODO: Clip vertices here.

            // After clipping the vertices, we can now perform a perspective divide
            clip_pos[0] = (clip_pos[0].xyz() / clip_pos[0].w).extend(clip_pos[0].w);
            clip_pos[1] = (clip_pos[1].xyz() / clip_pos[1].w).extend(clip_pos[1].w);
            clip_pos[2] = (clip_pos[2].xyz() / clip_pos[2].w).extend(clip_pos[2].w);

            // Finally, convert from ndc to screenspace
            let screen_p0 = (self.screenspace_matrix * clip_pos[0]).xy().as_ivec2();
            let screen_p1 = (self.screenspace_matrix * clip_pos[1]).xy().as_ivec2();
            let screen_p2 = (self.screenspace_matrix * clip_pos[2]).xy().as_ivec2();

            // TODO: Depth buffer

            match self.draw_mode {
                DrawMode::REGULAR => {
                    self.plot_triangle(screen_p0, screen_p1, screen_p2, shader, &varyings);
                }
                DrawMode::WIREFRAME => {
                    self.plot_line(screen_p0, screen_p1, shader, &varyings[0], &varyings[1]);
                    self.plot_line(screen_p1, screen_p2, shader, &varyings[1], &varyings[2]);
                    self.plot_line(screen_p2, screen_p0, shader, &varyings[2], &varyings[0]);
                }
            }
        }
        // Flush to screen
        self.fb.flush();
        true
    }

    fn tri_bounding_box(&self, p0: IVec2, p1: IVec2, p2: IVec2) -> BoundingBox2D {
        let min_x = p0.x.min(p1.x.min(p2.x));
        let max_x = p0.x.max(p1.x.max(p2.x));

        let min_y = p0.y.min(p1.y.min(p2.y));
        let max_y = p0.y.max(p1.y.max(p2.y));

        BoundingBox2D {
            origin: IVec2 { x: min_x, y: min_y },
            width: max_x - min_x,
            height: max_y - min_y,
        }
    }

    fn tri_area_signed(&self, p0: IVec2, p1: IVec2, p2: IVec2) -> i32 {
        (p1 - p0).perp_dot(p2 - p0) / 2
    }

    fn plot_triangle<S: Shader<V, VI>, V, VI: Barycentric>(
        &mut self,
        p0: IVec2,
        p1: IVec2,
        p2: IVec2,
        program: &mut S,
        program_inputs: &[VI; 3],
    ) {
        // Ignore colinear triangles
        // Why is this necessary? Why would a mesh ever have colinear/degenerate triangles?
        let area = self.tri_area_signed(p0, p1, p2);
        if area == 0 {
            return;
        }

        let bb = self.tri_bounding_box(p0, p1, p2);
        for y in bb.origin.y..=bb.origin.y + bb.height {
            for x in bb.origin.x..=bb.origin.x + bb.width {
                let pix = IVec2 { x, y };

                // Geometrically, we attempt to divide our primitive into three "subtriangles" all converging
                // at a given pixel. If all three subtriangles have a counter-clockwise winding order,
                // then the areas of all three triangles will be positive and this means the pixel lies
                // within the primitive. If any of the subtriangle areas are negative, the winding order
                // for that subtriangle is positive and the pixel must lie outside our primitive.
                let a = self.tri_area_signed(p0, p1, pix);
                let b = self.tri_area_signed(p1, p2, pix);
                let c = self.tri_area_signed(p2, p0, pix);

                if a >= 0 && b >= 0 && c >= 0 {
                    // Calculate barycentric coords for this pixel and inform the shader=
                    let barycentric_coords = Vec3::new(
                        b as f32 / area as f32,
                        c as f32 / area as f32,
                        a as f32 / area as f32,
                    );

                    // Run fragment shader
                    let interpolated = program_inputs[0].interpolated(
                        barycentric_coords,
                        &program_inputs[1],
                        &program_inputs[2],
                    );
                    let frag_output = program.fragment(interpolated);
                    let fb_color = frag_output.z | (frag_output.y << 8) | (frag_output.x << 16);
                    self.fb.plot_pixel(x as u16, y as u16, fb_color);
                }
            }
        }
    }

    fn plot_line<S: Shader<V, VI>, V, VI: Barycentric>(
        &mut self,
        mut p1: IVec2,
        mut p2: IVec2,
        program: &S,
        p1_input: &VI,
        p2_input: &VI,
    ) {
        // Skip points
        if p1 == p2 {
            // TODO: What should we really do to handle this?
            return;
        }

        // Determine the "Driving Axis", that is, whether the line is more vertical or horizontal
        // If driving axis is Y-axis, we need to flip so that we are iterating 1 per row instead of column
        let y_long = (p1.y - p2.y).abs() > (p1.x - p2.x).abs();
        if y_long {
            p1 = p1.yx();
            p2 = p2.yx();
        }

        // Save a copy of the original points before we potentially swap them, so that
        // barycentric coordinates work correctly.
        let p1_orig = p1;
        let p2_orig = p2;

        // The X-coordinate in our points now acts as the coordinate of the driving axis, regardless of what
        // axis it is in ndc. We need to ensure p1 always comes "before" p2 on the driving axis, to ensure
        // our for loop runs independently of ordering of the two points, so we re-order the points if
        // necessary.
        if p1.x > p2.x {
            std::mem::swap(&mut p1, &mut p2);
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
            // Barycentric coordinates for a line: treat it like an edge on a triangle
            // Basically, we just lerp between x and y, and set z to 0
            let pixel = IVec2::new(x, y);
            let barycentric_y =
                (pixel - p1_orig).as_vec2().length() / (p2_orig - p1_orig).as_vec2().length();

            let barycentric_coords = Vec3::new(1.0 - barycentric_y, barycentric_y, 0.0);

            // We pass p2_input again for the third argument because we know it will be zeroes out
            // by barycentric z coordinate, so its value is irrelevant
            let interpolated = p1_input.interpolated(barycentric_coords, p2_input, p2_input);

            let frag_output = program.fragment(interpolated);
            let fb_color = frag_output.z | (frag_output.y << 8) | (frag_output.x << 16);
            if y_long {
                // Swap back to screen-space
                self.fb.plot_pixel(y as u16, x as u16, fb_color);
            } else {
                // x and y are already in screen-space
                self.fb.plot_pixel(x as u16, y as u16, fb_color);
            }

            if eps >= 0 {
                y += sign;

                eps -= dx;
            }
            eps += dy_abs;
        }
    }
}
