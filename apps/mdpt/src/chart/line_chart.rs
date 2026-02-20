use super::braille::BrailleCanvas;
use super::{ChartData, ChartRenderer, AXIS_COLOR, CHART_COLORS, LABEL_COLOR, TITLE_COLOR};
use rust_pixel::render::buffer::Buffer;
use rust_pixel::render::style::Style;
use unicode_width::UnicodeWidthStr;

pub struct LineChart {
    pub data: ChartData,
}

impl LineChart {
    pub fn new(data: ChartData) -> Self {
        Self { data }
    }
}

impl ChartRenderer for LineChart {
    fn render(&self, buf: &mut Buffer, x: u16, y: u16, w: u16, h: u16) {
        if self.data.series.is_empty() || h < 5 {
            return;
        }

        let mut cy = y;

        // Title
        if let Some(ref title) = self.data.title {
            let tx = x + (w.saturating_sub(title.width() as u16)) / 2;
            buf.set_string(tx, cy, title, Style::default().fg(TITLE_COLOR));
            cy += 1;
        }

        // Layout
        let y_label_w: u16 = 6;
        let chart_x = x + y_label_w;
        let chart_w = w.saturating_sub(y_label_w + 1);
        let chart_h = h.saturating_sub((cy - y) + 2); // reserve for bottom labels

        if chart_w < 4 || chart_h < 2 {
            return;
        }

        // Find global min/max across all series
        let (mut min_val, mut max_val) = (f64::INFINITY, f64::NEG_INFINITY);
        for series in &self.data.series {
            for &v in series {
                min_val = min_val.min(v);
                max_val = max_val.max(v);
            }
        }
        if min_val == max_val {
            max_val = min_val + 1.0;
        }
        // Give some padding
        let range = max_val - min_val;
        let pad = range * 0.05;
        let min_val = min_val - pad;
        let max_val = max_val + pad;

        let label_style = Style::default().fg(LABEL_COLOR);
        let axis_style = Style::default().fg(AXIS_COLOR);

        // Draw Y axis labels
        let top_label = format!("{:>5}", format_val(max_val));
        buf.set_string(x, cy, &top_label, label_style);
        let mid_val = (min_val + max_val) / 2.0;
        let mid_y = cy + chart_h / 2;
        let mid_label = format!("{:>5}", format_val(mid_val));
        buf.set_string(x, mid_y, &mid_label, label_style);
        let bot_label = format!("{:>5}", format_val(min_val));
        buf.set_string(x, cy + chart_h - 1, &bot_label, label_style);

        // Draw Y axis line
        for row in 0..chart_h {
            buf.set_string(chart_x - 1, cy + row, "│", axis_style);
        }
        // Draw X axis
        let x_axis: String = "─".repeat(chart_w as usize);
        buf.set_string(chart_x, cy + chart_h, &x_axis, axis_style);
        buf.set_string(chart_x - 1, cy + chart_h, "└", axis_style);

        // Render each series with Braille
        for (si, series) in self.data.series.iter().enumerate() {
            if series.len() < 2 {
                continue;
            }
            let color = CHART_COLORS[si % CHART_COLORS.len()];
            let color_style = Style::default().fg(color);

            // Create a braille canvas for this series
            let mut canvas = BrailleCanvas::new(chart_w as usize, chart_h as usize);
            let dot_w = canvas.dot_width();
            let dot_h = canvas.dot_height();

            let n = series.len();
            for i in 0..n - 1 {
                let x0 = (i * (dot_w - 1)) / (n - 1);
                let x1 = ((i + 1) * (dot_w - 1)) / (n - 1);
                let y0 = val_to_dot(series[i], min_val, max_val, dot_h);
                let y1 = val_to_dot(series[i + 1], min_val, max_val, dot_h);
                canvas.line(x0, y0, x1, y1);
            }

            // Write Braille chars to buffer
            for row in canvas.rows().iter().enumerate() {
                let (ry, line) = row;
                for (cx, ch) in line.chars().enumerate() {
                    if ch != '⠀' && ch != ' ' {
                        let s = ch.to_string();
                        buf.set_string(chart_x + cx as u16, cy + ry as u16, &s, color_style);
                    }
                }
            }
        }

        // X axis labels
        let labels = &self.data.labels;
        if !labels.is_empty() {
            let n = labels.len();
            for (i, label) in labels.iter().enumerate() {
                let lx = if n > 1 {
                    chart_x + (i as u16 * (chart_w - 1)) / (n as u16 - 1).max(1)
                } else {
                    chart_x + chart_w / 2
                };
                let truncated: String = label.chars().take(6).collect();
                let lx = lx.saturating_sub(truncated.len() as u16 / 2);
                buf.set_string(lx, cy + chart_h + 1, &truncated, label_style);
            }
        }

        // Legend (if multiple series)
        if self.data.series.len() > 1 {
            let legend_y = cy + chart_h + 1;
            let mut lx = chart_x + chart_w.saturating_sub(self.data.series.len() as u16 * 8);
            for (si, name) in self.data.series_names.iter().enumerate() {
                let color = CHART_COLORS[si % CHART_COLORS.len()];
                buf.set_string(lx, legend_y, "━", Style::default().fg(color));
                lx += 1;
                buf.set_string(lx, legend_y, name, label_style);
                lx += name.len() as u16 + 1;
            }
        }
    }
}

/// Map a value to braille dot-space Y coordinate (top=0).
fn val_to_dot(val: f64, min_val: f64, max_val: f64, dot_h: usize) -> usize {
    let fraction = (val - min_val) / (max_val - min_val);
    let dot_y = ((1.0 - fraction) * (dot_h - 1) as f64).round() as usize;
    dot_y.min(dot_h - 1)
}

fn format_val(v: f64) -> String {
    if v == v.floor() && v.abs() < 100000.0 {
        format!("{}", v as i64)
    } else {
        format!("{:.1}", v)
    }
}
