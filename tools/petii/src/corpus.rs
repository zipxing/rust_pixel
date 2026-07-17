use crate::c64::{C64LOW, C64UP};
use crate::converter::bitmap_spur_penalty;
use crate::glyph_topology::{build_topology_catalog, GlyphRole, GlyphTopology, Side};
use rust_pixel::render::symbols::gen_charset_images;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CorpusReport {
    pub discovered_files: usize,
    pub files: usize,
    pub invalid_files: Vec<String>,
    pub cells: u64,
    pub glyph_counts: Vec<u64>,
    pub role_counts: BTreeMap<String, u64>,
    pub active_side_histogram: Vec<u64>,
    pub visible_role_counts: BTreeMap<String, u64>,
    pub visible_active_side_histogram: Vec<u64>,
    pub visible_edge_cells: u64,
    pub exact_port_connections: u64,
    pub tolerant_port_connections: u64,
    pub misaligned_port_connections: u64,
    pub one_sided_port_adjacencies: u64,
    pub mismatched_port_adjacencies: u64,
    pub internal_unexpected_endpoints: u64,
    pub image_border_endpoints: u64,
    pub glyph_junction_cells: u64,
    pub glyph_spur_cells: u64,
    pub junction_cells: u64,
    pub spur_cells: u64,
    pub same_foreground_background_cells: u64,
    pub foreground_palette_counts: Vec<u64>,
    pub background_palette_counts: Vec<u64>,
    pub color_pair_counts: BTreeMap<String, u64>,
}

struct PixArtwork {
    width: usize,
    height: usize,
    cells: Vec<PixCell>,
}

#[derive(Clone, Copy)]
struct PixCell {
    glyph: u8,
    foreground: u8,
    background: u8,
}

pub fn analyze_pix_corpus(directory: &Path) -> Result<CorpusReport, String> {
    let mut paths: Vec<PathBuf> = fs::read_dir(directory)
        .map_err(|error| format!("failed to read '{}': {error}", directory.display()))?
        .filter_map(|entry| entry.ok().map(|entry| entry.path()))
        .filter(|path| path.extension().is_some_and(|extension| extension == "pix"))
        .collect();
    paths.sort();
    if paths.is_empty() {
        return Err(format!("no .pix files found in '{}'", directory.display()));
    }

    let charset = gen_charset_images(false, 8, 8, &C64LOW, &C64UP);
    let topology_catalog = build_topology_catalog(&charset);
    let spur_catalog: Vec<_> = charset
        .iter()
        .map(|bitmap| bitmap_spur_penalty(bitmap) > 0.0)
        .collect();
    let mut report = CorpusReport {
        discovered_files: paths.len(),
        files: 0,
        invalid_files: Vec::new(),
        cells: 0,
        glyph_counts: vec![0; 256],
        role_counts: BTreeMap::new(),
        active_side_histogram: vec![0; 5],
        visible_role_counts: BTreeMap::new(),
        visible_active_side_histogram: vec![0; 5],
        visible_edge_cells: 0,
        exact_port_connections: 0,
        tolerant_port_connections: 0,
        misaligned_port_connections: 0,
        one_sided_port_adjacencies: 0,
        mismatched_port_adjacencies: 0,
        internal_unexpected_endpoints: 0,
        image_border_endpoints: 0,
        glyph_junction_cells: 0,
        glyph_spur_cells: 0,
        junction_cells: 0,
        spur_cells: 0,
        same_foreground_background_cells: 0,
        foreground_palette_counts: vec![0; 256],
        background_palette_counts: vec![0; 256],
        color_pair_counts: BTreeMap::new(),
    };

    for path in paths {
        match parse_pix(&path) {
            Ok(artwork) => {
                report.files += 1;
                analyze_artwork(&artwork, &topology_catalog, &spur_catalog, &mut report);
            }
            Err(error) => report.invalid_files.push(error),
        }
    }
    Ok(report)
}

fn analyze_artwork(
    artwork: &PixArtwork,
    topology_catalog: &[GlyphTopology],
    spur_catalog: &[bool],
    report: &mut CorpusReport,
) {
    report.cells += artwork.cells.len() as u64;
    let blank_topology = topology_catalog[32];
    for (index, cell) in artwork.cells.iter().copied().enumerate() {
        let glyph_topology = topology_catalog[cell.glyph as usize];
        let topology = if cell.foreground == cell.background {
            blank_topology
        } else {
            glyph_topology
        };
        report.glyph_counts[cell.glyph as usize] += 1;
        *report
            .role_counts
            .entry(role_name(glyph_topology.role()).to_string())
            .or_default() += 1;
        report.active_side_histogram[glyph_topology.active_sides()] += 1;
        *report
            .visible_role_counts
            .entry(role_name(topology.role()).to_string())
            .or_default() += 1;
        report.visible_active_side_histogram[topology.active_sides()] += 1;
        report.visible_edge_cells += (topology.active_sides() > 0) as u64;
        report.glyph_junction_cells += (glyph_topology.active_sides() >= 3) as u64;
        report.glyph_spur_cells += spur_catalog[cell.glyph as usize] as u64;
        report.junction_cells += (topology.active_sides() >= 3) as u64;
        report.spur_cells +=
            (cell.foreground != cell.background && spur_catalog[cell.glyph as usize]) as u64;
        report.same_foreground_background_cells += (cell.foreground == cell.background) as u64;
        report.foreground_palette_counts[cell.foreground as usize] += 1;
        report.background_palette_counts[cell.background as usize] += 1;
        *report
            .color_pair_counts
            .entry(format!("{}:{}", cell.foreground, cell.background))
            .or_default() += 1;

        let x = index % artwork.width;
        let y = index / artwork.width;
        for side in Side::ALL {
            let ports = topology.edge_ports(side);
            if ports == 0 {
                continue;
            }
            let neighbor = neighbor_index(x, y, artwork.width, artwork.height, side);
            let Some(neighbor) = neighbor else {
                report.image_border_endpoints += 1;
                continue;
            };
            let neighbor_cell = artwork.cells[neighbor];
            let neighbor_topology = if neighbor_cell.foreground == neighbor_cell.background {
                blank_topology
            } else {
                topology_catalog[neighbor_cell.glyph as usize]
            };
            if !ports_tolerantly_connect(ports, neighbor_topology.edge_ports(side.opposite())) {
                report.internal_unexpected_endpoints += 1;
            }
        }

        for side in [Side::Right, Side::Bottom] {
            let Some(neighbor) = neighbor_index(x, y, artwork.width, artwork.height, side) else {
                continue;
            };
            let first_ports = topology.edge_ports(side);
            let neighbor_cell = artwork.cells[neighbor];
            let neighbor_topology = if neighbor_cell.foreground == neighbor_cell.background {
                blank_topology
            } else {
                topology_catalog[neighbor_cell.glyph as usize]
            };
            let second_ports = neighbor_topology.edge_ports(side.opposite());
            if first_ports == 0 && second_ports == 0 {
                continue;
            }
            if first_ports == 0 || second_ports == 0 {
                report.one_sided_port_adjacencies += 1;
                report.mismatched_port_adjacencies += 1;
            } else if first_ports == second_ports {
                report.exact_port_connections += 1;
            } else if ports_tolerantly_connect(first_ports, second_ports) {
                report.tolerant_port_connections += 1;
            } else {
                report.misaligned_port_connections += 1;
                report.mismatched_port_adjacencies += 1;
            }
        }
    }
}

fn parse_pix(path: &Path) -> Result<PixArtwork, String> {
    let content = fs::read_to_string(path)
        .map_err(|error| format!("failed to read '{}': {error}", path.display()))?;
    let mut lines = content.lines();
    let header = lines
        .find_map(|line| {
            let line = line.trim_start_matches('\u{feff}');
            line.starts_with("width=").then_some(line)
        })
        .ok_or_else(|| format!("'{}' has no PIX header", path.display()))?;
    let mut width = None;
    let mut height = None;
    for field in header.split(',') {
        let Some((name, value)) = field.split_once('=') else {
            continue;
        };
        match name.trim() {
            "width" => width = value.trim().parse::<usize>().ok(),
            "height" => height = value.trim().parse::<usize>().ok(),
            _ => {}
        }
    }
    let width = width.ok_or_else(|| format!("'{}' has no valid width", path.display()))?;
    let height = height.ok_or_else(|| format!("'{}' has no valid height", path.display()))?;
    let mut cells = Vec::with_capacity(width * height);
    for token in lines.flat_map(str::split_whitespace) {
        let fields: Vec<_> = token.split(',').collect();
        if fields.len() < 3 {
            return Err(format!("'{}' has invalid cell '{token}'", path.display()));
        }
        let parse = |field: &str| {
            field.parse::<u8>().map_err(|error| {
                format!("'{}' has invalid cell '{token}': {error}", path.display())
            })
        };
        cells.push(PixCell {
            glyph: parse(fields[0])?,
            foreground: parse(fields[1])?,
            background: if fields.len() >= 4 {
                parse(fields[3])?
            } else {
                0
            },
        });
    }
    if cells.len() != width * height {
        return Err(format!(
            "'{}' declares {}x{} but contains {} cells",
            path.display(),
            width,
            height,
            cells.len()
        ));
    }
    Ok(PixArtwork {
        width,
        height,
        cells,
    })
}

fn neighbor_index(x: usize, y: usize, width: usize, height: usize, side: Side) -> Option<usize> {
    match side {
        Side::Top => y.checked_sub(1).map(|ny| ny * width + x),
        Side::Right => (x + 1 < width).then_some(y * width + x + 1),
        Side::Bottom => (y + 1 < height).then_some((y + 1) * width + x),
        Side::Left => x.checked_sub(1).map(|nx| y * width + nx),
    }
}

fn ports_tolerantly_connect(first: u8, second: u8) -> bool {
    first != 0 && second != 0 && first & (second | (second << 1) | (second >> 1)) & 0x7f != 0
}

fn role_name(role: GlyphRole) -> &'static str {
    match role {
        GlyphRole::Blank => "blank",
        GlyphRole::Solid => "solid",
        GlyphRole::Texture => "texture",
        GlyphRole::Endpoint => "endpoint",
        GlyphRole::Straight => "straight",
        GlyphRole::Corner => "corner",
        GlyphRole::Junction => "junction",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn corpus_report_is_sorted_and_deterministic() {
        let directory = std::env::temp_dir().join(format!("petii-corpus-{}", std::process::id()));
        let _ = fs::remove_dir_all(&directory);
        fs::create_dir_all(&directory).unwrap();
        fs::write(
            directory.join("b.pix"),
            "width=2,height=1,texture=255\n32,0,1,0 160,1,1,0\n",
        )
        .unwrap();
        fs::write(
            directory.join("a.pix"),
            "historical command\n\u{feff}width=1,height=1,texture=255\n32,0,1,0\n",
        )
        .unwrap();

        let first = analyze_pix_corpus(&directory).unwrap();
        let second = analyze_pix_corpus(&directory).unwrap();
        fs::remove_dir_all(&directory).unwrap();

        assert_eq!(first, second);
        assert_eq!(first.files, 2);
        assert_eq!(first.discovered_files, 2);
        assert!(first.invalid_files.is_empty());
        assert_eq!(first.cells, 3);
        assert_eq!(first.glyph_counts[32], 2);
        assert_eq!(
            serde_json::to_vec(&first).unwrap(),
            serde_json::to_vec(&second).unwrap()
        );
    }
}
