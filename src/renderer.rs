use crate::{
    fb::Framebuffer,
    math::{ClipPlane, InverseLerp},
    shader::{Barycentric, Shader},
};

use arrayvec::ArrayVec;
use glam::{vec4, IVec2, Mat4, Vec2, Vec2Swizzles, Vec3, Vec4, Vec4Swizzles};

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
        vec4(max_cols / 2.0, max_rows / 2.0, 1.0, 1.0),
    )
}

pub struct Renderer {
    cb: Framebuffer<u32>,
    db: Framebuffer<f32>,
    draw_mode: DrawMode,
    screenspace_matrix: Mat4,
}

// TODO: Determine how stateful this renderer should be. Store state, or pass as args to draw call?
impl Renderer {
    pub fn new(width: u16, height: u16) -> Self {
        let a = calculate_screenspace_matrix(width as f32, height as f32);
        Renderer {
            cb: Framebuffer::new(width, height),
            db: Framebuffer::new(width, height),
            draw_mode: DrawMode::REGULAR,
            screenspace_matrix: a,
        }
    }

    pub fn set_fb_size(&mut self, width: u16, height: u16) {
        self.cb.resize(width, height, 0);
        self.db.resize(width, height, 0.0);
        self.screenspace_matrix = calculate_screenspace_matrix(width as f32, height as f32);
    }

    pub fn clear_framebuffer(&mut self, new_color: u32) {
        // TODO: Allow specifying which to clear
        self.cb.fill(new_color);
        self.db.fill(1.0);
    }

    pub fn set_draw_mode(&mut self, new_mode: DrawMode) {
        self.draw_mode = new_mode;
    }

    pub fn draw<S: Shader<Vertex, VI>, Vertex, VI: Barycentric + Clone>(
        &mut self,
        shader: &mut S,
        vbo: &[Vertex],
        ibo: &[u32],
    ) -> &Framebuffer<u32> {
        // Rough draft of the pipeline. Will likely change.
        // TODO: Multithreading

        // Each triangle will always have 3 indices/vertices
        for i in (0..ibo.len()).step_by(3) {
            let v0_idx = ibo[i] as usize;
            let v1_idx = ibo[i + 1] as usize;
            let v2_idx = ibo[i + 2] as usize;

            // After the vertex shader is run, our vertices now exist in clip space.
            let final_tris = self.clip_triangle(
                shader.vertex(&vbo[v0_idx]),
                shader.vertex(&vbo[v1_idx]),
                shader.vertex(&vbo[v2_idx]),
            );

            // Now we iterate over every triangle in our fan for the rest of this original user primitive
            for j in 0..final_tris.len() {
                let mut clip_pos = [final_tris[j].0[0], final_tris[j].0[1], final_tris[j].0[2]];

                // After clipping the vertices, we can now perform a perspective divide
                clip_pos[0] = (clip_pos[0].xyz() / clip_pos[0].w).extend(clip_pos[0].w);
                clip_pos[1] = (clip_pos[1].xyz() / clip_pos[1].w).extend(clip_pos[1].w);
                clip_pos[2] = (clip_pos[2].xyz() / clip_pos[2].w).extend(clip_pos[2].w);

                // Finally, convert from ndc to screenspace
                // Homogenous component must be 1.0!
                let screen_p0 = (self.screenspace_matrix * clip_pos[0].xyz().extend(1.0)).xy();
                let screen_p1 = (self.screenspace_matrix * clip_pos[1].xyz().extend(1.0)).xy();
                let screen_p2 = (self.screenspace_matrix * clip_pos[2].xyz().extend(1.0)).xy();

                match self.draw_mode {
                    DrawMode::REGULAR => {
                        let clip_z = [clip_pos[0].z, clip_pos[1].z, clip_pos[2].z];
                        self.plot_triangle(
                            screen_p0,
                            screen_p1,
                            screen_p2,
                            &clip_z,
                            shader,
                            &final_tris[j].1,
                        );
                    }
                    DrawMode::WIREFRAME => {
                        self.plot_line(
                            screen_p0,
                            screen_p1,
                            shader,
                            &final_tris[j].1[0],
                            &final_tris[j].1[1],
                        );
                        self.plot_line(
                            screen_p1,
                            screen_p2,
                            shader,
                            &final_tris[j].1[1],
                            &final_tris[j].1[2],
                        );
                        self.plot_line(
                            screen_p2,
                            screen_p0,
                            shader,
                            &final_tris[j].1[2],
                            &final_tris[j].1[0],
                        );
                    }
                }
            }
        }

        // We've completed a drawcall into the framebuffer, present it to the user so they can
        // do whatever they need with it
        &self.cb
    }

    /// Clips a triangle primitive against the viewing frustum, using the homogenous coordinate w
    ///
    /// This algorithm uses an adaptation of the Sutherland-Hodgman algorithm to clip a triangle primitive
    /// against a viewing frustum generated by either a perspective or orthographic projection matrix.
    /// Since the input vertices are in clip space, determining whether a vertex falls within the viewing
    /// frustum can be efficiently calculated by checking if -w <= a <= w where a is the x, y, z components
    /// of the vector's clip-space position.
    /// When clipping occurs, the newly created vertex will receive properly interpolated attributes via
    /// barycentric coordinates of the line segment it was contained in.
    /// Clipping a triangle primitive can result in generating new vertices, resulting in a polygon with
    /// a maximum of six vertices. If this occurs, the algorithm will triangulate the polygon by building
    /// a triangle fan. As a result, a maximum of up to four triangles may be returned by this algorithm.
    ///
    /// # Arguments
    ///
    /// * v0 - A tuple containing the first vertex's (in counter-clockwise order) clip-space position
    ///        and its vertex attributes.
    /// * v1 - A tuple containing the second vertex's (in counter-clockwise order) clip-space position
    ///        and its vertex attributes.
    /// * v2 - A tuple containing the third vertex's (in counter-clockwise order) clip-space position
    ///        and its vertex attributes.
    ///
    /// # Returns
    ///
    /// A fixed-length array containing up to four triangles.
    fn clip_triangle<VI: Barycentric + Clone>(
        &self,
        v0: (Vec4, VI),
        v1: (Vec4, VI),
        v2: (Vec4, VI),
    ) -> ArrayVec<([Vec4; 3], [VI; 3]), 4> {
        let mut output_verts = ArrayVec::<_, 6>::new();
        output_verts.push(v0);
        output_verts.push(v1);
        output_verts.push(v2);
        let mut clip_plane = ClipPlane { sign: 1.0, axis: 2 };

        // View frustum has 6 planes - left, right, top, bottom, near, far.
        for clip_plane_idx in 0..6 {
            // The input vertices for each new clip plane should consist of the output vertices
            // of the previous iteration
            let input_verts = output_verts.clone();
            output_verts.clear();

            // Our clipping planes are mathematically represented by two things: An axis (X, Y, Z), and the
            // sign of the homogenous coordinate w, which tells us which plane we are testing for this axis.
            // For example, left and right both have normal vectors aligned on the X axis, but the sign of w
            // tells us which direction the normal vector is pointing (+X or -X).
            let inside_clip_plane = if clip_plane_idx % 2 == 0 {
                clip_plane.sign = -1.0;
                clip_plane.axis = (clip_plane.axis + 1) % 3; // We've handled -,+ for a single axis,
                                                             // move to next axis
                |w: f32, x: f32| -w <= x
            } else {
                clip_plane.sign = 1.0;
                |w: f32, x: f32| x <= w
            };

            // idx must be i8 as we are utilizing modulus arithmetic on negative values to wrap the
            // index for input_verts
            for vert_idx in 0i8..input_verts.len() as i8 {
                // Sutherland-Hodgman algorithm clips a polygon by considering the line segments that make up
                // its edges. We test each line segment making up this iteration of the polygon against
                // the clipping plane, and generate a new vertex where the line intersects with the plane,
                // if clipping is necessary
                let curr_idx = vert_idx as usize;
                let prev_idx = (vert_idx - 1).rem_euclid(input_verts.len() as i8) as usize;
                let (curr_pos, _) = input_verts[curr_idx];
                let (prev_pos, _) = input_verts[prev_idx];

                if inside_clip_plane(curr_pos[3], curr_pos[clip_plane.axis]) {
                    // Current point is inside the clip plane...
                    if !inside_clip_plane(prev_pos[3], prev_pos[clip_plane.axis]) {
                        // Current is inside, but prev is outside, so we have a verified
                        // intersection on this plane. Drop the vertex outside the clip plane and generate
                        // a new vertex directly at the intersection
                        let clipped_vertex = self.compute_clipping_intersection(
                            &input_verts[prev_idx],
                            &input_verts[curr_idx],
                            &clip_plane,
                        );
                        output_verts.push(clipped_vertex);
                    }
                    // Both points are inside this clipping plane, so we just have to push the current
                    // vertex as-is. No clipping necessary.
                    output_verts.push((curr_pos, input_verts[curr_idx].1.clone()));
                } else if inside_clip_plane(prev_pos[3], prev_pos[clip_plane.axis]) {
                    // Current point is outside, but prev is inside. We disregard curr and truncate
                    // this line segment to prev -> intersection
                    let clipped_vertex = self.compute_clipping_intersection(
                        &input_verts[prev_idx],
                        &input_verts[curr_idx],
                        &clip_plane,
                    );
                    output_verts.push(clipped_vertex);
                } else {
                    // Both points lay outside this clipping plane, we can discard this line entirely
                }
            }
        }

        let mut final_tris = ArrayVec::<([Vec4; 3], [VI; 3]), 4>::new();
        // If the Sutherland-Hodgman algorithm produced no vertices at all, the triangle was entirely
        // outside the viewing frustum, so we can just return an empty array of 0 new triangles.
        if output_verts.is_empty() {
            return final_tris;
        }
        // We have generated a set of vertices representing the new clipped polygon. The last step
        // is to build a triangle fan out of this polygon, since our rasterizer only works on triangles.
        let triangle_count = output_verts.len() - 2;
        for j in 0..triangle_count {
            final_tris.push((
                [
                    output_verts[0].0,
                    output_verts[j + 1].0,
                    output_verts[j + 2].0,
                ],
                [
                    output_verts[0].1.clone(),
                    output_verts[j + 1].1.clone(),
                    output_verts[j + 2].1.clone(),
                ],
            ));
        }

        final_tris
    }

    /// Computes the interpolated intersection point between a line segment and a clipping plane
    ///
    /// If the given line segment does not actually intersect the plane, the vertex returned will be
    /// extrapolated.
    ///
    /// # Arguments
    ///
    /// * from - The start vertex of the line segment, in clip space
    /// * to - The end vertex of the line segment, in clip space
    /// * plane - The clipping plane to test against
    ///
    /// # Returns
    ///
    /// A new vertex (with interpolated attributes) where the line segment intersects the clipping plane
    fn compute_clipping_intersection<VI: Barycentric>(
        &self,
        from: &(Vec4, VI),
        to: &(Vec4, VI),
        plane: &ClipPlane,
    ) -> (Vec4, VI) {
        let (to_pos, to_attrib) = to;
        let (from_pos, from_attrib) = from;

        // Perform an inverse lerp that factors in the fact that we have not yet performed
        // the perspective divide, due to the vertices being in clip space.
        let interp_val = (plane.sign * to_pos[3] - to_pos[plane.axis])
            / ((plane.sign * to_pos[3] - to_pos[plane.axis])
                - (plane.sign * from_pos[3] - from_pos[plane.axis]));
        // Find the clip space position where the line segment intersects with the plane
        let intersect_pos = to_pos.lerp(*from_pos, interp_val);
        // Perform a interpolation of the two vertices' attributes by using the line segment's
        // barycentric coordinates
        let intersect_attribs = self.tri_barycentric_interpolate_edge(
            from_pos.xy(),
            to_pos.xy(),
            intersect_pos.xy(),
            from_attrib,
            to_attrib,
        );

        (intersect_pos, intersect_attribs)
    }

    /// Interpolates vertex attributes of a line using barycentric coordinates
    ///
    /// # Arguments
    ///
    /// * from - The start vertex of the line segment, in clip space, compressed to two dimensions
    /// * to - The end vertex of the line segment, in clip space, compressed to two dimensions
    /// * point - The clip space position to find barycentric coordinates for
    /// * attrib1 - The vertex attributes belonging to the start of the line segment
    /// * attrib2 - The vertex attributes belonging to the end of the line segment
    ///
    /// # Returns
    ///
    /// A new set of interpolated vertex attributess
    fn tri_barycentric_interpolate_edge<VI: Barycentric>(
        &self,
        from: Vec2,
        to: Vec2,
        point: Vec2,
        attrib1: &VI,
        attrib2: &VI,
    ) -> VI {
        let barycentric_y = from.inverse_lerp(to, point);
        let mut barycentric_coords = Vec2::new(1.0 - barycentric_y, barycentric_y);
        barycentric_coords = barycentric_coords.clamp(Vec2::ZERO, Vec2::new(1.0, 1.0));
        attrib1.line_interpolated(barycentric_coords, attrib2)
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

    // Calculate edge equations with subpixel precision
    #[inline(always)]
    fn tri_area_signed_squared(&self, p0: Vec2, p1: Vec2, p2: Vec2) -> f32 {
        (p1 - p0).perp_dot(p2 - p0)
    }

    fn plot_triangle<S: Shader<V, VI>, V, VI: Barycentric>(
        &mut self,
        p0: Vec2,
        p1: Vec2,
        p2: Vec2,
        clip_z: &[f32; 3],
        program: &mut S,
        program_inputs: &[VI; 3],
    ) {
        let area = self.tri_area_signed_squared(p0, p1, p2);

        let bb = self.tri_bounding_box(p0.as_ivec2(), p1.as_ivec2(), p2.as_ivec2());

        // Determine the starting non-normalized barycentric values of our pixel iterations, in this case,
        // the bottom left corner of the triangle's bounding box
        let start_pix = Vec2 {
            x: bb.origin.x as f32,
            y: bb.origin.y as f32,
        };
        let dxa = p0.x - p1.x;
        let dya = p0.y - p1.y;
        let mut efa = self.tri_area_signed_squared(p0, p1, start_pix);
        let dxb = p1.x - p2.x;
        let dyb = p1.y - p2.y;
        let mut efb = self.tri_area_signed_squared(p1, p2, start_pix);
        let dxc = p2.x - p0.x;
        let dyc = p2.y - p0.y;
        let mut efc = self.tri_area_signed_squared(p2, p0, start_pix);

        for y in bb.origin.y..=bb.origin.y + bb.height {
            // Save the result of our edge function at the start of every row
            // for when we need to increment up a column
            let saved_efa = efa;
            let saved_efb = efb;
            let saved_efc = efc;
            for x in bb.origin.x..=bb.origin.x + bb.width {
                // TODO: Consider guarding attempts to access memory outside the screen
                // Currently, this should never happen due to clipping, but if we choose
                // to use guard-band clipping it may become necessary.

                // Geometrically, we attempt to divide our primitive into three "subtriangles" all converging
                // at a given pixel. If all three subtriangles have a counter-clockwise winding order,
                // then the areas of all three triangles will be positive and this means the pixel lies
                // within the primitive. If any of the subtriangle areas are negative, the winding order
                // for that subtriangle is positive and the pixel must lie outside our primitive.
                if efa >= 0.0 && efb >= 0.0 && efc >= 0.0 {
                    // Normalize the given barycentric coordinate values
                    let barycentric_coords = Vec3::new(efb, efc, efa) / area;

                    // Calculate this triangle's z depth at this fragment via barycentric coordinates
                    // The perspective divide has already occured on these z values, which should
                    // give us a proper non-linear depth buffer with high precision near the screen and
                    // low precision towards the far plane.
                    let z_depth =
                        clip_z[0].interpolated(barycentric_coords, &clip_z[1], &clip_z[2]);

                    // TODO: Consider early-z discard

                    // Run fragment shader
                    let interpolated = program_inputs[0].interpolated(
                        barycentric_coords,
                        &program_inputs[1],
                        &program_inputs[2],
                    );
                    let frag_output = program.fragment(interpolated);
                    let fb_color = frag_output.z | (frag_output.y << 8) | (frag_output.x << 16);

                    // We only update the buffers if the z test determines that this primitive is closer
                    // than any other primitive we have processed so far.
                    if z_depth < self.db.get_pixel(x as u16, y as u16) {
                        self.db.plot_pixel(x as u16, y as u16, z_depth);
                        self.cb.plot_pixel(x as u16, y as u16, fb_color);
                    }
                }
                efa += dya;
                efb += dyb;
                efc += dyc;
            }
            efa = saved_efa;
            efb = saved_efb;
            efc = saved_efc;
            efa -= dxa;
            efb -= dxb;
            efc -= dxc;
        }
    }

    fn plot_line<S: Shader<V, VI>, V, VI: Barycentric>(
        &mut self,
        mut p1: Vec2,
        mut p2: Vec2,
        program: &S,
        p1_input: &VI,
        p2_input: &VI,
    ) {
        // TODO: This line algorithm doesn't seem to handle subpixel precision correctly.
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
        let sign = if dy >= 0.0 { 1.0 } else { -1.0 };

        for x in p1.x as i32..p2.x as i32 {
            // Barycentric coordinates for a line: treat it like an edge on a triangle
            // Basically, we just lerp between x and y, and set z to 0
            let pixel = IVec2::new(x as i32, y as i32);
            let interpolated = self.tri_barycentric_interpolate_edge(
                p1_orig,
                p2_orig,
                pixel.as_vec2(),
                p1_input,
                p2_input,
            );

            let frag_output = program.fragment(interpolated);
            let fb_color = frag_output.z | (frag_output.y << 8) | (frag_output.x << 16);
            if y_long {
                // Swap back to screen-space
                self.cb.plot_pixel(y as u16, x as u16, fb_color);
            } else {
                // x and y are already in screen-space
                self.cb.plot_pixel(x as u16, y as u16, fb_color);
            }

            if eps >= 0.0 {
                y += sign;

                eps -= dx;
            }
            eps += dy_abs;
        }
    }
}
