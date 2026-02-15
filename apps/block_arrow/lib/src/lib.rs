//
// Block Arrow - Core algorithm library
// Shapes, coverage algorithm, arrow assignment, level generation
//

#![allow(dead_code)]

use rand::seq::SliceRandom;
use std::collections::HashSet;

// ============================================================
// Direction
// ============================================================

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Direction {
    Up,
    Down,
    Left,
    Right,
}

impl Direction {
    pub const ALL: [Direction; 4] = [
        Direction::Up,
        Direction::Down,
        Direction::Left,
        Direction::Right,
    ];

    pub fn arrow_char(&self) -> &'static str {
        match self {
            Direction::Up => "▲",
            Direction::Down => "▼",
            Direction::Left => "◀",
            Direction::Right => "▶",
        }
    }
}

// ============================================================
// Shape variant: a normalized set of relative (dx, dy) cells
// ============================================================

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct ShapeVariant {
    pub cells: Vec<(i8, i8)>,
}

/// All unique shape variants grouped by base shape name
#[derive(Clone, Debug)]
pub struct ShapeFamily {
    pub name: &'static str,
    pub variants: Vec<ShapeVariant>,
}

/// Normalize cells: translate so min x=0, min y=0, then sort
fn normalize(cells: &[(i8, i8)]) -> Vec<(i8, i8)> {
    let min_x = cells.iter().map(|c| c.0).min().unwrap_or(0);
    let min_y = cells.iter().map(|c| c.1).min().unwrap_or(0);
    let mut norm: Vec<(i8, i8)> = cells.iter().map(|&(x, y)| (x - min_x, y - min_y)).collect();
    norm.sort();
    norm
}

/// Rotate 90 degrees clockwise: (x, y) -> (y, -x), then normalize
fn rotate90(cells: &[(i8, i8)]) -> Vec<(i8, i8)> {
    let rotated: Vec<(i8, i8)> = cells.iter().map(|&(x, y)| (y, -x)).collect();
    normalize(&rotated)
}

/// Mirror horizontally: (x, y) -> (-x, y), then normalize
fn mirror(cells: &[(i8, i8)]) -> Vec<(i8, i8)> {
    let mirrored: Vec<(i8, i8)> = cells.iter().map(|&(x, y)| (-x, y)).collect();
    normalize(&mirrored)
}

/// Generate all unique rotation variants of a shape (no mirror,
/// since S/Z and J/L are defined separately)
fn generate_variants(base: &[(i8, i8)]) -> Vec<ShapeVariant> {
    let mut seen: HashSet<Vec<(i8, i8)>> = HashSet::new();
    let mut variants = Vec::new();

    let mut current = normalize(base);
    for _ in 0..4 {
        let n = normalize(&current);
        if seen.insert(n.clone()) {
            variants.push(ShapeVariant { cells: n });
        }
        current = rotate90(&current);
    }
    variants
}

/// Build the complete shape library
pub fn build_shape_library() -> Vec<ShapeFamily> {
    let shapes: Vec<(&str, Vec<(i8, i8)>)> = vec![
        // monomino
        ("O1", vec![(0, 0)]),
        // domino
        ("I2", vec![(0, 0), (1, 0)]),
        // triomino
        ("I3", vec![(0, 0), (1, 0), (2, 0)]),
        ("L3", vec![(0, 0), (1, 0), (0, 1)]),
        // tetromino
        ("I4", vec![(0, 0), (1, 0), (2, 0), (3, 0)]),
        ("O4", vec![(0, 0), (1, 0), (0, 1), (1, 1)]),
        ("T4", vec![(0, 0), (1, 0), (2, 0), (1, 1)]),
        ("S4", vec![(1, 0), (2, 0), (0, 1), (1, 1)]),
        ("Z4", vec![(0, 0), (1, 0), (1, 1), (2, 1)]),
        ("J4", vec![(0, 0), (0, 1), (1, 1), (2, 1)]),
        ("L4", vec![(2, 0), (0, 1), (1, 1), (2, 1)]),
    ];

    shapes
        .into_iter()
        .map(|(name, base)| {
            let variants = generate_variants(&base);
            ShapeFamily { name, variants }
        })
        .collect()
}

// ============================================================
// Placed block and Level
// ============================================================

#[derive(Clone, Debug)]
pub struct PlacedBlock {
    pub id: usize,
    pub cells: Vec<(usize, usize)>, // absolute board coords (x, y)
    pub color: u8,                   // bitmap color 1-15
    pub arrow: Direction,
}

#[derive(Clone, Debug)]
pub struct Level {
    pub width: usize,
    pub height: usize,
    pub bitmap: Vec<Vec<u8>>,       // [y][x], 0=bg, 1-15=color
    pub blocks: Vec<PlacedBlock>,
    pub solution: Vec<usize>,        // removal order (block IDs)
}

// ============================================================
// Coverage algorithm: per-color backtracking
// ============================================================

/// Collect all cells of a given color from the bitmap
fn cells_of_color(bitmap: &[Vec<u8>], color: u8) -> Vec<(usize, usize)> {
    let mut cells = Vec::new();
    for (y, row) in bitmap.iter().enumerate() {
        for (x, &val) in row.iter().enumerate() {
            if val == color {
                cells.push((x, y));
            }
        }
    }
    cells
}

/// All shape variants sorted by cell count (large first), with all variants per family
fn all_variants_sorted(library: &[ShapeFamily]) -> Vec<&ShapeVariant> {
    let mut all: Vec<(&ShapeVariant, usize)> = Vec::new();
    for family in library {
        for variant in &family.variants {
            all.push((variant, variant.cells.len()));
        }
    }
    // Sort by size descending (prefer larger blocks)
    all.sort_by(|a, b| b.1.cmp(&a.1));
    all.into_iter().map(|(v, _)| v).collect()
}

/// Try to cover a set of same-color cells using backtracking
fn cover_region(
    uncovered: &mut Vec<(usize, usize)>,
    variants: &[&ShapeVariant],
    blocks: &mut Vec<PlacedBlock>,
    color: u8,
    next_id: &mut usize,
    rng: &mut impl rand::Rng,
) -> bool {
    if uncovered.is_empty() {
        return true;
    }

    // Pick the first (top-left) uncovered cell as anchor
    uncovered.sort_by(|a, b| a.1.cmp(&b.1).then(a.0.cmp(&b.0)));
    let target = uncovered[0];

    let uncov_set: HashSet<(usize, usize)> = uncovered.iter().cloned().collect();

    // Build list of variant indices, shuffled within each size group
    let mut by_size: Vec<Vec<usize>> = Vec::new();
    let mut current_size = 0;
    for (i, v) in variants.iter().enumerate() {
        let sz = v.cells.len();
        if sz != current_size {
            by_size.push(Vec::new());
            current_size = sz;
        }
        by_size.last_mut().unwrap().push(i);
    }
    for group in &mut by_size {
        group.shuffle(rng);
    }
    let variant_order: Vec<usize> = by_size.into_iter().flatten().collect();

    for &vi in &variant_order {
        let variant = variants[vi];

        // For each cell in the variant, try it as the anchor point
        for anchor_cell in &variant.cells {
            let ox = target.0 as i8 - anchor_cell.0;
            let oy = target.1 as i8 - anchor_cell.1;

            // Compute absolute positions
            let mut placement: Vec<(usize, usize)> = Vec::new();
            let mut valid = true;
            for &(dx, dy) in &variant.cells {
                let ax = ox + dx;
                let ay = oy + dy;
                if ax < 0 || ay < 0 {
                    valid = false;
                    break;
                }
                let pos = (ax as usize, ay as usize);
                if !uncov_set.contains(&pos) {
                    valid = false;
                    break;
                }
                placement.push(pos);
            }

            if !valid {
                continue;
            }

            // Place the block
            let block = PlacedBlock {
                id: *next_id,
                cells: placement.clone(),
                color,
                arrow: Direction::Right, // placeholder
            };
            *next_id += 1;
            blocks.push(block);

            // Remove placed cells
            let placed_set: HashSet<(usize, usize)> = placement.iter().cloned().collect();
            let remaining: Vec<(usize, usize)> =
                uncovered.iter().filter(|c| !placed_set.contains(c)).cloned().collect();
            *uncovered = remaining;

            if cover_region(uncovered, variants, blocks, color, next_id, rng) {
                return true;
            }

            // Backtrack
            *next_id -= 1;
            blocks.pop();
            for p in &placement {
                uncovered.push(*p);
            }
        }
    }

    false
}

/// Cover all non-zero cells in the bitmap, per color
fn solve_cover(
    bitmap: &[Vec<u8>],
    library: &[ShapeFamily],
    rng: &mut impl rand::Rng,
) -> Option<Vec<PlacedBlock>> {
    let variants = all_variants_sorted(library);
    let mut all_blocks = Vec::new();
    let mut next_id = 0;

    for color in 1..=15u8 {
        let mut cells = cells_of_color(bitmap, color);
        if cells.is_empty() {
            continue;
        }
        if !cover_region(&mut cells, &variants, &mut all_blocks, color, &mut next_id, rng) {
            return None;
        }
    }

    Some(all_blocks)
}

// ============================================================
// Arrow assignment + solvability
// ============================================================

/// Check if a block can fly in the given direction without hitting other remaining blocks.
/// "Fly" means every cell of the block can move in `dir` all the way to the board edge
/// without passing through any cell occupied by another remaining block.
fn can_fly(
    block: &PlacedBlock,
    dir: Direction,
    remaining: &[&PlacedBlock],
    width: usize,
    height: usize,
) -> bool {
    // Collect all cells of other remaining blocks
    let mut occupied: HashSet<(usize, usize)> = HashSet::new();
    for other in remaining {
        if other.id == block.id {
            continue;
        }
        for &cell in &other.cells {
            occupied.insert(cell);
        }
    }

    let block_cells: HashSet<(usize, usize)> = block.cells.iter().cloned().collect();

    // For each cell in the block, check the path in the given direction
    for &(x, y) in &block.cells {
        match dir {
            Direction::Up => {
                // Check all cells from y-1 up to 0
                for cy in (0..y).rev() {
                    let pos = (x, cy);
                    if !block_cells.contains(&pos) && occupied.contains(&pos) {
                        return false;
                    }
                }
            }
            Direction::Down => {
                for cy in (y + 1)..height {
                    let pos = (x, cy);
                    if !block_cells.contains(&pos) && occupied.contains(&pos) {
                        return false;
                    }
                }
            }
            Direction::Left => {
                for cx in (0..x).rev() {
                    let pos = (cx, y);
                    if !block_cells.contains(&pos) && occupied.contains(&pos) {
                        return false;
                    }
                }
            }
            Direction::Right => {
                for cx in (x + 1)..width {
                    let pos = (cx, y);
                    if !block_cells.contains(&pos) && occupied.contains(&pos) {
                        return false;
                    }
                }
            }
        }
    }

    true
}

/// Greedy arrow assignment: repeatedly find a block that can fly in some direction,
/// assign that arrow, remove it, and repeat.
fn assign_arrows(
    blocks: &mut [PlacedBlock],
    width: usize,
    height: usize,
    rng: &mut impl rand::Rng,
) -> Option<Vec<usize>> {
    let n = blocks.len();
    let mut removed = vec![false; n];
    let mut solution = Vec::new();
    let mut dirs = Direction::ALL;

    for _ in 0..n {
        let mut indices: Vec<usize> = (0..n).filter(|&i| !removed[i]).collect();
        indices.shuffle(rng);
        dirs.shuffle(rng);

        let mut found = false;
        for &idx in &indices {
            let remaining: Vec<&PlacedBlock> = (0..n)
                .filter(|&i| !removed[i])
                .map(|i| &blocks[i])
                .collect();

            for &dir in &dirs {
                if can_fly(&blocks[idx], dir, &remaining, width, height) {
                    blocks[idx].arrow = dir;
                    removed[idx] = true;
                    solution.push(blocks[idx].id);
                    found = true;
                    break;
                }
            }
            if found {
                break;
            }
        }

        if !found {
            return None;
        }
    }

    // Reverse: the last removed should be first in the player's solution
    solution.reverse();
    Some(solution)
}

// ============================================================
// Level generation: cover + arrows, with retries
// ============================================================

const MAX_ATTEMPTS: usize = 100;

pub fn generate_level(bitmap: &[Vec<u8>]) -> Option<Level> {
    let height = bitmap.len();
    if height == 0 {
        return None;
    }
    let width = bitmap[0].len();
    let library = build_shape_library();
    let mut rng = rand::rng();

    for _ in 0..MAX_ATTEMPTS {
        if let Some(mut blocks) = solve_cover(bitmap, &library, &mut rng) {
            if let Some(solution) = assign_arrows(&mut blocks, width, height, &mut rng) {
                return Some(Level {
                    width,
                    height,
                    bitmap: bitmap.to_vec(),
                    blocks,
                    solution,
                });
            }
        }
    }

    None
}

// ============================================================
// Built-in level bitmaps
// ============================================================

pub fn builtin_levels() -> Vec<Vec<Vec<u8>>> {
    vec![
        // Level 0: 9×9 (colors 1,2,3)
        parse_bitmap(&[
            "001111100",
            "011112210",
            "122111211",
            "122213222",
            "221121122",
            "121121122",
            "112211221",
            "012221110",
            "001211100",
        ]),
        // Level 1: 9×9 (colors 4,5,6,7)
        parse_bitmap(&[
            "044444440",
            "444444444",
            "445544444",
            "445555555",
            "445565565",
            "455555555",
            "455557755",
            "445555554",
            "446681844",
        ]),
    ]
}

/// Parse a string bitmap into Vec<Vec<u8>>
pub fn parse_bitmap(rows: &[&str]) -> Vec<Vec<u8>> {
    rows.iter()
        .map(|row| {
            row.chars()
                .map(|c| c.to_digit(16).unwrap_or(0) as u8)
                .collect()
        })
        .collect()
}

// ============================================================
// Board state (runtime)
// ============================================================

pub struct Board {
    pub width: usize,
    pub height: usize,
    pub grid: Vec<Vec<Option<usize>>>, // block ID at each cell
    pub blocks: Vec<PlacedBlock>,
    pub removed: Vec<bool>,
}

impl Board {
    pub fn from_level(level: &Level) -> Self {
        let mut grid = vec![vec![None; level.width]; level.height];
        for block in &level.blocks {
            for &(x, y) in &block.cells {
                grid[y][x] = Some(block.id);
            }
        }
        let n = level.blocks.len();
        Board {
            width: level.width,
            height: level.height,
            grid,
            blocks: level.blocks.clone(),
            removed: vec![false; n],
        }
    }

    /// Get the block ID at position (x, y), if any and not removed
    pub fn block_at(&self, x: usize, y: usize) -> Option<usize> {
        if let Some(id) = self.grid[y][x] {
            if !self.removed[id] {
                return Some(id);
            }
        }
        None
    }

    /// Try to fly a block. Returns true if successful.
    pub fn try_fly(&mut self, block_id: usize) -> bool {
        if block_id >= self.blocks.len() || self.removed[block_id] {
            return false;
        }

        let remaining: Vec<&PlacedBlock> = self
            .blocks
            .iter()
            .filter(|b| !self.removed[b.id])
            .collect();

        let block = &self.blocks[block_id];
        let dir = block.arrow;

        if can_fly(block, dir, &remaining, self.width, self.height) {
            self.removed[block_id] = true;
            for &(x, y) in &self.blocks[block_id].cells {
                self.grid[y][x] = None;
            }
            true
        } else {
            false
        }
    }

    /// Check if all blocks have been removed
    pub fn all_removed(&self) -> bool {
        self.removed.iter().all(|&r| r)
    }

    /// Count remaining blocks
    pub fn remaining_count(&self) -> usize {
        self.removed.iter().filter(|&&r| !r).count()
    }

    /// Calculate border type for a cell at (x, y) belonging to block_id.
    /// Returns a 4-bit value: bit3=top, bit2=bottom, bit1=left, bit0=right
    /// A bit is set if that neighbor does NOT belong to the same block.
    pub fn border_type(&self, x: usize, y: usize, block_id: usize) -> u8 {
        let mut bits = 0u8;
        // Top
        if y == 0 || self.grid[y - 1][x] != Some(block_id) {
            bits |= 0b1000;
        }
        // Bottom
        if y + 1 >= self.height || self.grid[y + 1][x] != Some(block_id) {
            bits |= 0b0100;
        }
        // Left
        if x == 0 || self.grid[y][x - 1] != Some(block_id) {
            bits |= 0b0010;
        }
        // Right
        if x + 1 >= self.width || self.grid[y][x + 1] != Some(block_id) {
            bits |= 0b0001;
        }
        bits
    }
}

// ============================================================
// Tests
// ============================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_shape_variants() {
        let library = build_shape_library();
        for family in &library {
            match family.name {
                "O1" => assert_eq!(family.variants.len(), 1),
                "I2" => assert_eq!(family.variants.len(), 2),
                "I3" => assert_eq!(family.variants.len(), 2),
                "L3" => assert_eq!(family.variants.len(), 4),
                "I4" => assert_eq!(family.variants.len(), 2),
                "O4" => assert_eq!(family.variants.len(), 1),
                "T4" => assert_eq!(family.variants.len(), 4),
                "S4" => assert_eq!(family.variants.len(), 2),
                "Z4" => assert_eq!(family.variants.len(), 2),
                "J4" => assert_eq!(family.variants.len(), 4),
                "L4" => assert_eq!(family.variants.len(), 4),
                _ => {}
            }
        }
    }

    #[test]
    fn test_level_generation() {
        let levels = builtin_levels();
        for (i, bitmap) in levels.iter().enumerate() {
            let level = generate_level(bitmap);
            assert!(level.is_some(), "Failed to generate level {}", i);
            let level = level.unwrap();
            // Verify full coverage
            let mut covered = vec![vec![false; level.width]; level.height];
            for block in &level.blocks {
                for &(x, y) in &block.cells {
                    assert!(!covered[y][x], "Overlap at ({}, {})", x, y);
                    covered[y][x] = true;
                    assert_eq!(
                        bitmap[y][x], block.color,
                        "Color mismatch at ({}, {})",
                        x, y
                    );
                }
            }
            // All non-zero cells covered
            for (y, row) in bitmap.iter().enumerate() {
                for (x, &val) in row.iter().enumerate() {
                    if val != 0 {
                        assert!(covered[y][x], "Uncovered at ({}, {})", x, y);
                    }
                }
            }
        }
    }
}
