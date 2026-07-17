use crate::glyph_topology::{GlyphTopology, Side};
use std::collections::VecDeque;

const PORT_MASK: u8 = 0x7f;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ContourChain {
    pub cells: Vec<usize>,
}

#[derive(Debug, Clone)]
pub struct ContourGraph {
    width: u32,
    adjacency: Vec<Vec<usize>>,
}

impl ContourGraph {
    pub fn from_targets(width: u32, height: u32, targets: &[Option<GlyphTopology>]) -> Self {
        let expected = width as usize * height as usize;
        debug_assert_eq!(targets.len(), expected);
        let mut adjacency = vec![Vec::new(); expected];
        for y in 0..height {
            for x in 0..width {
                let index = (y * width + x) as usize;
                for (neighbor, side) in [
                    ((x + 1 < width).then_some(index + 1), Side::Right),
                    (
                        (y + 1 < height).then_some(index + width as usize),
                        Side::Bottom,
                    ),
                ] {
                    let Some(neighbor) = neighbor else {
                        continue;
                    };
                    let (Some(current), Some(next)) = (targets[index], targets[neighbor]) else {
                        continue;
                    };
                    if ports_connect(current.edge_ports(side), next.edge_ports(side.opposite())) {
                        adjacency[index].push(neighbor);
                        adjacency[neighbor].push(index);
                    }
                }
            }
        }
        for neighbors in &mut adjacency {
            neighbors.sort_unstable();
        }
        Self { width, adjacency }
    }

    pub fn side_between(&self, first: usize, second: usize) -> Option<Side> {
        if second + self.width as usize == first {
            Some(Side::Top)
        } else if first + 1 == second && first / self.width as usize == second / self.width as usize
        {
            Some(Side::Right)
        } else if first + self.width as usize == second {
            Some(Side::Bottom)
        } else if second + 1 == first && first / self.width as usize == second / self.width as usize
        {
            Some(Side::Left)
        } else {
            None
        }
    }

    /// Return stable open chains whose connected component contains no junction.
    /// Closed loops and junction components are intentionally left to the bounded
    /// local coordinator until their cyclic/junction solver is implemented.
    pub fn open_chains(&self) -> Vec<ContourChain> {
        let mut chains = Vec::new();
        for component in self.components() {
            if component
                .iter()
                .any(|index| self.adjacency[*index].len() > 2)
            {
                continue;
            }
            let endpoints: Vec<_> = component
                .iter()
                .copied()
                .filter(|index| self.adjacency[*index].len() == 1)
                .collect();
            if endpoints.len() != 2 {
                continue;
            }

            let mut cells = Vec::with_capacity(component.len());
            let mut previous = None;
            let mut current = endpoints[0].min(endpoints[1]);
            loop {
                cells.push(current);
                let next = self.adjacency[current]
                    .iter()
                    .copied()
                    .find(|neighbor| Some(*neighbor) != previous);
                let Some(next) = next else {
                    break;
                };
                previous = Some(current);
                current = next;
            }
            if cells.len() == component.len() {
                chains.push(ContourChain { cells });
            }
        }
        chains.sort_by_key(|chain| chain.cells[0]);
        chains
    }

    /// Return stable simple cycles. The smallest cell index starts the loop and
    /// the smaller of its two neighbors fixes traversal direction.
    pub fn closed_loops(&self) -> Vec<ContourChain> {
        let mut loops = Vec::new();
        for component in self.components() {
            if component.len() < 4
                || component
                    .iter()
                    .any(|index| self.adjacency[*index].len() != 2)
            {
                continue;
            }
            let start = component[0];
            let mut cells = Vec::with_capacity(component.len());
            let mut previous = start;
            let mut current = self.adjacency[start][0].min(self.adjacency[start][1]);
            cells.push(start);
            while current != start && cells.len() <= component.len() {
                cells.push(current);
                let neighbors = &self.adjacency[current];
                let next = if neighbors[0] == previous {
                    neighbors[1]
                } else {
                    neighbors[0]
                };
                previous = current;
                current = next;
            }
            if current == start && cells.len() == component.len() {
                loops.push(ContourChain { cells });
            }
        }
        loops.sort_by_key(|chain| chain.cells[0]);
        loops
    }

    pub fn junction_cells(&self) -> Vec<usize> {
        self.adjacency
            .iter()
            .enumerate()
            .filter_map(|(index, neighbors)| (neighbors.len() >= 3).then_some(index))
            .collect()
    }

    pub fn neighbors(&self, index: usize) -> &[usize] {
        &self.adjacency[index]
    }

    pub fn width(&self) -> u32 {
        self.width
    }

    pub fn height(&self) -> u32 {
        self.adjacency.len() as u32 / self.width
    }

    pub fn connections(&self) -> Vec<(usize, usize, Side)> {
        let mut connections = Vec::new();
        for first in 0..self.adjacency.len() {
            for &second in &self.adjacency[first] {
                if first < second {
                    let side = self
                        .side_between(first, second)
                        .expect("contour graph neighbors must be adjacent");
                    connections.push((first, second, side));
                }
            }
        }
        connections
    }

    fn components(&self) -> Vec<Vec<usize>> {
        let mut seen = vec![false; self.adjacency.len()];
        let mut components = Vec::new();
        for seed in 0..self.adjacency.len() {
            if seen[seed] || self.adjacency[seed].is_empty() {
                continue;
            }
            let mut queue = VecDeque::from([seed]);
            let mut component = Vec::new();
            seen[seed] = true;
            while let Some(index) = queue.pop_front() {
                component.push(index);
                for &neighbor in &self.adjacency[index] {
                    if !seen[neighbor] {
                        seen[neighbor] = true;
                        queue.push_back(neighbor);
                    }
                }
            }
            component.sort_unstable();
            components.push(component);
        }
        components
    }
}

fn ports_connect(first: u8, second: u8) -> bool {
    first != 0 && second != 0 && first & dilate_ports(second) != 0
}

fn dilate_ports(ports: u8) -> u8 {
    (ports | (ports << 1) | (ports >> 1)) & PORT_MASK
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_pixel::render::symbols::BlockGrayImage;

    fn horizontal_fill(start_y: usize) -> GlyphTopology {
        let mut bitmap = vec![vec![0u8; 8]; 8];
        for row in bitmap.iter_mut().skip(start_y) {
            row.fill(255);
        }
        GlyphTopology::from_bitmap(&bitmap)
    }

    fn vertical_fill(start_x: usize) -> GlyphTopology {
        let mut bitmap = vec![vec![0u8; 8]; 8];
        for row in &mut bitmap {
            row[start_x..].fill(255);
        }
        GlyphTopology::from_bitmap(&bitmap)
    }

    fn bottom_right_quadrant() -> GlyphTopology {
        let mut bitmap = vec![vec![0u8; 8]; 8];
        for row in bitmap.iter_mut().skip(4) {
            row[4..].fill(255);
        }
        GlyphTopology::from_bitmap(&bitmap)
    }

    fn quadrant(top: bool, left: bool) -> GlyphTopology {
        let mut bitmap = vec![vec![0u8; 8]; 8];
        let y_range = if top { 0..4 } else { 4..8 };
        let x_range = if left { 0..4 } else { 4..8 };
        for y in y_range {
            for x in x_range.clone() {
                bitmap[y][x] = 255;
            }
        }
        GlyphTopology::from_bitmap(&bitmap)
    }

    fn t_junction() -> GlyphTopology {
        let mut bitmap: BlockGrayImage = vec![vec![0u8; 8]; 8];
        for row in &mut bitmap {
            row[3..5].fill(255);
        }
        for row in bitmap.iter_mut().skip(3).take(2) {
            row[4..].fill(255);
        }
        GlyphTopology::from_bitmap(&bitmap)
    }

    #[test]
    fn traces_straight_chain_in_stable_order() {
        let line = horizontal_fill(4);
        let graph = ContourGraph::from_targets(3, 1, &[Some(line), Some(line), Some(line)]);
        assert_eq!(
            graph.open_chains(),
            vec![ContourChain {
                cells: vec![0, 1, 2]
            }]
        );
        assert_eq!(graph.side_between(0, 1), Some(Side::Right));
        assert_eq!(graph.side_between(1, 0), Some(Side::Left));
    }

    #[test]
    fn traces_corner_as_one_chain() {
        let graph = ContourGraph::from_targets(
            2,
            2,
            &[
                Some(bottom_right_quadrant()),
                Some(horizontal_fill(4)),
                Some(vertical_fill(4)),
                None,
            ],
        );
        assert_eq!(
            graph.open_chains(),
            vec![ContourChain {
                cells: vec![1, 0, 2]
            }]
        );
    }

    #[test]
    fn junction_component_is_left_for_local_coordination() {
        let vertical = vertical_fill(4);
        let horizontal = horizontal_fill(4);
        let graph = ContourGraph::from_targets(
            3,
            3,
            &[
                None,
                Some(vertical),
                None,
                None,
                Some(t_junction()),
                Some(horizontal),
                None,
                Some(vertical),
                None,
            ],
        );
        assert!(graph.open_chains().is_empty());
    }

    #[test]
    fn port_connection_tolerance_is_one_pixel() {
        assert!(ports_connect(1 << 3, 1 << 3));
        assert!(ports_connect(1 << 3, 1 << 4));
        assert!(!ports_connect(1 << 3, 1 << 5));
    }

    #[test]
    fn traces_closed_loop_with_stable_start_and_direction() {
        let graph = ContourGraph::from_targets(
            2,
            2,
            &[
                Some(quadrant(false, false)),
                Some(quadrant(false, true)),
                Some(quadrant(true, false)),
                Some(quadrant(true, true)),
            ],
        );
        assert!(graph.open_chains().is_empty());
        assert_eq!(
            graph.closed_loops(),
            vec![ContourChain {
                cells: vec![0, 1, 3, 2]
            }]
        );
        assert_eq!(graph.side_between(2, 0), Some(Side::Top));
    }
}
