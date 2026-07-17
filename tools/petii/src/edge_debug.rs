use crate::glyph_topology::Side;
use crate::EdgeDebugData;
use image::{Rgba, RgbaImage};

const GLYPH_SIZE: u32 = 8;
const BACKGROUND: Rgba<u8> = Rgba([18, 18, 22, 255]);
const GRID: Rgba<u8> = Rgba([58, 58, 66, 255]);
const TARGET: Rgba<u8> = Rgba([30, 210, 230, 255]);
const CONNECTED: Rgba<u8> = Rgba([45, 220, 105, 255]);
const BROKEN: Rgba<u8> = Rgba([245, 55, 65, 255]);
const EDITED: Rgba<u8> = Rgba([255, 155, 35, 255]);
const SPUR: Rgba<u8> = Rgba([245, 220, 45, 255]);
const JUNCTION: Rgba<u8> = Rgba([190, 70, 235, 255]);

pub fn render_edge_debug(data: &EdgeDebugData, scale: u32) -> Result<RgbaImage, String> {
    if !(1..=8).contains(&scale) {
        return Err("edge debug scale must be between 1 and 8".to_string());
    }
    let cell_size = GLYPH_SIZE * scale;
    let mut image =
        RgbaImage::from_pixel(data.width * cell_size, data.height * cell_size, BACKGROUND);

    for index in 0..data.target_topologies.len() {
        let x = index as u32 % data.width;
        let y = index as u32 / data.width;
        let left = x * cell_size;
        let top = y * cell_size;
        if data.target_topologies[index].is_some() {
            draw_rect(&mut image, left, top, cell_size, cell_size, GRID);
        }
        if data.edited_cells[index] {
            draw_rect_inset(&mut image, left, top, cell_size, EDITED, 1);
        }
        if data.spur_cells[index] {
            draw_rect_inset(&mut image, left, top, cell_size, SPUR, 2);
        }
        if data.junctions.binary_search(&index).is_ok() {
            draw_rect_inset(&mut image, left, top, cell_size, JUNCTION, 3);
        }
        let Some(target) = data.target_topologies[index] else {
            continue;
        };
        let selected = data.final_topologies[index];
        for side in Side::ALL {
            draw_ports(
                &mut image,
                left,
                top,
                cell_size,
                scale,
                side,
                target.edge_ports(side),
                TARGET,
                0,
            );
            let selected_ports = selected.edge_ports(side);
            let aligned = dilate_ports(target.edge_ports(side));
            for bit in 0..7 {
                if selected_ports & (1 << bit) == 0 {
                    continue;
                }
                let color = if aligned & (1 << bit) != 0 {
                    CONNECTED
                } else {
                    BROKEN
                };
                draw_single_port(&mut image, left, top, cell_size, scale, side, bit, color, 2);
            }
        }
    }

    for &(first, second, side) in &data.connections {
        let mismatch = data.final_topologies[first]
            .shared_port_mismatch_tolerant(side, data.final_topologies[second]);
        let color = if mismatch == 0.0 { CONNECTED } else { BROKEN };
        draw_connection(&mut image, data.width, first, side, cell_size, color);
    }
    Ok(image)
}

fn draw_ports(
    image: &mut RgbaImage,
    left: u32,
    top: u32,
    cell_size: u32,
    scale: u32,
    side: Side,
    ports: u8,
    color: Rgba<u8>,
    inset: u32,
) {
    for bit in 0..7 {
        if ports & (1 << bit) != 0 {
            draw_single_port(image, left, top, cell_size, scale, side, bit, color, inset);
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn draw_single_port(
    image: &mut RgbaImage,
    left: u32,
    top: u32,
    cell_size: u32,
    scale: u32,
    side: Side,
    bit: u32,
    color: Rgba<u8>,
    inset: u32,
) {
    let position = (bit + 1) * scale;
    let inset = inset.min(cell_size.saturating_sub(1));
    let (x, y) = match side {
        Side::Top => (left + position, top + inset),
        Side::Right => (left + cell_size - 1 - inset, top + position),
        Side::Bottom => (left + position, top + cell_size - 1 - inset),
        Side::Left => (left + inset, top + position),
    };
    put_square(image, x, y, scale.max(1), color);
}

fn draw_connection(
    image: &mut RgbaImage,
    width: u32,
    first: usize,
    side: Side,
    cell_size: u32,
    color: Rgba<u8>,
) {
    let x = first as u32 % width;
    let y = first as u32 / width;
    match side {
        Side::Right => {
            let boundary = (x + 1) * cell_size - 1;
            for py in y * cell_size..(y + 1) * cell_size {
                image.put_pixel(boundary, py, color);
            }
        }
        Side::Bottom => {
            let boundary = (y + 1) * cell_size - 1;
            for px in x * cell_size..(x + 1) * cell_size {
                image.put_pixel(px, boundary, color);
            }
        }
        Side::Top | Side::Left => {}
    }
}

fn draw_rect(image: &mut RgbaImage, left: u32, top: u32, width: u32, height: u32, color: Rgba<u8>) {
    for x in left..left + width {
        image.put_pixel(x, top, color);
        image.put_pixel(x, top + height - 1, color);
    }
    for y in top..top + height {
        image.put_pixel(left, y, color);
        image.put_pixel(left + width - 1, y, color);
    }
}

fn draw_rect_inset(
    image: &mut RgbaImage,
    left: u32,
    top: u32,
    size: u32,
    color: Rgba<u8>,
    inset: u32,
) {
    if size > inset * 2 {
        draw_rect(
            image,
            left + inset,
            top + inset,
            size - inset * 2,
            size - inset * 2,
            color,
        );
    }
}

fn put_square(image: &mut RgbaImage, center_x: u32, center_y: u32, size: u32, color: Rgba<u8>) {
    let radius = size / 2;
    for y in center_y.saturating_sub(radius)..=(center_y + radius).min(image.height() - 1) {
        for x in center_x.saturating_sub(radius)..=(center_x + radius).min(image.width() - 1) {
            image.put_pixel(x, y, color);
        }
    }
}

fn dilate_ports(ports: u8) -> u8 {
    (ports | (ports << 1) | (ports >> 1)) & 0x7f
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::glyph_topology::GlyphTopology;
    use rust_pixel::render::symbols::BlockGrayImage;

    fn horizontal_fill(start_y: usize) -> GlyphTopology {
        let mut bitmap: BlockGrayImage = vec![vec![0u8; 8]; 8];
        for row in bitmap.iter_mut().skip(start_y) {
            row.fill(255);
        }
        GlyphTopology::from_bitmap(&bitmap)
    }

    #[test]
    fn debug_overlay_is_exact_and_deterministic() {
        let aligned = horizontal_fill(4);
        let shifted = horizontal_fill(6);
        let data = EdgeDebugData {
            width: 2,
            height: 1,
            target_topologies: vec![Some(aligned), Some(aligned)],
            baseline_topologies: vec![shifted, aligned],
            final_topologies: vec![shifted, aligned],
            edited_cells: vec![true, false],
            spur_cells: vec![false, true],
            connections: vec![(0, 1, Side::Right)],
            junctions: vec![],
        };
        let first = render_edge_debug(&data, 2).unwrap();
        let second = render_edge_debug(&data, 2).unwrap();
        assert_eq!(first.dimensions(), (32, 16));
        assert_eq!(first, second);
        assert!(first.pixels().any(|pixel| *pixel == EDITED));
        assert!(first.pixels().any(|pixel| *pixel == SPUR));
        assert!(first.pixels().any(|pixel| *pixel == BROKEN));
    }
}
