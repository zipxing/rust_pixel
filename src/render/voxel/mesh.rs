// RustPixel
// copyright zipxing@hotmail.com 2022～2026

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FaceDir {
    PosX,
    NegX,
    PosY,
    NegY,
    PosZ,
    NegZ,
}

#[derive(Debug, Clone, Copy, Default)]
pub struct VoxelVertex {
    pub position: [f32; 3],
    pub normal: [f32; 3],
    pub uv: [f32; 2],
    pub layer: u32,
}

#[derive(Debug, Default, Clone)]
pub struct ChunkMesh {
    pub vertices: Vec<VoxelVertex>,
    pub indices: Vec<u32>,
}
