use std::ops::{Div, Mul, Sub};

use glam::{Vec2, Vec4};

pub struct ClipPlane {
    pub sign: f32,
    pub axis: usize,
}

pub trait InverseLerp<T: Mul + Sub + Div> {
    fn inverse_lerp(&self, to: T, point: T) -> f32;
}

impl InverseLerp<Vec4> for Vec4 {
    fn inverse_lerp(&self, to: Vec4, point: Vec4) -> f32 {
        (point - *self).length_squared() / (to - *self).length_squared()
    }
}

impl InverseLerp<Vec2> for Vec2 {
    fn inverse_lerp(&self, to: Vec2, point: Vec2) -> f32 {
        (point - *self).length() / (to - *self).length()
    }
}
