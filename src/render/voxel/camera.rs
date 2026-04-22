// RustPixel
// copyright zipxing@hotmail.com 2022～2026

#[derive(Debug, Clone, Copy)]
pub struct Camera3D {
    pub position: [f32; 3],
    pub yaw: f32,
    pub pitch: f32,
    pub fov_y_deg: f32,
    pub z_near: f32,
    pub z_far: f32,
}

impl Default for Camera3D {
    fn default() -> Self {
        Self {
            position: [0.0, 0.0, 0.0],
            yaw: 0.0,
            pitch: 0.0,
            fov_y_deg: 70.0,
            z_near: 0.1,
            z_far: 1000.0,
        }
    }
}
