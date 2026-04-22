// RustPixel
// copyright zipxing@hotmail.com 2022～2026

//! Minimal voxel rendering skeleton for the future `3d` mode.

pub mod atlas;
pub mod camera;
pub mod material;
pub mod mesh;
pub mod renderer;
pub mod world;

pub use atlas::{VoxelAtlasEntry, VoxelAtlasResolver, VoxelFaceRef};
pub use camera::Camera3D;
pub use material::{VoxelFaceMaterial, VoxelMaterial};
pub use mesh::{ChunkMesh, FaceDir, VoxelVertex};
pub use renderer::VoxelRenderer;
pub use world::{BlockId, Chunk, ChunkCoord, VoxelWorld};
