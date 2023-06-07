use glam::{UVec3, Vec4};

// TODO: Programmable shaders need to be able to take in multiple inputs (and outputs). Use a vec?
pub trait Shader<Vertex> {
    fn vertex(&self, pos: &Vertex) -> Vec4;
    fn fragment(&self) -> UVec3;
    fn set_barycentric_coords(&mut self, x: f32, y: f32, z: f32); // TODO: Friendlier way of setting this.
}
