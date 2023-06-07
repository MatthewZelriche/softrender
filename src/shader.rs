use glam::{UVec3, Vec2, Vec3, Vec4};

pub trait Shader<VertexIn, VertexOut> {
    fn vertex(&self, pos: &VertexIn) -> (Vec4, VertexOut);
    fn fragment(&self, interpolated: VertexOut) -> UVec3;
}

pub trait Barycentric {
    fn interpolated(&self, coords: Vec3, second: &Self, third: &Self) -> Self;
}

impl Barycentric for f32 {
    fn interpolated(&self, coords: Vec3, second: &Self, third: &Self) -> Self {
        self * coords.x + second * coords.y + third * coords.z
    }
}

impl Barycentric for Vec2 {
    fn interpolated(&self, coords: Vec3, second: &Self, third: &Self) -> Self {
        *self * coords.x + *second * coords.y + *third * coords.z
    }
}

impl Barycentric for Vec3 {
    fn interpolated(&self, coords: Vec3, second: &Self, third: &Self) -> Self {
        *self * coords.x + *second * coords.y + *third * coords.z
    }
}

impl Barycentric for Vec4 {
    fn interpolated(&self, coords: Vec3, second: &Self, third: &Self) -> Self {
        *self * coords.x + *second * coords.y + *third * coords.z
    }
}

impl Barycentric for () {
    fn interpolated(&self, _coords: Vec3, _second: &Self, _third: &Self) -> Self {}
}
