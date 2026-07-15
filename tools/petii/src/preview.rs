use crate::c64::C64UP;
use crate::types::{PetsciiCell, PetsciiGrid};
use image::{Rgba, RgbaImage};
use rust_pixel::render::style::ANSI_COLOR_RGB;

pub const GLYPH_SIZE: u32 = 8;

/// Render the exact fixed PETSCII grid on the CPU. No terminal or GPU is required.
pub fn render_grid(grid: &PetsciiGrid, scale: u32) -> Result<RgbaImage, String> {
    if scale == 0 || scale > 32 {
        return Err("preview scale must be between 1 and 32".to_string());
    }
    let mut image = RgbaImage::new(
        grid.width * GLYPH_SIZE * scale,
        grid.height * GLYPH_SIZE * scale,
    );
    for y in 0..grid.height {
        for x in 0..grid.width {
            draw_cell(&mut image, x, y, grid.get(x, y), scale);
        }
    }
    Ok(image)
}

fn draw_cell(image: &mut RgbaImage, cell_x: u32, cell_y: u32, cell: PetsciiCell, scale: u32) {
    let glyph_index = (cell.glyph % 128) as usize;
    let inverted = cell.glyph >= 128;
    let fg = ANSI_COLOR_RGB[cell.fg as usize];
    let bg = ANSI_COLOR_RGB[cell.bg as usize];
    for py in 0..GLYPH_SIZE {
        let bits = C64UP[glyph_index][py as usize];
        for px in 0..GLYPH_SIZE {
            let set = ((bits >> (7 - px)) & 1 == 1) ^ inverted;
            let color = if set { fg } else { bg };
            for sy in 0..scale {
                for sx in 0..scale {
                    image.put_pixel(
                        (cell_x * GLYPH_SIZE + px) * scale + sx,
                        (cell_y * GLYPH_SIZE + py) * scale + sy,
                        Rgba([color[0], color[1], color[2], 255]),
                    );
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn preview_has_exact_grid_dimensions() {
        let grid = PetsciiGrid::new(
            2,
            3,
            vec![
                PetsciiCell {
                    glyph: 0,
                    fg: 1,
                    bg: 0,
                    texture: 1
                };
                6
            ],
        )
        .unwrap();
        let image = render_grid(&grid, 2).unwrap();
        assert_eq!(image.dimensions(), (32, 48));
    }
}
