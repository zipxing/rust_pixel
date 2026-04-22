// RustPixel
// copyright zipxing@hotmail.com 2022～2026

use super::{Camera3D, ChunkMesh, VoxelWorld};

#[derive(Debug, Default)]
pub struct VoxelRenderer {
    pub initialized: bool,
}

impl VoxelRenderer {
    pub fn new() -> Self {
        Self { initialized: false }
    }

    pub fn init(&mut self) {
        self.initialized = true;
    }

    pub fn render(&mut self, _world: &VoxelWorld, _camera: &Camera3D, _meshes: &[ChunkMesh]) {
        // Skeleton only: actual WGPU voxel pass will be added in follow-up work.
    }
}
