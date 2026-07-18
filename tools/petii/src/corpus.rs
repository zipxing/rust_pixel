use crate::c64::{C64LOW, C64UP};
use crate::converter::bitmap_spur_penalty;
use crate::glyph_topology::{build_topology_catalog, GlyphRole, GlyphTopology, Side};
use crate::types::PetsciiGrid;
use rust_pixel::render::symbols::gen_charset_images;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

const GLYPHS: usize = 256;
/// Laplace smoothing so unseen glyphs and adjacencies keep a finite, comparable penalty.
const PRIOR_SMOOTHING: f64 = 0.5;

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
    /// Visible-glyph unigram counts (cells with `fg == bg` count as space). This is the "as
    /// perceived" distribution used to score how human-like a conversion's glyph vocabulary is.
    pub visible_glyph_counts: Vec<u64>,
    /// Visible-glyph horizontal adjacency counts, keyed "left:right".
    pub horizontal_bigram_counts: BTreeMap<String, u64>,
    /// Visible-glyph vertical adjacency counts, keyed "top:bottom".
    pub vertical_bigram_counts: BTreeMap<String, u64>,
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
        visible_glyph_counts: vec![0; 256],
        horizontal_bigram_counts: BTreeMap::new(),
        vertical_bigram_counts: BTreeMap::new(),
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
        report.visible_glyph_counts[visible_glyph(cell) as usize] += 1;

        let x = index % artwork.width;
        let y = index / artwork.width;
        for (side, counts) in [
            (Side::Right, &mut report.horizontal_bigram_counts),
            (Side::Bottom, &mut report.vertical_bigram_counts),
        ] {
            if let Some(neighbor) = neighbor_index(x, y, artwork.width, artwork.height, side) {
                let key = format!(
                    "{}:{}",
                    visible_glyph(cell),
                    visible_glyph(artwork.cells[neighbor])
                );
                *counts.entry(key).or_default() += 1;
            }
        }
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

/// The glyph a cell actually shows. Cells whose foreground equals their background render as an
/// empty space regardless of the stored glyph id, so they count as space for perceptual stats.
fn visible_glyph(cell: PixCell) -> u8 {
    if cell.foreground == cell.background {
        32
    } else {
        cell.glyph
    }
}

/// How human-like a conversion's glyph vocabulary and local layout are, measured as the mean
/// negative log-likelihood of its visible glyphs and adjacencies under a corpus of hand-authored
/// PETSCII art. Lower is more human-like. The two components are reported separately so a caller
/// can see whether a change moved vocabulary, layout, or both.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct NaturalnessScore {
    pub unigram_nll: f64,
    pub bigram_nll: f64,
}

/// Smoothed log-probability tables derived from a [`CorpusReport`], used to score conversions.
pub struct CorpusPrior {
    unigram_logp: Vec<f64>,
    horizontal_logp: Vec<f64>,
    vertical_logp: Vec<f64>,
}

impl CorpusPrior {
    /// Load a serialized [`CorpusReport`] and precompute its log-probability tables.
    pub fn load(path: &Path) -> Result<Self, String> {
        let report: CorpusReport = serde_json::from_slice(
            &fs::read(path).map_err(|error| format!("failed to read {}: {error}", path.display()))?,
        )
        .map_err(|error| format!("invalid corpus report {}: {error}", path.display()))?;
        Ok(Self::from_report(&report))
    }

    pub fn from_report(report: &CorpusReport) -> Self {
        let unigram_logp = smoothed_unigram_logp(&report.visible_glyph_counts);
        let horizontal_logp = smoothed_bigram_logp(&report.horizontal_bigram_counts);
        let vertical_logp = smoothed_bigram_logp(&report.vertical_bigram_counts);
        Self {
            unigram_logp,
            horizontal_logp,
            vertical_logp,
        }
    }

    /// Log-probability of `second` following `first` along a row (`vertical == false`) or down a
    /// column (`vertical == true`). Used to price how natural a local adjacency is.
    pub fn bigram_logp(&self, first: u8, second: u8, vertical: bool) -> f64 {
        let table = if vertical {
            &self.vertical_logp
        } else {
            &self.horizontal_logp
        };
        table[first as usize * GLYPHS + second as usize]
    }

    /// Score a converted grid. Visible glyphs mirror the corpus convention (`fg == bg` is space).
    pub fn naturalness(&self, grid: &PetsciiGrid) -> NaturalnessScore {
        let visible = |x: u32, y: u32| -> usize {
            let cell = grid.get(x, y);
            if cell.fg == cell.bg {
                32
            } else {
                cell.glyph as usize
            }
        };
        let mut unigram_sum = 0.0;
        for y in 0..grid.height {
            for x in 0..grid.width {
                unigram_sum -= self.unigram_logp[visible(x, y)];
            }
        }
        let cells = (grid.width * grid.height).max(1) as f64;

        let mut bigram_sum = 0.0;
        let mut adjacencies = 0u64;
        for y in 0..grid.height {
            for x in 0..grid.width {
                if x + 1 < grid.width {
                    bigram_sum -= self.horizontal_logp[visible(x, y) * GLYPHS + visible(x + 1, y)];
                    adjacencies += 1;
                }
                if y + 1 < grid.height {
                    bigram_sum -= self.vertical_logp[visible(x, y) * GLYPHS + visible(x, y + 1)];
                    adjacencies += 1;
                }
            }
        }
        NaturalnessScore {
            unigram_nll: unigram_sum / cells,
            bigram_nll: bigram_sum / adjacencies.max(1) as f64,
        }
    }
}

fn smoothed_unigram_logp(counts: &[u64]) -> Vec<f64> {
    let total: u64 = counts.iter().sum();
    let denominator = total as f64 + PRIOR_SMOOTHING * GLYPHS as f64;
    counts
        .iter()
        .map(|count| ((*count as f64 + PRIOR_SMOOTHING) / denominator).ln())
        .collect()
}

/// Build a dense `256*256` table of `log P(second | first)` from sparse "first:second" counts.
fn smoothed_bigram_logp(counts: &BTreeMap<String, u64>) -> Vec<f64> {
    let mut joint = vec![0u64; GLYPHS * GLYPHS];
    let mut row_totals = vec![0u64; GLYPHS];
    for (key, count) in counts {
        if let Some((first, second)) = parse_pair(key) {
            joint[first * GLYPHS + second] += *count;
            row_totals[first] += *count;
        }
    }
    let mut logp = vec![0.0; GLYPHS * GLYPHS];
    for first in 0..GLYPHS {
        let denominator = row_totals[first] as f64 + PRIOR_SMOOTHING * GLYPHS as f64;
        for second in 0..GLYPHS {
            let numerator = joint[first * GLYPHS + second] as f64 + PRIOR_SMOOTHING;
            logp[first * GLYPHS + second] = (numerator / denominator).ln();
        }
    }
    logp
}

fn parse_pair(key: &str) -> Option<(usize, usize)> {
    let (first, second) = key.split_once(':')?;
    Some((first.parse().ok()?, second.parse().ok()?))
}

#[cfg(test)]
pub(crate) fn empty_corpus_report() -> CorpusReport {
    CorpusReport {
        discovered_files: 0,
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
        visible_glyph_counts: vec![0; 256],
        horizontal_bigram_counts: BTreeMap::new(),
        vertical_bigram_counts: BTreeMap::new(),
    }
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
    use crate::PetsciiCell;

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
        // b.pix: [space(32) fg==bg, solid(160) fg!=bg]; a.pix: [space]. Visible: 32 twice, 160 once.
        assert_eq!(first.visible_glyph_counts[32], 2);
        assert_eq!(first.visible_glyph_counts[160], 1);
        // The only horizontal adjacency is space next to the solid block in b.pix.
        assert_eq!(first.horizontal_bigram_counts.get("32:160"), Some(&1));
        assert!(first.vertical_bigram_counts.is_empty());
        assert_eq!(
            serde_json::to_vec(&first).unwrap(),
            serde_json::to_vec(&second).unwrap()
        );
    }

    #[test]
    fn corpus_prior_scores_common_glyphs_as_more_human_like() {
        let mut report = CorpusReport {
            discovered_files: 0,
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
            visible_glyph_counts: vec![0; 256],
            horizontal_bigram_counts: BTreeMap::new(),
            vertical_bigram_counts: BTreeMap::new(),
        };
        // A corpus dominated by space next to space, the real PETSCII majority.
        report.visible_glyph_counts[32] = 1000;
        report.visible_glyph_counts[102] = 50;
        report.horizontal_bigram_counts.insert("32:32".into(), 900);
        report.vertical_bigram_counts.insert("32:32".into(), 900);
        let prior = CorpusPrior::from_report(&report);

        let space = PetsciiCell {
            glyph: 32,
            fg: 0,
            bg: 0,
            texture: 1,
        };
        let rare = PetsciiCell {
            glyph: 200,
            fg: 1,
            bg: 0,
            texture: 1,
        };
        let common_grid = PetsciiGrid::new(2, 2, vec![space; 4]).unwrap();
        let rare_grid = PetsciiGrid::new(2, 2, vec![rare; 4]).unwrap();
        let common = prior.naturalness(&common_grid);
        let rare = prior.naturalness(&rare_grid);
        assert!(common.unigram_nll < rare.unigram_nll);
        assert!(common.bigram_nll < rare.bigram_nll);
    }
}
