use glam::{UVec3, Vec3, Vec4};

// TODO: Programmable shaders need to be able to take in multiple inputs (and outputs). Use a vec?
pub trait Shader {
    fn vertex(&self, pos: Vec3) -> Vec4;
    fn fragment(&self) -> UVec3;
}
