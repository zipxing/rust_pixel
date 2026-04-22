// RustPixel
// copyright zipxing@hotmail.com 2022～2026

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum VoxelFaceMaterial {
    All(String),
    TopBottomSide {
        top: String,
        bottom: String,
        side: String,
    },
    SixFace {
        pos_x: String,
        neg_x: String,
        pos_y: String,
        neg_y: String,
        pos_z: String,
        neg_z: String,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VoxelMaterial {
    pub id: String,
    pub faces: VoxelFaceMaterial,
}
