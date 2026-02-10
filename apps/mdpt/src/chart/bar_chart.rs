use super::{ChartData, ChartRenderer, AXIS_COLOR, CHART_COLORS, LABEL_COLOR, TITLE_COLOR};
use rust_pixel::render::buffer::Buffer;
use rust_pixel::render::style::Style;

/// Block element characters for 1/8 height precision (index 0 = empty, 8 = full).
const BLOCKS: [char; 9] = [' ', '▁', '▂', '▃', '▄', '▅', '▆', '▇', '█'];

pub struct BarChart {
    pub data: ChartData,
}

impl BarChart {
    pub fn new(data: ChartData) -> Self {
        Self { data }
    }
}

impl ChartRenderer for BarChart {
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

        let mut cy = y;
        let label_style = Style::default().fg(LABEL_COLOR);
        let axis_style = Style::default().fg(AXIS_COLOR);

        // Title
        if let Some(ref title) = self.data.title {
            let tx = x + (w.saturating_sub(title.len() as u16)) / 2;
            buf.set_string(tx, cy, title, Style::default().fg(TITLE_COLOR));
            cy += 1;
        }

        // Chart area dimensions
        let y_label_w: u16 = 6; // space for Y axis labels
        let chart_x = x + y_label_w;
        let chart_w = w.saturating_sub(y_label_w + 1);
        let chart_h = h.saturating_sub((cy - y) + 2); // reserve for title + bottom labels

        if chart_w < 2 || chart_h < 2 {
            return;
        }

        // Find max value for scaling
        let max_val = values.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
        let max_val = if max_val <= 0.0 { 1.0 } else { max_val };

        // Calculate bar width and spacing
        let bar_total = chart_w as usize / n.max(1);
        let bar_w = bar_total.saturating_sub(1).max(1);

        // Draw Y axis labels (top and bottom)
        let top_label = format!("{:>5}", format_val(max_val));
        buf.set_string(x, cy, &top_label, label_style);
        let bottom_label = format!("{:>5}", "0");
        buf.set_string(x, cy + chart_h - 1, &bottom_label, label_style);

        // Draw Y axis line
        for row in 0..chart_h {
            buf.set_string(chart_x - 1, cy + row, "│", axis_style);
        }
        // Draw X axis line
        let x_axis = "─".repeat(chart_w as usize);
        buf.set_string(chart_x, cy + chart_h, &x_axis, axis_style);
        // Corner
        buf.set_string(chart_x - 1, cy + chart_h, "└", axis_style);

        // Draw bars
        for (i, &val) in values.iter().enumerate() {
            let color = CHART_COLORS[i % CHART_COLORS.len()];
            let color_style = Style::default().fg(color);
            let bx = chart_x + (i * bar_total) as u16;

            // Bar height in sub-rows (each char = 8 sub-rows)
            let fraction = val / max_val;
            let total_sub = (fraction * chart_h as f64 * 8.0).round() as usize;
            let full_rows = total_sub / 8;
            let partial = total_sub % 8;

            // Draw from bottom up
            for row in 0..full_rows {
                let ry = cy + chart_h - 1 - row as u16;
                let block_str: String = std::iter::repeat(BLOCKS[8]).take(bar_w).collect();
                buf.set_string(bx, ry, &block_str, color_style);
            }

            // Partial top block
            if partial > 0 && full_rows < chart_h as usize {
                let ry = cy + chart_h - 1 - full_rows as u16;
                let block_str: String = std::iter::repeat(BLOCKS[partial]).take(bar_w).collect();
                buf.set_string(bx, ry, &block_str, color_style);
            }

            // Value label on top of bar
            let val_str = format_val(val);
            if full_rows + 1 < chart_h as usize {
                let vy = cy + chart_h - 2 - full_rows as u16;
                let vx = bx + (bar_w as u16).saturating_sub(val_str.len() as u16) / 2;
                buf.set_string(vx, vy, &val_str, label_style);
            }

            // Bottom label
            if i < labels.len() {
                let label = &labels[i];
                let truncated: String = label.chars().take(bar_total).collect();
                buf.set_string(bx, cy + chart_h + 1, &truncated, label_style);
            }
        }
    }
}

/// Format a number for display (compact).
fn format_val(v: f64) -> String {
    if v == v.floor() && v.abs() < 100000.0 {
        format!("{}", v as i64)
    } else {
        format!("{:.1}", v)
    }
}
