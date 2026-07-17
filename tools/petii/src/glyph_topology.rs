use rust_pixel::render::symbols::BlockGrayImage;

const SIDE_COUNT: usize = 4;
const BORDER_PIXELS: usize = 8;
const PORT_POSITIONS: f64 = 7.0;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Side {
    Top = 0,
    Right = 1,
    Bottom = 2,
    Left = 3,
}

impl Side {
    pub const ALL: [Self; SIDE_COUNT] = [Self::Top, Self::Right, Self::Bottom, Self::Left];

    pub const fn opposite(self) -> Self {
        match self {
            Self::Top => Self::Bottom,
            Self::Right => Self::Left,
            Self::Bottom => Self::Top,
            Self::Left => Self::Right,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GlyphRole {
    Blank,
    Solid,
    Texture,
    Endpoint,
    Straight,
    Corner,
    Junction,
}

/// Geometry visible at a glyph's cell boundaries.
///
/// `fill_masks` records foreground pixels along each side. `edge_ports` records
/// transitions between foreground and background pixels, which are the points
/// where a perceived fill boundary crosses into the adjacent cell.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GlyphTopology {
    fill_masks: [u8; SIDE_COUNT],
    edge_ports: [u8; SIDE_COUNT],
    foreground_pixels: u8,
    role: GlyphRole,
}

impl GlyphTopology {
    pub fn from_bitmap(bitmap: &BlockGrayImage) -> Self {
        debug_assert_eq!(bitmap.len(), BORDER_PIXELS);
        debug_assert!(bitmap.iter().all(|row| row.len() == BORDER_PIXELS));

        let mut fill_masks = [0u8; SIDE_COUNT];
        for side in Side::ALL {
            let mut mask = 0u8;
            for offset in 0..BORDER_PIXELS {
                let (x, y) = border_coordinate(side, offset);
                if bitmap[y][x] >= 128 {
                    mask |= 1 << offset;
                }
            }
            fill_masks[side as usize] = mask;
        }

        let edge_ports = fill_masks.map(transition_mask);
        let foreground_pixels = bitmap
            .iter()
            .flatten()
            .filter(|pixel| **pixel >= 128)
            .count() as u8;
        let role = classify_role(foreground_pixels, edge_ports);
        Self {
            fill_masks,
            edge_ports,
            foreground_pixels,
            role,
        }
    }

    pub const fn fill_mask(self, side: Side) -> u8 {
        self.fill_masks[side as usize]
    }

    pub const fn edge_ports(self, side: Side) -> u8 {
        self.edge_ports[side as usize]
    }

    pub const fn foreground_pixels(self) -> u8 {
        self.foreground_pixels
    }

    pub const fn role(self) -> GlyphRole {
        self.role
    }

    pub fn active_sides(self) -> usize {
        self.edge_ports.iter().filter(|ports| **ports != 0).count()
    }

    /// Compare a candidate glyph to the desired topology inside one cell.
    pub fn target_distance(self, target: Self) -> f64 {
        let port_loss = self.port_distance(target);
        let fill_loss = (self.foreground_pixels as f64 - target.foreground_pixels as f64).abs()
            / (BORDER_PIXELS * BORDER_PIXELS) as f64;
        let side_loss =
            self.active_sides().abs_diff(target.active_sides()) as f64 / SIDE_COUNT as f64;
        0.7 * port_loss + 0.15 * fill_loss + 0.15 * side_loss
    }

    pub fn port_distance(self, target: Self) -> f64 {
        let port_bits = Side::ALL
            .iter()
            .map(|side| (self.edge_ports(*side) ^ target.edge_ports(*side)).count_ones())
            .sum::<u32>() as f64;
        port_bits / (PORT_POSITIONS * SIDE_COUNT as f64)
    }

    /// Compare the contour crossings on a boundary shared by two cells.
    pub fn shared_port_mismatch(self, side: Side, neighbor: Self) -> f64 {
        let first = self.edge_ports(side);
        let second = neighbor.edge_ports(side.opposite());
        (first ^ second).count_ones() as f64 / PORT_POSITIONS
    }

    /// Compare shared contour crossings while accepting the one-pixel offset
    /// that dominates hand-authored PETSCII edge transitions.
    pub fn shared_port_mismatch_tolerant(self, side: Side, neighbor: Self) -> f64 {
        let first = self.edge_ports(side);
        let second = neighbor.edge_ports(side.opposite());
        let unmatched_first = first & !dilate_ports(second);
        let unmatched_second = second & !dilate_ports(first);
        unmatched_first
            .count_ones()
            .max(unmatched_second.count_ones()) as f64
            / PORT_POSITIONS
    }
}

pub fn build_topology_catalog(charset: &[BlockGrayImage]) -> Vec<GlyphTopology> {
    charset.iter().map(GlyphTopology::from_bitmap).collect()
}

fn border_coordinate(side: Side, offset: usize) -> (usize, usize) {
    match side {
        Side::Top => (offset, 0),
        Side::Right => (BORDER_PIXELS - 1, offset),
        Side::Bottom => (offset, BORDER_PIXELS - 1),
        Side::Left => (0, offset),
    }
}

fn transition_mask(fill_mask: u8) -> u8 {
    (fill_mask ^ (fill_mask >> 1)) & 0x7f
}

fn dilate_ports(ports: u8) -> u8 {
    (ports | (ports << 1) | (ports >> 1)) & 0x7f
}

fn classify_role(foreground_pixels: u8, edge_ports: [u8; SIDE_COUNT]) -> GlyphRole {
    if foreground_pixels == 0 {
        return GlyphRole::Blank;
    }
    if foreground_pixels as usize == BORDER_PIXELS * BORDER_PIXELS {
        return GlyphRole::Solid;
    }

    let active: Vec<Side> = Side::ALL
        .into_iter()
        .filter(|side| edge_ports[*side as usize] != 0)
        .collect();
    match active.as_slice() {
        [] => GlyphRole::Texture,
        [_] => GlyphRole::Endpoint,
        [first, second] if first.opposite() == *second => GlyphRole::Straight,
        [_, _] => GlyphRole::Corner,
        _ => GlyphRole::Junction,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::c64::C64UP;

    fn bitmap_with_right_half() -> BlockGrayImage {
        let mut bitmap = vec![vec![0u8; 8]; 8];
        for row in &mut bitmap {
            row[4..].fill(255);
        }
        bitmap
    }

    #[test]
    fn flat_glyphs_have_no_edge_ports() {
        let blank = GlyphTopology::from_bitmap(&vec![vec![0u8; 8]; 8]);
        let solid = GlyphTopology::from_bitmap(&vec![vec![255u8; 8]; 8]);
        assert_eq!(blank.role(), GlyphRole::Blank);
        assert_eq!(solid.role(), GlyphRole::Solid);
        assert!(Side::ALL
            .iter()
            .all(|side| blank.edge_ports(*side) == 0 && solid.edge_ports(*side) == 0));
    }

    #[test]
    fn half_block_exposes_aligned_straight_ports() {
        let topology = GlyphTopology::from_bitmap(&bitmap_with_right_half());
        assert_eq!(topology.role(), GlyphRole::Straight);
        assert_eq!(topology.edge_ports(Side::Top), 1 << 3);
        assert_eq!(topology.edge_ports(Side::Bottom), 1 << 3);
        assert_eq!(topology.edge_ports(Side::Left), 0);
        assert_eq!(topology.edge_ports(Side::Right), 0);
    }

    #[test]
    fn inverse_fill_preserves_edge_crossings() {
        let bitmap = bitmap_with_right_half();
        let inverse: BlockGrayImage = bitmap
            .iter()
            .map(|row| row.iter().map(|pixel| 255 - pixel).collect())
            .collect();
        let normal = GlyphTopology::from_bitmap(&bitmap);
        let inverted = GlyphTopology::from_bitmap(&inverse);
        for side in Side::ALL {
            assert_eq!(normal.edge_ports(side), inverted.edge_ports(side));
        }
    }

    #[test]
    fn shared_port_score_distinguishes_alignment() {
        let centered = GlyphTopology::from_bitmap(&bitmap_with_right_half());
        let mut shifted_bitmap = vec![vec![0u8; 8]; 8];
        for row in &mut shifted_bitmap {
            row[5..].fill(255);
        }
        let shifted = GlyphTopology::from_bitmap(&shifted_bitmap);
        assert_eq!(centered.shared_port_mismatch(Side::Top, centered), 0.0);
        assert!(centered.shared_port_mismatch(Side::Top, shifted) > 0.0);
        assert_eq!(
            centered.shared_port_mismatch_tolerant(Side::Top, shifted),
            0.0
        );
    }

    #[test]
    fn classifies_endpoint_corner_and_junction_fixtures() {
        let mut endpoint = vec![vec![0u8; 8]; 8];
        for row in endpoint.iter_mut().take(5) {
            row[3..5].fill(255);
        }
        assert_eq!(
            GlyphTopology::from_bitmap(&endpoint).role(),
            GlyphRole::Endpoint
        );

        let mut corner = vec![vec![0u8; 8]; 8];
        for row in corner.iter_mut().skip(4) {
            row[4..].fill(255);
        }
        assert_eq!(
            GlyphTopology::from_bitmap(&corner).role(),
            GlyphRole::Corner
        );

        let mut junction = vec![vec![0u8; 8]; 8];
        for row in &mut junction {
            row[3..5].fill(255);
        }
        for row in junction.iter_mut().skip(3).take(2) {
            row[4..].fill(255);
        }
        assert_eq!(
            GlyphTopology::from_bitmap(&junction).role(),
            GlyphRole::Junction
        );
    }

    #[test]
    fn all_c64_inverse_pairs_preserve_edge_ports() {
        for rows in C64UP {
            let normal: BlockGrayImage = rows
                .iter()
                .map(|bits| {
                    (0..8)
                        .map(|x| if (bits >> (7 - x)) & 1 == 1 { 255 } else { 0 })
                        .collect()
                })
                .collect();
            let inverse: BlockGrayImage = normal
                .iter()
                .map(|row| row.iter().map(|pixel| 255 - pixel).collect())
                .collect();
            let normal = GlyphTopology::from_bitmap(&normal);
            let inverse = GlyphTopology::from_bitmap(&inverse);
            for side in Side::ALL {
                assert_eq!(normal.edge_ports(side), inverse.edge_ports(side));
            }
        }
    }
}
