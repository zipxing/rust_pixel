use crate::c64::{C64LOW, C64UP};
use crate::types::{ConversionConfig, GlyphCandidate, PetsciiGrid};
use image::{DynamicImage, GrayImage, Luma};
use rust_pixel::render::style::ANSI_COLOR_RGB;
use rust_pixel::render::symbols::{
    binarize_grayscale_block, calculate_mse, find_background_color, find_best_color,
    find_best_color_u32, gen_charset_images, get_block_color, get_grayscale_block_at,
    get_petii_block_color, BlockGrayImage,
};
use std::collections::BTreeSet;

pub const GLYPH_WIDTH: u32 = 8;
pub const GLYPH_HEIGHT: u32 = 8;
const SPACE_GLYPH: u8 = 32;
const SOLID_GLYPH: u8 = SPACE_GLYPH + 128;
const FLAT_BACKGROUND_RANGE: u8 = 18;
const BACKGROUND_MEAN_TOLERANCE: i16 = 24;
const EDGE_CELL_MEAN_THRESHOLD: u32 = 18;
const EDGE_WEAK_THRESHOLD: u8 = 32;
const EDGE_STRONG_THRESHOLD: u8 = 96;
const EDGE_MIN_COMPONENT_PIXELS: usize = 8;
const EDGE_CONTINUITY_CANDIDATES: usize = 16;
const EDGE_CONTINUITY_WEIGHT: f64 = 0.28;
const EDGE_SPUR_WEIGHT: f64 = 0.3;
const EDGE_NEIGHBORHOOD_SPUR_WEIGHT: f64 = 0.22;
const EDGE_CONTINUITY_PASSES: usize = 4;
const EQUALS_MAX_DISTANCE_REGRESSION: f64 = 0.1;

#[derive(Debug, Clone)]
pub struct ConversionResult {
    pub grid: PetsciiGrid,
    /// Alternatives are row-major and always include the selected baseline first.
    pub alternatives: Vec<Vec<GlyphCandidate>>,
    /// Exact preprocessed image used by candidate generation and scoring.
    pub reference: DynamicImage,
}

/// Generate bounded deterministic preprocessing variants. The first item is always the
/// caller-provided baseline configuration.
pub fn generate_config_variants(base: &ConversionConfig) -> Vec<ConversionConfig> {
    let mut variants = vec![base.clone()];
    for contrast in [-18.0, 18.0, 36.0] {
        let mut config = base.clone();
        config.contrast = contrast;
        if !variants.contains(&config) {
            variants.push(config);
        }
    }
    variants
}

pub fn convert_image(
    image: &DynamicImage,
    config: &ConversionConfig,
) -> Result<ConversionResult, String> {
    config.validate()?;

    let adjusted = if config.contrast.abs() > f32::EPSILON {
        image.adjust_contrast(config.contrast)
    } else {
        image.clone()
    };
    let reference = adjusted.resize_exact(
        config.width * GLYPH_WIDTH,
        config.height * GLYPH_HEIGHT,
        image::imageops::FilterType::Lanczos3,
    );
    let gray = reference.clone().into_luma8();
    let edge_image = clean_edge_image(&sobel_image(&gray));
    let charset = gen_charset_images(
        false,
        GLYPH_WIDTH as usize,
        GLYPH_HEIGHT as usize,
        &C64LOW,
        &C64UP,
    );

    let (background_gray, background_rgb) =
        find_background_color(&reference, &gray, reference.width(), reference.height());
    let background_color = find_best_color_u32(background_rgb) as u8;

    let mut cells = Vec::with_capacity((config.width * config.height) as usize);
    let mut alternatives = Vec::with_capacity(cells.capacity());
    let mut edge_cells = Vec::with_capacity(cells.capacity());

    for y in 0..config.height {
        for x in 0..config.width {
            let raw_block = get_grayscale_block_at(&gray, x, y, GLYPH_WIDTH, GLYPH_HEIGHT);
            let edge_block = get_grayscale_block_at(&edge_image, x, y, GLYPH_WIDTH, GLYPH_HEIGHT);
            let flat_mean = (config.mode != 1)
                .then(|| flat_block_mean(&raw_block))
                .flatten();
            let edge_aware = config.mode != 1 && flat_mean.is_none() && is_edge_cell(&edge_block);
            let color_mode = if edge_aware { 1 } else { config.mode };
            let (bg, fg) = colors_for_cell(&reference, &gray, x, y, color_mode, background_rgb);
            let edge_target = edge_aware.then(|| EdgeTarget::new(&raw_block, fg, bg));
            // Exact extraction binarizes an already-PETSCII cell. General-image
            // modes retain grayscale structure for nearest-glyph matching.
            let block = if config.mode == 1 {
                binarize_grayscale_block(
                    &raw_block,
                    background_gray,
                    GLYPH_WIDTH as usize,
                    GLYPH_HEIGHT as usize,
                )
            } else {
                raw_block
            };
            let ranked = match flat_mean {
                Some(mean)
                    if (mean as i16 - background_gray as i16).abs()
                        <= BACKGROUND_MEAN_TOLERANCE =>
                {
                    vec![solid_candidate(
                        SPACE_GLYPH,
                        background_color,
                        background_color,
                    )]
                }
                Some(_) => vec![solid_candidate(SOLID_GLYPH, fg, bg)],
                None => rank_glyphs(
                    &block,
                    &charset,
                    config.mode,
                    if edge_aware {
                        config.top_k.max(EDGE_CONTINUITY_CANDIDATES)
                    } else {
                        config.top_k
                    },
                    fg,
                    bg,
                    edge_target.as_ref(),
                ),
            };
            cells.push(ranked[0].cell());
            alternatives.push(ranked);
            edge_cells.push(edge_aware);
        }
    }

    let selected = select_continuous_edges(
        config.width,
        config.height,
        &alternatives,
        &edge_cells,
        &charset,
    );
    let selected = refine_equals_noise(
        config.width,
        config.height,
        &alternatives,
        &charset,
        selected,
    );
    for (index, selected_index) in selected.into_iter().enumerate() {
        let selected_candidate = alternatives[index][selected_index];
        cells[index] = selected_candidate.cell();
        if selected_index != 0 {
            alternatives[index].swap(0, selected_index);
        }
        alternatives[index].truncate(config.top_k);
    }

    Ok(ConversionResult {
        grid: PetsciiGrid::new(config.width, config.height, cells)?,
        alternatives,
        reference,
    })
}

fn colors_for_cell(
    reference: &DynamicImage,
    gray: &GrayImage,
    x: u32,
    y: u32,
    mode: u8,
    background_rgb: u32,
) -> (u8, u8) {
    if mode == 1 {
        let (bg, fg) = get_petii_block_color(
            reference,
            gray,
            x,
            y,
            background_rgb,
            GLYPH_WIDTH,
            GLYPH_HEIGHT,
        );
        (bg as u8, fg as u8)
    } else {
        let color = get_block_color(reference, x, y, GLYPH_WIDTH, GLYPH_HEIGHT);
        (0, find_best_color(color) as u8)
    }
}

fn rank_glyphs(
    input: &BlockGrayImage,
    charset: &[BlockGrayImage],
    mode: u8,
    top_k: usize,
    fg: u8,
    bg: u8,
    edge_target: Option<&EdgeTarget>,
) -> Vec<GlyphCandidate> {
    let mut ranked: Vec<_> = charset
        .iter()
        .enumerate()
        .filter(|(glyph, _)| glyph_allowed(mode, *glyph as u8))
        .map(|(glyph, bitmap)| {
            let distance = edge_target.map_or_else(
                || calculate_mse(input, bitmap, GLYPH_WIDTH as usize, GLYPH_HEIGHT as usize),
                |target| target.distance(bitmap),
            );
            GlyphCandidate {
                glyph: glyph as u8,
                distance,
                fg,
                bg,
                texture: 1,
            }
        })
        .collect();
    ranked.sort_by(|a, b| {
        a.distance
            .total_cmp(&b.distance)
            .then_with(|| a.glyph.cmp(&b.glyph))
    });
    ranked.truncate(top_k.min(ranked.len()));
    ranked
}

fn glyph_allowed(mode: u8, glyph: u8) -> bool {
    if mode != 2 {
        return true;
    }
    let base = glyph % 128;
    !((1..=26).contains(&base) || (48..=57).contains(&base))
}

fn solid_candidate(glyph: u8, fg: u8, bg: u8) -> GlyphCandidate {
    GlyphCandidate {
        glyph,
        distance: 0.0,
        fg,
        bg,
        texture: 1,
    }
}

fn flat_block_mean(block: &BlockGrayImage) -> Option<u8> {
    let mut min = u8::MAX;
    let mut max = u8::MIN;
    let mut sum = 0u32;
    let mut count = 0u32;
    for pixel in block.iter().flatten().copied() {
        min = min.min(pixel);
        max = max.max(pixel);
        sum += pixel as u32;
        count += 1;
    }
    if count == 0 || max.saturating_sub(min) > FLAT_BACKGROUND_RANGE {
        return None;
    }
    Some((sum / count) as u8)
}

fn sobel_image(gray: &GrayImage) -> GrayImage {
    let mut edges = GrayImage::new(gray.width(), gray.height());
    if gray.width() < 3 || gray.height() < 3 {
        return edges;
    }
    for y in 1..gray.height() - 1 {
        for x in 1..gray.width() - 1 {
            let sample = |dx: i32, dy: i32| {
                gray.get_pixel((x as i32 + dx) as u32, (y as i32 + dy) as u32)
                    .0[0] as i32
            };
            let gx = -sample(-1, -1) + sample(1, -1) - 2 * sample(-1, 0) + 2 * sample(1, 0)
                - sample(-1, 1)
                + sample(1, 1);
            let gy = -sample(-1, -1) - 2 * sample(0, -1) - sample(1, -1)
                + sample(-1, 1)
                + 2 * sample(0, 1)
                + sample(1, 1);
            edges.put_pixel(x, y, Luma([(gx.abs() + gy.abs()).min(255) as u8]));
        }
    }
    edges
}

/// Keep weak Sobel pixels only when they belong to a meaningful component that
/// also contains a strong edge. This is performed before splitting the image
/// into character cells, so real thin contours may continue across cell borders.
fn clean_edge_image(edges: &GrayImage) -> GrayImage {
    let width = edges.width();
    let height = edges.height();
    let mut cleaned = GrayImage::new(width, height);
    let mut visited = vec![false; (width * height) as usize];

    for start_y in 0..height {
        for start_x in 0..width {
            let start = (start_y * width + start_x) as usize;
            if visited[start] || edges.get_pixel(start_x, start_y).0[0] < EDGE_WEAK_THRESHOLD {
                continue;
            }

            let mut stack = vec![(start_x, start_y)];
            let mut component = Vec::new();
            let mut has_strong_edge = false;
            visited[start] = true;

            while let Some((x, y)) = stack.pop() {
                component.push((x, y));
                has_strong_edge |= edges.get_pixel(x, y).0[0] >= EDGE_STRONG_THRESHOLD;
                for dy in -1i32..=1 {
                    for dx in -1i32..=1 {
                        if dx == 0 && dy == 0 {
                            continue;
                        }
                        let nx = x as i32 + dx;
                        let ny = y as i32 + dy;
                        if nx < 0 || ny < 0 || nx >= width as i32 || ny >= height as i32 {
                            continue;
                        }
                        let nx = nx as u32;
                        let ny = ny as u32;
                        let index = (ny * width + nx) as usize;
                        if !visited[index] && edges.get_pixel(nx, ny).0[0] >= EDGE_WEAK_THRESHOLD {
                            visited[index] = true;
                            stack.push((nx, ny));
                        }
                    }
                }
            }

            if has_strong_edge && component.len() >= EDGE_MIN_COMPONENT_PIXELS {
                for (x, y) in component {
                    cleaned.put_pixel(x, y, *edges.get_pixel(x, y));
                }
            }
        }
    }
    cleaned
}

fn is_edge_cell(edge_block: &BlockGrayImage) -> bool {
    let (sum, count) = edge_block
        .iter()
        .flatten()
        .fold((0u32, 0u32), |(sum, count), pixel| {
            (sum + *pixel as u32, count + 1)
        });
    count > 0 && sum / count >= EDGE_CELL_MEAN_THRESHOLD
}

struct EdgeTarget {
    mask: BlockGrayImage,
    edges: BlockGrayImage,
}

impl EdgeTarget {
    fn new(input: &BlockGrayImage, fg: u8, bg: u8) -> Self {
        let fg_luma = palette_luma(fg);
        let bg_luma = palette_luma(bg);
        let mut mask = vec![vec![0u8; GLYPH_WIDTH as usize]; GLYPH_HEIGHT as usize];
        if (fg_luma - bg_luma).abs() < f32::EPSILON {
            let min = input.iter().flatten().copied().min().unwrap_or(0) as u16;
            let max = input.iter().flatten().copied().max().unwrap_or(0) as u16;
            let threshold = (min + max) / 2;
            for (source_row, target_row) in input.iter().zip(mask.iter_mut()) {
                for (source, target) in source_row.iter().zip(target_row.iter_mut()) {
                    *target = if *source as u16 > threshold { 255 } else { 0 };
                }
            }
        } else {
            for (source_row, target_row) in input.iter().zip(mask.iter_mut()) {
                for (source, target) in source_row.iter().zip(target_row.iter_mut()) {
                    let luma = *source as f32;
                    *target = if (luma - fg_luma).abs() <= (luma - bg_luma).abs() {
                        255
                    } else {
                        0
                    };
                }
            }
        }
        let edges = sobel_block(&mask);
        Self { mask, edges }
    }

    fn distance(&self, glyph: &BlockGrayImage) -> f64 {
        let glyph_edges = sobel_block(glyph);
        let mut mask_mismatch = 0u32;
        let mut edge_intersection = 0u32;
        let mut edge_union = 0u32;
        for y in 0..GLYPH_HEIGHT as usize {
            for x in 0..GLYPH_WIDTH as usize {
                if self.mask[y][x] != glyph[y][x] {
                    mask_mismatch += 1;
                }
                let target_edge = self.edges[y][x] > 0;
                let glyph_edge = glyph_edges[y][x] > 0;
                edge_intersection += (target_edge && glyph_edge) as u32;
                edge_union += (target_edge || glyph_edge) as u32;
            }
        }
        let mask_loss = mask_mismatch as f64 / (GLYPH_WIDTH * GLYPH_HEIGHT) as f64;
        let edge_loss = if edge_union == 0 {
            0.0
        } else {
            1.0 - edge_intersection as f64 / edge_union as f64
        };
        0.7 * mask_loss + 0.3 * edge_loss
    }
}

#[derive(Clone, Copy)]
enum Border {
    Top,
    Right,
    Bottom,
    Left,
}

fn select_continuous_edges(
    width: u32,
    height: u32,
    alternatives: &[Vec<GlyphCandidate>],
    edge_cells: &[bool],
    charset: &[BlockGrayImage],
) -> Vec<usize> {
    let mut selected = vec![0usize; alternatives.len()];
    for pass in 0..EDGE_CONTINUITY_PASSES {
        let reverse = pass % 2 == 1;
        for step in 0..alternatives.len() {
            let index = if reverse {
                alternatives.len() - 1 - step
            } else {
                step
            };
            if !edge_cells[index] || alternatives[index].len() < 2 {
                continue;
            }
            let x = index as u32 % width;
            let y = index as u32 / width;
            let mut best = selected[index];
            let mut best_score = f64::INFINITY;
            for (candidate_index, candidate) in alternatives[index].iter().enumerate() {
                let mut continuity = 0.0;
                let mut neighbor_count = 0usize;
                for (neighbor_index, own_border, neighbor_border) in [
                    (
                        y.checked_sub(1).map(|ny| (ny * width + x) as usize),
                        Border::Top,
                        Border::Bottom,
                    ),
                    (
                        (x + 1 < width).then(|| (y * width + x + 1) as usize),
                        Border::Right,
                        Border::Left,
                    ),
                    (
                        (y + 1 < height).then(|| ((y + 1) * width + x) as usize),
                        Border::Bottom,
                        Border::Top,
                    ),
                    (
                        x.checked_sub(1).map(|nx| (y * width + nx) as usize),
                        Border::Left,
                        Border::Right,
                    ),
                ] {
                    if let Some(neighbor_index) = neighbor_index {
                        let neighbor = alternatives[neighbor_index][selected[neighbor_index]];
                        continuity += border_mismatch(
                            *candidate,
                            own_border,
                            neighbor,
                            neighbor_border,
                            charset,
                        );
                        neighbor_count += 1;
                    }
                }
                let continuity = continuity / neighbor_count.max(1) as f64;
                let spur = bitmap_spur_penalty(&charset[candidate.glyph as usize]);
                let (neighborhood_spur, _) = neighborhood_artifact_penalties(
                    index,
                    *candidate,
                    width,
                    height,
                    alternatives,
                    &selected,
                    charset,
                );
                let score = candidate.distance
                    + EDGE_CONTINUITY_WEIGHT * continuity
                    + EDGE_SPUR_WEIGHT * spur
                    + EDGE_NEIGHBORHOOD_SPUR_WEIGHT * neighborhood_spur;
                if score < best_score
                    || (score == best_score && candidate.glyph < alternatives[index][best].glyph)
                {
                    best = candidate_index;
                    best_score = score;
                }
            }
            selected[index] = best;
        }
    }
    selected
}

fn refine_equals_noise(
    width: u32,
    height: u32,
    alternatives: &[Vec<GlyphCandidate>],
    charset: &[BlockGrayImage],
    mut selected: Vec<usize>,
) -> Vec<usize> {
    let mut colors = render_selected_colors(width, height, alternatives, &selected, charset);
    for _ in 0..2 {
        let (mut pair_count, targets) = global_equals_targets(&colors, width, height);
        if pair_count == 0 {
            break;
        }
        let mut changed = false;
        for index in targets {
            if alternatives[index].len() < 2 {
                continue;
            }
            let current_index = selected[index];
            let current = alternatives[index][current_index];
            let current_fragmentation =
                bitmap_fragmentation_penalty(&charset[current.glyph as usize]);
            let mut best = current_index;
            let mut best_pairs = pair_count;
            let mut best_distance = current.distance;

            for (candidate_index, candidate) in alternatives[index].iter().enumerate() {
                if candidate_index == current_index
                    || candidate.distance > current.distance + EQUALS_MAX_DISTANCE_REGRESSION
                    || bitmap_fragmentation_penalty(&charset[candidate.glyph as usize])
                        > current_fragmentation + f64::EPSILON
                {
                    continue;
                }
                paint_candidate(&mut colors, index, width, *candidate, charset);
                let candidate_pairs = global_equals_targets(&colors, width, height).0;
                paint_candidate(&mut colors, index, width, current, charset);
                if candidate_pairs < best_pairs
                    || (candidate_pairs == best_pairs && candidate.distance < best_distance)
                {
                    best = candidate_index;
                    best_pairs = candidate_pairs;
                    best_distance = candidate.distance;
                }
            }
            if best != current_index && best_pairs < pair_count {
                selected[index] = best;
                paint_candidate(
                    &mut colors,
                    index,
                    width,
                    alternatives[index][best],
                    charset,
                );
                pair_count = best_pairs;
                changed = true;
            }
        }
        if !changed {
            break;
        }
    }
    selected
}

fn render_selected_colors(
    width: u32,
    height: u32,
    alternatives: &[Vec<GlyphCandidate>],
    selected: &[usize],
    charset: &[BlockGrayImage],
) -> Vec<Vec<u8>> {
    let mut colors =
        vec![vec![0u8; (width * GLYPH_WIDTH) as usize]; (height * GLYPH_HEIGHT) as usize];
    for index in 0..alternatives.len() {
        paint_candidate(
            &mut colors,
            index,
            width,
            alternatives[index][selected[index]],
            charset,
        );
    }
    colors
}

fn paint_candidate(
    colors: &mut [Vec<u8>],
    index: usize,
    width: u32,
    candidate: GlyphCandidate,
    charset: &[BlockGrayImage],
) {
    let cell_x = index as u32 % width;
    let cell_y = index as u32 / width;
    let bitmap = &charset[candidate.glyph as usize];
    for y in 0..GLYPH_HEIGHT as usize {
        for x in 0..GLYPH_WIDTH as usize {
            colors[cell_y as usize * 8 + y][cell_x as usize * 8 + x] = if bitmap[y][x] >= 128 {
                candidate.fg
            } else {
                candidate.bg
            };
        }
    }
}

fn neighborhood_artifact_penalties(
    center_index: usize,
    center: GlyphCandidate,
    width: u32,
    height: u32,
    alternatives: &[Vec<GlyphCandidate>],
    selected: &[usize],
    charset: &[BlockGrayImage],
) -> (f64, f64) {
    const PATCH_CELLS: usize = 3;
    const PATCH_SIZE: usize = PATCH_CELLS * GLYPH_WIDTH as usize;
    let center_x = center_index as u32 % width;
    let center_y = center_index as u32 / width;
    let mut colors = vec![vec![center.bg; PATCH_SIZE]; PATCH_SIZE];
    for cell_y in 0..PATCH_CELLS {
        for cell_x in 0..PATCH_CELLS {
            let grid_x = center_x as i32 + cell_x as i32 - 1;
            let grid_y = center_y as i32 + cell_y as i32 - 1;
            if grid_x < 0 || grid_y < 0 || grid_x >= width as i32 || grid_y >= height as i32 {
                continue;
            }
            let index = (grid_y as u32 * width + grid_x as u32) as usize;
            let candidate = if cell_x == 1 && cell_y == 1 {
                center
            } else {
                alternatives[index][selected[index]]
            };
            let bitmap = &charset[candidate.glyph as usize];
            for y in 0..GLYPH_HEIGHT as usize {
                for x in 0..GLYPH_WIDTH as usize {
                    colors[cell_y * 8 + y][cell_x * 8 + x] = if bitmap[y][x] >= 128 {
                        candidate.fg
                    } else {
                        candidate.bg
                    };
                }
            }
        }
    }

    let mut thin = vec![vec![false; PATCH_SIZE]; PATCH_SIZE];
    for y in 0..PATCH_SIZE {
        for x in 0..PATCH_SIZE {
            let mut same_neighbors = 0usize;
            for dy in -1isize..=1 {
                for dx in -1isize..=1 {
                    if dx == 0 && dy == 0 {
                        continue;
                    }
                    let nx = x as isize + dx;
                    let ny = y as isize + dy;
                    if nx >= 0
                        && ny >= 0
                        && nx < PATCH_SIZE as isize
                        && ny < PATCH_SIZE as isize
                        && colors[ny as usize][nx as usize] == colors[y][x]
                    {
                        same_neighbors += 1;
                    }
                }
            }
            thin[y][x] = same_neighbors <= 2;
        }
    }

    let mut visited = vec![vec![false; PATCH_SIZE]; PATCH_SIZE];
    let mut penalty = 0.0f64;
    for start_y in 0..PATCH_SIZE {
        for start_x in 0..PATCH_SIZE {
            if !thin[start_y][start_x] || visited[start_y][start_x] {
                continue;
            }
            let color = colors[start_y][start_x];
            let mut stack = vec![(start_x, start_y)];
            let mut size = 0usize;
            let mut intersects_center = false;
            let mut touches_patch_edge = false;
            visited[start_y][start_x] = true;
            while let Some((x, y)) = stack.pop() {
                size += 1;
                intersects_center |= (8..16).contains(&x) && (8..16).contains(&y);
                touches_patch_edge |=
                    x == 0 || y == 0 || x + 1 == PATCH_SIZE || y + 1 == PATCH_SIZE;
                for dy in -1isize..=1 {
                    for dx in -1isize..=1 {
                        let nx = x as isize + dx;
                        let ny = y as isize + dy;
                        if nx < 0
                            || ny < 0
                            || nx >= PATCH_SIZE as isize
                            || ny >= PATCH_SIZE as isize
                        {
                            continue;
                        }
                        let nx = nx as usize;
                        let ny = ny as usize;
                        if !visited[ny][nx] && thin[ny][nx] && colors[ny][nx] == color {
                            visited[ny][nx] = true;
                            stack.push((nx, ny));
                        }
                    }
                }
            }
            if intersects_center && !touches_patch_edge {
                penalty = penalty.max((size.min(8) as f64) / 8.0);
            }
        }
    }
    (penalty, equals_noise_penalty(&colors))
}

#[derive(Debug, Clone, Copy)]
struct ThinRun {
    position: usize,
    start: usize,
    end: usize,
    color: u8,
}

fn equals_noise_penalty(colors: &[Vec<u8>]) -> f64 {
    let mut counts = [0usize; 256];
    for color in colors.iter().flatten() {
        counts[*color as usize] += 1;
    }
    let dominant = counts
        .iter()
        .enumerate()
        .max_by_key(|(_, count)| *count)
        .map(|(color, _)| color as u8)
        .unwrap_or(0);
    let horizontal = collect_thin_runs(colors, false);
    let vertical = collect_thin_runs(colors, true);
    parallel_run_pair_penalty(&horizontal, colors, false, dominant)
        .max(parallel_run_pair_penalty(&vertical, colors, true, dominant))
}

fn collect_thin_runs(colors: &[Vec<u8>], vertical: bool) -> Vec<ThinRun> {
    let height = colors.len();
    let width = colors.first().map_or(0, Vec::len);
    let positions = if vertical { width } else { height };
    let along_size = if vertical { height } else { width };
    let mut runs = Vec::new();
    for position in 0..positions {
        let mut start = 0usize;
        while start < along_size {
            let (x, y) = if vertical {
                (position, start)
            } else {
                (start, position)
            };
            let color = colors[y][x];
            if !pixel_is_orthogonally_thin(colors, x, y, vertical) {
                start += 1;
                continue;
            }
            let mut end = start + 1;
            while end < along_size {
                let (x, y) = if vertical {
                    (position, end)
                } else {
                    (end, position)
                };
                if colors[y][x] != color || !pixel_is_orthogonally_thin(colors, x, y, vertical) {
                    break;
                }
                end += 1;
            }
            if end - start >= 6 {
                runs.push(ThinRun {
                    position,
                    start,
                    end,
                    color,
                });
            }
            start = end;
        }
    }
    runs
}

fn pixel_is_orthogonally_thin(colors: &[Vec<u8>], x: usize, y: usize, vertical: bool) -> bool {
    let height = colors.len();
    let width = colors.first().map_or(0, Vec::len);
    let color = colors[y][x];
    let neighbors = if vertical {
        [
            x.checked_sub(1).map(|nx| (nx, y)),
            (x + 1 < width).then_some((x + 1, y)),
        ]
    } else {
        [
            y.checked_sub(1).map(|ny| (x, ny)),
            (y + 1 < height).then_some((x, y + 1)),
        ]
    };
    neighbors
        .into_iter()
        .flatten()
        .filter(|(nx, ny)| colors[*ny][*nx] == color)
        .count()
        <= 1
}

fn parallel_run_pair_penalty(
    runs: &[ThinRun],
    colors: &[Vec<u8>],
    vertical: bool,
    dominant: u8,
) -> f64 {
    const CENTER_START: usize = 8;
    const CENTER_END: usize = 16;
    let mut penalty = 0.0f64;
    for (index, first) in runs.iter().enumerate() {
        for second in &runs[index + 1..] {
            if (palette_distance(first.color, dominant) <= 0.22
                && palette_distance(second.color, dominant) <= 0.22)
                || palette_distance(first.color, second.color) > 0.28
            {
                continue;
            }
            let separation = second.position.saturating_sub(first.position);
            if !(3..=10).contains(&separation) {
                continue;
            }
            let overlap_start = first.start.max(second.start);
            let overlap_end = first.end.min(second.end);
            if overlap_end.saturating_sub(overlap_start) < 6 {
                continue;
            }
            let intersects_center = ((CENTER_START..CENTER_END).contains(&first.position)
                || (CENTER_START..CENTER_END).contains(&second.position))
                && overlap_start < CENTER_END
                && overlap_end > CENTER_START;
            if !intersects_center {
                continue;
            }

            let mut gap_pixels = 0usize;
            let mut different_pixels = 0usize;
            for position in first.position + 1..second.position {
                for along in overlap_start..overlap_end {
                    let (x, y) = if vertical {
                        (position, along)
                    } else {
                        (along, position)
                    };
                    gap_pixels += 1;
                    different_pixels += usize::from(
                        palette_distance(colors[y][x], first.color) > 0.18
                            && palette_distance(colors[y][x], second.color) > 0.18,
                    );
                }
            }
            if gap_pixels == 0 || different_pixels * 10 < gap_pixels * 7 {
                continue;
            }
            let overlap = overlap_end - overlap_start;
            penalty = penalty.max((overlap.min(12) as f64) / 12.0);
        }
    }
    penalty
}

fn global_equals_targets(
    colors: &[Vec<u8>],
    cell_width: u32,
    cell_height: u32,
) -> (usize, Vec<usize>) {
    let mut counts = [0usize; 256];
    for color in colors.iter().flatten() {
        counts[*color as usize] += 1;
    }
    let dominant = counts
        .iter()
        .enumerate()
        .max_by_key(|(_, count)| *count)
        .map(|(color, _)| color as u8)
        .unwrap_or(0);
    let mut targets = BTreeSet::new();
    let horizontal = collect_thin_runs(colors, false);
    let vertical = collect_thin_runs(colors, true);
    let count = collect_global_parallel_pairs(
        &horizontal,
        colors,
        false,
        dominant,
        cell_width,
        cell_height,
        &mut targets,
    ) + collect_global_parallel_pairs(
        &vertical,
        colors,
        true,
        dominant,
        cell_width,
        cell_height,
        &mut targets,
    );
    (count, targets.into_iter().collect())
}

fn collect_global_parallel_pairs(
    runs: &[ThinRun],
    colors: &[Vec<u8>],
    vertical: bool,
    dominant: u8,
    cell_width: u32,
    cell_height: u32,
    targets: &mut BTreeSet<usize>,
) -> usize {
    let mut pair_count = 0usize;
    for (index, first) in runs.iter().enumerate() {
        for second in &runs[index + 1..] {
            let separation = second.position.saturating_sub(first.position);
            if separation > 10 {
                break;
            }
            if separation < 3
                || (palette_distance(first.color, dominant) <= 0.22
                    && palette_distance(second.color, dominant) <= 0.22)
                || palette_distance(first.color, second.color) > 0.28
            {
                continue;
            }
            let overlap_start = first.start.max(second.start);
            let overlap_end = first.end.min(second.end);
            if overlap_end.saturating_sub(overlap_start) < 6 {
                continue;
            }
            let mut gap_pixels = 0usize;
            let mut different_pixels = 0usize;
            for position in first.position + 1..second.position {
                for along in overlap_start..overlap_end {
                    let (x, y) = if vertical {
                        (position, along)
                    } else {
                        (along, position)
                    };
                    gap_pixels += 1;
                    different_pixels += usize::from(
                        palette_distance(colors[y][x], first.color) > 0.18
                            && palette_distance(colors[y][x], second.color) > 0.18,
                    );
                }
            }
            if gap_pixels == 0 || different_pixels * 10 < gap_pixels * 7 {
                continue;
            }
            // Use overlap length rather than a binary pair count. A candidate
            // that shortens a long visual "=" must receive credit even when
            // the remaining fragments are still long enough to be detected.
            pair_count += overlap_end - overlap_start;
            if vertical {
                let first_x = first.position / 8;
                let second_x = second.position / 8;
                for cell_y in overlap_start / 8..=(overlap_end - 1) / 8 {
                    if first_x < cell_width as usize && cell_y < cell_height as usize {
                        targets.insert(cell_y * cell_width as usize + first_x);
                    }
                    if second_x < cell_width as usize && cell_y < cell_height as usize {
                        targets.insert(cell_y * cell_width as usize + second_x);
                    }
                }
            } else {
                let first_y = first.position / 8;
                let second_y = second.position / 8;
                for cell_x in overlap_start / 8..=(overlap_end - 1) / 8 {
                    if cell_x < cell_width as usize && first_y < cell_height as usize {
                        targets.insert(first_y * cell_width as usize + cell_x);
                    }
                    if cell_x < cell_width as usize && second_y < cell_height as usize {
                        targets.insert(second_y * cell_width as usize + cell_x);
                    }
                }
            }
        }
    }
    pair_count
}

fn palette_distance(first: u8, second: u8) -> f64 {
    ANSI_COLOR_RGB[first as usize]
        .iter()
        .zip(ANSI_COLOR_RGB[second as usize].iter())
        .map(|(a, b)| (*a as f64 - *b as f64).abs())
        .sum::<f64>()
        / (255.0 * 3.0)
}

/// Penalize a tiny foreground stroke or background notch that enters from at
/// most one cell side. Shapes crossing the cell or turning through a corner
/// touch at least two sides and remain available for genuine thin contours.
fn bitmap_spur_penalty(bitmap: &BlockGrayImage) -> f64 {
    let foreground = bitmap
        .iter()
        .flatten()
        .filter(|pixel| **pixel >= 128)
        .count();
    let total = (GLYPH_WIDTH * GLYPH_HEIGHT) as usize;
    let minority_is_foreground = foreground <= total / 2;
    let minority_count = if minority_is_foreground {
        foreground
    } else {
        total - foreground
    };
    let minority_penalty = if minority_count == 0 || minority_count > 16 {
        0.0
    } else {
        let is_minority = |x: usize, y: usize| {
            let foreground_pixel = bitmap[y][x] >= 128;
            foreground_pixel == minority_is_foreground
        };
        let last_x = GLYPH_WIDTH as usize - 1;
        let last_y = GLYPH_HEIGHT as usize - 1;
        let touched_sides = [
            (0..=last_x).any(|x| is_minority(x, 0)),
            (0..=last_y).any(|y| is_minority(last_x, y)),
            (0..=last_x).any(|x| is_minority(x, last_y)),
            (0..=last_y).any(|y| is_minority(0, y)),
        ]
        .into_iter()
        .filter(|touched| *touched)
        .count();
        if touched_sides <= 1 {
            1.0 - (minority_count.saturating_sub(1) as f64 / 32.0)
        } else {
            0.0
        }
    };

    minority_penalty.max(thin_branch_penalty(bitmap))
}

fn bitmap_fragmentation_penalty(bitmap: &BlockGrayImage) -> f64 {
    let foreground = bitmap
        .iter()
        .flatten()
        .filter(|pixel| **pixel >= 128)
        .count();
    let minority_is_foreground = foreground <= (GLYPH_WIDTH * GLYPH_HEIGHT) as usize / 2;
    let mut visited = vec![vec![false; GLYPH_WIDTH as usize]; GLYPH_HEIGHT as usize];
    let mut components = 0usize;
    for start_y in 0..GLYPH_HEIGHT as usize {
        for start_x in 0..GLYPH_WIDTH as usize {
            let is_minority = (bitmap[start_y][start_x] >= 128) == minority_is_foreground;
            if !is_minority || visited[start_y][start_x] {
                continue;
            }
            components += 1;
            let mut stack = vec![(start_x, start_y)];
            visited[start_y][start_x] = true;
            while let Some((x, y)) = stack.pop() {
                for (dx, dy) in [(0i32, -1i32), (1, 0), (0, 1), (-1, 0)] {
                    let nx = x as i32 + dx;
                    let ny = y as i32 + dy;
                    if nx < 0 || ny < 0 || nx >= GLYPH_WIDTH as i32 || ny >= GLYPH_HEIGHT as i32 {
                        continue;
                    }
                    let nx = nx as usize;
                    let ny = ny as usize;
                    let is_minority = (bitmap[ny][nx] >= 128) == minority_is_foreground;
                    if is_minority && !visited[ny][nx] {
                        visited[ny][nx] = true;
                        stack.push((nx, ny));
                    }
                }
            }
        }
    }

    let mut transitions = 0usize;
    for y in 0..GLYPH_HEIGHT as usize {
        for x in 0..GLYPH_WIDTH as usize {
            if x + 1 < GLYPH_WIDTH as usize && bitmap[y][x] != bitmap[y][x + 1] {
                transitions += 1;
            }
            if y + 1 < GLYPH_HEIGHT as usize && bitmap[y][x] != bitmap[y + 1][x] {
                transitions += 1;
            }
        }
    }
    let component_loss = components.saturating_sub(1).min(3) as f64 / 3.0;
    let transition_loss = transitions.saturating_sub(20).min(40) as f64 / 40.0;
    component_loss.max(transition_loss)
}

fn thin_branch_penalty(bitmap: &BlockGrayImage) -> f64 {
    let last_x = GLYPH_WIDTH as usize - 1;
    let last_y = GLYPH_HEIGHT as usize - 1;
    let mut thin = vec![vec![false; GLYPH_WIDTH as usize]; GLYPH_HEIGHT as usize];
    for y in 0..=last_y {
        for x in 0..=last_x {
            let value = bitmap[y][x] >= 128;
            let mut same_neighbors = 0usize;
            for dy in -1isize..=1 {
                for dx in -1isize..=1 {
                    if dx == 0 && dy == 0 {
                        continue;
                    }
                    let nx = x as isize + dx;
                    let ny = y as isize + dy;
                    if nx >= 0
                        && ny >= 0
                        && nx <= last_x as isize
                        && ny <= last_y as isize
                        && (bitmap[ny as usize][nx as usize] >= 128) == value
                    {
                        same_neighbors += 1;
                    }
                }
            }
            thin[y][x] = same_neighbors <= 2;
        }
    }

    let mut visited = vec![vec![false; GLYPH_WIDTH as usize]; GLYPH_HEIGHT as usize];
    let mut penalty = 0.0f64;
    for start_y in 0..=last_y {
        for start_x in 0..=last_x {
            if !thin[start_y][start_x] || visited[start_y][start_x] {
                continue;
            }
            let value = bitmap[start_y][start_x] >= 128;
            let mut stack = vec![(start_x, start_y)];
            let mut size = 0usize;
            let mut touched = [false; 4];
            visited[start_y][start_x] = true;
            while let Some((x, y)) = stack.pop() {
                size += 1;
                touched[0] |= y == 0;
                touched[1] |= x == last_x;
                touched[2] |= y == last_y;
                touched[3] |= x == 0;
                for dy in -1isize..=1 {
                    for dx in -1isize..=1 {
                        let nx = x as isize + dx;
                        let ny = y as isize + dy;
                        if nx < 0 || ny < 0 || nx > last_x as isize || ny > last_y as isize {
                            continue;
                        }
                        let nx = nx as usize;
                        let ny = ny as usize;
                        if !visited[ny][nx] && thin[ny][nx] && (bitmap[ny][nx] >= 128) == value {
                            visited[ny][nx] = true;
                            stack.push((nx, ny));
                        }
                    }
                }
            }
            let touched_sides = touched.into_iter().filter(|side| *side).count();
            if touched_sides <= 1 {
                penalty = penalty.max((size.min(8) as f64) / 8.0);
            }
        }
    }
    penalty
}

fn border_mismatch(
    first: GlyphCandidate,
    first_border: Border,
    second: GlyphCandidate,
    second_border: Border,
    charset: &[BlockGrayImage],
) -> f64 {
    let mut mismatch = 0.0;
    for offset in 0..GLYPH_WIDTH as usize {
        let first_rgb = rendered_border_rgb(first, first_border, offset, charset);
        let second_rgb = rendered_border_rgb(second, second_border, offset, charset);
        mismatch += first_rgb
            .iter()
            .zip(second_rgb.iter())
            .map(|(a, b)| (*a as f64 - *b as f64).abs())
            .sum::<f64>()
            / (255.0 * 3.0);
    }
    mismatch / GLYPH_WIDTH as f64
}

fn rendered_border_rgb(
    candidate: GlyphCandidate,
    border: Border,
    offset: usize,
    charset: &[BlockGrayImage],
) -> [u8; 3] {
    let (x, y) = match border {
        Border::Top => (offset, 0),
        Border::Right => (GLYPH_WIDTH as usize - 1, offset),
        Border::Bottom => (offset, GLYPH_HEIGHT as usize - 1),
        Border::Left => (0, offset),
    };
    let color = if charset[candidate.glyph as usize][y][x] >= 128 {
        candidate.fg
    } else {
        candidate.bg
    };
    ANSI_COLOR_RGB[color as usize]
}

fn palette_luma(index: u8) -> f32 {
    let color = ANSI_COLOR_RGB[index as usize];
    0.299 * color[0] as f32 + 0.587 * color[1] as f32 + 0.114 * color[2] as f32
}

fn sobel_block(block: &BlockGrayImage) -> BlockGrayImage {
    let mut edges = vec![vec![0u8; GLYPH_WIDTH as usize]; GLYPH_HEIGHT as usize];
    for y in 1..GLYPH_HEIGHT as usize - 1 {
        for x in 1..GLYPH_WIDTH as usize - 1 {
            let sample = |dx: isize, dy: isize| {
                block[(y as isize + dy) as usize][(x as isize + dx) as usize] as i32
            };
            let gx = -sample(-1, -1) + sample(1, -1) - 2 * sample(-1, 0) + 2 * sample(1, 0)
                - sample(-1, 1)
                + sample(1, 1);
            let gy = -sample(-1, -1) - 2 * sample(0, -1) - sample(1, -1)
                + sample(-1, 1)
                + 2 * sample(0, 1)
                + sample(1, 1);
            edges[y][x] = (gx.abs() + gy.abs()).min(255) as u8;
        }
    }
    edges
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{render_grid, PetsciiCell, PetsciiGrid};
    use image::{ImageBuffer, Rgba};

    #[test]
    fn top_k_is_sorted_and_deterministic() {
        let image = DynamicImage::ImageRgba8(ImageBuffer::from_pixel(8, 8, Rgba([0, 0, 0, 255])));
        let config = ConversionConfig {
            width: 1,
            height: 1,
            mode: 1,
            top_k: 4,
            contrast: 0.0,
        };
        let first = convert_image(&image, &config).unwrap();
        let second = convert_image(&image, &config).unwrap();
        assert_eq!(first.grid, second.grid);
        assert_eq!(first.alternatives, second.alternatives);
        assert_eq!(first.alternatives[0].len(), 4);
        assert!(first.alternatives[0]
            .windows(2)
            .all(|w| w[0].distance <= w[1].distance));
    }

    #[test]
    fn generated_grid_is_valid_pix() {
        let image =
            DynamicImage::ImageRgba8(ImageBuffer::from_pixel(16, 8, Rgba([20, 40, 80, 255])));
        let config = ConversionConfig {
            width: 2,
            height: 1,
            mode: 1,
            top_k: 2,
            contrast: 0.0,
        };
        let result = convert_image(&image, &config).unwrap();
        let pix = result.grid.to_pix_string();
        assert!(pix.starts_with("width=2,height=1,texture=255\n"));
        assert_eq!(result.grid.cells.len(), 2);
    }

    #[test]
    fn exact_mode_preserves_a_known_glyph() {
        let source_grid = PetsciiGrid::new(
            1,
            1,
            vec![PetsciiCell {
                glyph: 65,
                fg: 15,
                bg: 0,
                texture: 1,
            }],
        )
        .unwrap();
        let source = DynamicImage::ImageRgba8(render_grid(&source_grid, 1).unwrap());
        let config = ConversionConfig {
            width: 1,
            height: 1,
            mode: 1,
            top_k: 1,
            contrast: 0.0,
        };
        let result = convert_image(&source, &config).unwrap();
        assert_eq!(result.grid.cells[0].glyph, 65);
    }

    #[test]
    fn mode_two_candidates_never_include_letters_or_digits() {
        let image =
            DynamicImage::ImageRgba8(ImageBuffer::from_pixel(8, 8, Rgba([10, 10, 10, 255])));
        let config = ConversionConfig {
            width: 1,
            height: 1,
            mode: 2,
            top_k: 16,
            contrast: 0.0,
        };
        let result = convert_image(&image, &config).unwrap();
        assert!(result
            .alternatives
            .iter()
            .flatten()
            .all(|candidate| glyph_allowed(2, candidate.glyph)));
    }

    #[test]
    fn flat_background_uses_space_and_detected_background_color() {
        let image =
            DynamicImage::ImageRgba8(ImageBuffer::from_pixel(16, 8, Rgba([64, 91, 137, 255])));
        let config = ConversionConfig {
            width: 2,
            height: 1,
            mode: 2,
            top_k: 6,
            contrast: 0.0,
        };
        let result = convert_image(&image, &config).unwrap();
        assert!(result
            .grid
            .cells
            .iter()
            .all(|cell| cell.glyph == SPACE_GLYPH && cell.bg != 0));
    }

    #[test]
    fn flat_non_background_region_uses_solid_glyph() {
        let mut image = ImageBuffer::from_pixel(24, 8, Rgba([64, 91, 137, 255]));
        for y in 0..8 {
            for x in 16..24 {
                image.put_pixel(x, y, Rgba([235, 235, 225, 255]));
            }
        }
        let config = ConversionConfig {
            width: 3,
            height: 1,
            mode: 2,
            top_k: 6,
            contrast: 0.0,
        };
        let result = convert_image(&DynamicImage::ImageRgba8(image), &config).unwrap();
        assert_eq!(result.grid.cells[0].glyph, SPACE_GLYPH);
        assert_eq!(result.grid.cells[1].glyph, SPACE_GLYPH);
        assert_eq!(result.grid.cells[2].glyph, SOLID_GLYPH);
    }

    #[test]
    fn edge_target_prefers_matching_fill_side() {
        let mut input = vec![vec![0u8; 8]; 8];
        for row in &mut input {
            row[4..].fill(255);
        }
        let target = EdgeTarget::new(&input, 15, 0);
        let matching = input.clone();
        let inverted: BlockGrayImage = input
            .iter()
            .map(|row| row.iter().map(|pixel| 255 - pixel).collect())
            .collect();
        assert_eq!(target.distance(&matching), 0.0);
        assert!(target.distance(&matching) < target.distance(&inverted));
    }

    #[test]
    fn sobel_detects_edge_crossing_character_cells() {
        let mut image = GrayImage::new(16, 8);
        for y in 0..8 {
            for x in 8..16 {
                image.put_pixel(x, y, Luma([255]));
            }
        }
        let edges = sobel_image(&image);
        let left = get_grayscale_block_at(&edges, 0, 0, 8, 8);
        let right = get_grayscale_block_at(&edges, 1, 0, 8, 8);
        assert!(is_edge_cell(&left));
        assert!(is_edge_cell(&right));
    }

    #[test]
    fn edge_cleanup_removes_tiny_components_but_keeps_connected_contours() {
        let mut edges = GrayImage::new(16, 8);
        edges.put_pixel(1, 1, Luma([255]));
        edges.put_pixel(2, 1, Luma([64]));
        for x in 5..13 {
            edges.put_pixel(x, 4, Luma([if x == 8 { 255 } else { 64 }]));
        }
        let cleaned = clean_edge_image(&edges);
        assert_eq!(cleaned.get_pixel(1, 1).0[0], 0);
        assert_eq!(cleaned.get_pixel(2, 1).0[0], 0);
        assert_eq!(cleaned.get_pixel(5, 4).0[0], 64);
        assert_eq!(cleaned.get_pixel(8, 4).0[0], 255);
    }

    #[test]
    fn continuity_selection_rejects_a_dangling_cell_border() {
        let charset = gen_charset_images(false, 8, 8, &C64LOW, &C64UP);
        let continuous = GlyphCandidate {
            glyph: SPACE_GLYPH,
            distance: 0.02,
            fg: 6,
            bg: 6,
            texture: 1,
        };
        let dangling = GlyphCandidate {
            glyph: SOLID_GLYPH,
            distance: 0.0,
            fg: 0,
            bg: 6,
            texture: 1,
        };
        let blue_space = solid_candidate(SPACE_GLYPH, 6, 6);
        let alternatives = vec![
            vec![blue_space],
            vec![dangling, continuous],
            vec![blue_space],
        ];
        let selected =
            select_continuous_edges(3, 1, &alternatives, &[false, true, false], &charset);
        assert_eq!(selected[1], 1);
    }

    #[test]
    fn spur_penalty_distinguishes_dangling_and_crossing_lines() {
        let mut dangling = vec![vec![0u8; 8]; 8];
        for row in dangling.iter_mut().take(5) {
            row[3] = 255;
        }
        let mut crossing = vec![vec![0u8; 8]; 8];
        for row in &mut crossing {
            row[3] = 255;
        }
        assert!(bitmap_spur_penalty(&dangling) > 0.0);
        assert_eq!(bitmap_spur_penalty(&crossing), 0.0);
    }

    #[test]
    fn equals_noise_detects_horizontal_and_vertical_parallel_pairs() {
        let mut horizontal = vec![vec![0u8; 24]; 24];
        horizontal[9][6..18].fill(3);
        horizontal[14][6..18].fill(3);
        assert!(equals_noise_penalty(&horizontal) > 0.0);
        let (count, targets) = global_equals_targets(&horizontal, 3, 3);
        assert!(count > 0);
        assert!(!targets.is_empty());

        let mut vertical = vec![vec![0u8; 24]; 24];
        for row in vertical.iter_mut().take(18).skip(6) {
            row[9] = 3;
            row[14] = 3;
        }
        assert!(equals_noise_penalty(&vertical) > 0.0);
    }

    #[test]
    fn equals_noise_does_not_mistake_a_filled_band_for_parallel_lines() {
        let mut filled = vec![vec![0u8; 24]; 24];
        for row in filled.iter_mut().take(15).skip(9) {
            row[6..18].fill(3);
        }
        assert_eq!(equals_noise_penalty(&filled), 0.0);
        assert_eq!(global_equals_targets(&filled, 3, 3).0, 0);
    }

    #[test]
    fn fragmented_checker_is_costlier_than_a_continuous_rule() {
        let charset = gen_charset_images(false, 8, 8, &C64LOW, &C64UP);
        assert!(bitmap_fragmentation_penalty(&charset[104]) > 0.0);
        assert_eq!(bitmap_fragmentation_penalty(&charset[68]), 0.0);
    }

    #[test]
    fn mode_two_edge_cell_uses_graphic_partial_fill() {
        let mut image = ImageBuffer::from_pixel(32, 8, Rgba([64, 91, 137, 255]));
        for y in 0..8 {
            for x in 20..32 {
                image.put_pixel(x, y, Rgba([235, 235, 225, 255]));
            }
        }
        let config = ConversionConfig {
            width: 4,
            height: 1,
            mode: 2,
            top_k: 6,
            contrast: 0.0,
        };
        let result = convert_image(&DynamicImage::ImageRgba8(image), &config).unwrap();
        let edge_cell = result.grid.cells[2];
        assert_ne!(edge_cell.glyph, SPACE_GLYPH);
        assert_ne!(edge_cell.glyph, SOLID_GLYPH);
        assert!(glyph_allowed(2, edge_cell.glyph));
        assert_ne!(edge_cell.fg, edge_cell.bg);
    }
}
