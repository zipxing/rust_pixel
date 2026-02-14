use super::braille::BrailleCanvas;
use super::{ChartData, ChartRenderer, CHART_COLORS, LABEL_COLOR, TITLE_COLOR};
use rust_pixel::render::buffer::Buffer;
use rust_pixel::render::style::Style;
use std::f64::consts::TAU;

pub struct PieChart {
    pub data: ChartData,
}

impl PieChart {
    pub fn new(data: ChartData) -> Self {
        Self { data }
    }
}

impl ChartRenderer for PieChart {
    fn render(&self, buf: &mut Buffer, x: u16, y: u16, w: u16, h: u16) {
        let values = match self.data.series.first() {
            Some(v) if !v.is_empty() => v,
            _ => return,
        };
        let labels = &self.data.labels;
        let n = values.len();
        if n == 0 || h < 4 {
            return;
        }

        let total: f64 = values.iter().sum();
        if total <= 0.0 {
            return;
        }

        let mut cy = y;

        // Title
        if let Some(ref title) = self.data.title {
            let tx = x + (w.saturating_sub(title.len() as u16)) / 2;
            buf.set_string(tx, cy, title, Style::default().fg(TITLE_COLOR));
            cy += 1;
        }

        // Chart dimensions
        // Legend width: enough for "█label 100%"
        let legend_w: u16 = 16;
        let chart_area_w = w.saturating_sub(legend_w + 1);
        let chart_h = h.saturating_sub((cy - y) + 1);

        // Canvas uses all available space
        let canvas_w = chart_area_w as usize;
        let canvas_h = chart_h as usize;

        // Radius in dot-space
        let dot_r = if let Some(r) = self.data.radius {
            r as usize
        } else {
            // Default: fill the canvas
            canvas_w.max(canvas_h * 2)
        };

        if canvas_w < 4 || canvas_h < 2 {
            return;
        }

        // Center in dot-space
        let dot_cx = canvas_w; // dot_width = canvas_w * 2, center at half
        let dot_cy = canvas_h * 2; // dot_height = canvas_h * 4, center at half
        let dot_r_adj = dot_r; // radius in dot-space

        // Build sectors
        let mut angle = 0.0f64;
        let sectors: Vec<(f64, f64)> = values
            .iter()
            .map(|&v| {
                let sweep = (v / total) * TAU;
                let start = angle;
                angle += sweep;
                (start, start + sweep)
            })
            .collect();

        // Render each sector with its own canvas and color
        for (i, &(start, end)) in sectors.iter().enumerate() {
            let color = CHART_COLORS[i % CHART_COLORS.len()];
            let color_style = Style::default().fg(color);
            let mut canvas = BrailleCanvas::new(canvas_w, canvas_h);
            canvas.fill_sector(dot_cx, dot_cy, dot_r_adj, start, end);

            // Write to buffer
            let offset_x = x + (chart_area_w.saturating_sub(canvas_w as u16)) / 2;
            for (ry, line) in canvas.rows().iter().enumerate() {
                for (cx, ch) in line.chars().enumerate() {
                    if ch != '⠀' && ch != ' ' {
                        let s = ch.to_string();
                        buf.set_string(offset_x + cx as u16, cy + ry as u16, &s, color_style);
                    }
                }
            }
        }

        // Legend (right side, compact format)
        let label_style = Style::default().fg(LABEL_COLOR);
        let legend_x = x + chart_area_w + 1;
        for (i, &(_, _)) in sectors.iter().enumerate() {
            let color = CHART_COLORS[i % CHART_COLORS.len()];
            let pct = (values[i] / total) * 100.0;
            let label: String = if i < labels.len() {
                // Truncate label to fit legend width (char-safe for CJK)
                labels[i].chars().take(10).collect()
            } else {
                "?".to_string()
            };
            let ly = cy + i as u16;
            if ly < y + h {
                buf.set_string(legend_x, ly, "█", Style::default().fg(color));
                let text = format!("{} {:.0}%", label, pct);
                buf.set_string(legend_x + 2, ly, &text, label_style);
            }
        }
    }
}
