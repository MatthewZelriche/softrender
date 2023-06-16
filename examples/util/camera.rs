use glam::{Mat3A, Mat4, Vec2, Vec3};

pub struct Camera {
    persp_matrix: Mat4,
    view_matrix: Mat4,
    world_pos: Vec3,
    rot_matrix: Mat3A,
    look_dir: Vec3,
    right_dir: Vec3,
    up_dir: Vec3,
    pitch: f32,
}

impl Camera {
    pub fn new(fov: f32, aspect_ratio: f32, near: f32, far: f32, world_pos: Vec3) -> Self {
        let look_dir = Vec3::new(0.0, 0.0, -1.0);
        let right_dir = Vec3::new(0.0, 1.0, 0.0).cross(look_dir).normalize();
        Camera {
            persp_matrix: Mat4::perspective_rh(fov, aspect_ratio, near, far),
            view_matrix: Mat4::IDENTITY,
            world_pos,
            rot_matrix: Mat3A::from_euler(glam::EulerRot::XYZ, 0.0, 0.0, 0.0),
            look_dir,
            right_dir,
            up_dir: look_dir.cross(right_dir).normalize(),
            pitch: 0.0,
        }
    }

    pub fn rotate(&mut self, mut pitch: f32, yaw: f32) {
        if self.pitch + pitch >= 89.9f32.to_radians() {
            pitch = 0.0;
        }
        self.pitch += pitch;
        self.rot_matrix = Mat3A::from_euler(glam::EulerRot::XYZ, pitch, yaw, 0.0);
        self.look_dir = self.rot_matrix * self.look_dir;
        self.right_dir = Vec3::new(0.0, 1.0, 0.0).cross(self.look_dir).normalize();
        self.up_dir = self.look_dir.cross(self.right_dir).normalize();
    }

    pub fn move_cam(&mut self, delta: Vec2) {
        self.world_pos += self.look_dir * delta.x;
        self.world_pos += self.right_dir * delta.y;
    }

    pub fn tick(&mut self) {
        self.view_matrix = glam::Mat4::look_to_rh(self.world_pos, self.look_dir, self.up_dir);
    }

    pub fn view_projection_matrix(&self) -> Mat4 {
        self.persp_matrix * self.view_matrix
    }
}
