// RustPixel
// copyright zipxing@hotmail.com 2022～2026

use std::collections::HashMap;

pub type BlockId = u16;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct ChunkCoord {
    pub x: i32,
    pub y: i32,
    pub z: i32,
}

#[derive(Debug, Clone)]
pub struct Chunk {
    pub size: [u32; 3],
    pub blocks: Vec<BlockId>,
}

impl Chunk {
    pub fn new(size: [u32; 3]) -> Self {
        let len = size[0] as usize * size[1] as usize * size[2] as usize;
        Self {
            size,
            blocks: vec![0; len],
        }
    }
}

#[derive(Debug, Default)]
pub struct VoxelWorld {
    pub chunks: HashMap<ChunkCoord, Chunk>,
}

impl VoxelWorld {
    pub fn insert_chunk(&mut self, coord: ChunkCoord, chunk: Chunk) {
        self.chunks.insert(coord, chunk);
    }
}
