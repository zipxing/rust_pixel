// RustPixel
// copyright zipxing@hotmail.com 2022～2026

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VoxelFaceRef {
    pub pua: String,
    pub symbol: String,
}

#[derive(Debug, Clone, Copy, Default)]
pub struct VoxelAtlasEntry {
    pub layer: u16,
    pub uv_min: [f32; 2],
    pub uv_max: [f32; 2],
}

#[derive(Debug, Default)]
pub struct VoxelAtlasResolver;

impl VoxelAtlasResolver {
    pub fn resolve_face(&self, _face: &VoxelFaceRef) -> Option<VoxelAtlasEntry> {
        None
    }
}
