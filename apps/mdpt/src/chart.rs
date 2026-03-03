pub mod braille;
pub mod line_chart;
pub mod bar_chart;
pub mod pie_chart;
pub mod mermaid;

use rust_pixel::render::buffer::Buffer;
use rust_pixel::render::style::Color;

/// Chart color palette — cycles for multi-series/multi-sector data.
pub const CHART_COLORS: [Color; 8] = [
    Color::LightRed,
    Color::LightGreen,
    Color::LightBlue,
    Color::LightYellow,
    Color::LightMagenta,
    Color::LightCyan,
    Color::Indexed(208), // Orange
    Color::Indexed(141), // Purple
];

/// Axis/border color.
pub const AXIS_COLOR: Color = Color::Rgba(120, 130, 140, 255);
/// Title color.
pub const TITLE_COLOR: Color = Color::White;
/// Label color.
pub const LABEL_COLOR: Color = Color::Rgba(180, 180, 180, 255);

/// Parsed chart data from a code block.
#[derive(Debug, Clone)]
pub struct ChartData {
    pub title: Option<String>,
    pub labels: Vec<String>,
    pub series: Vec<Vec<f64>>,
    pub series_names: Vec<String>,
    pub width: Option<u16>,
    pub height: Option<u16>,
    pub radius: Option<u16>,
}

/// Common trait for rendering a chart to a Buffer region.
pub trait ChartRenderer {
    fn render(&self, buf: &mut Buffer, x: u16, y: u16, w: u16, h: u16);
}

/// Parse a simple key: value format from chart code block content.
///
/// Supported fields:
///   title: Chart Title
///   x: [label1, label2, ...]
///   labels: [label1, label2, ...]   (alias for x)
///   y: [1.0, 2.5, 3.0]
///   y2: [4.0, 5.0]                  (additional series)
///   values: [1, 2, 3]               (alias for y)
///   width: 60
///   height: 15
///   radius: 8
pub fn parse_chart_data(content: &str) -> ChartData {
    let mut data = ChartData {
        title: None,
        labels: Vec::new(),
        series: Vec::new(),
        series_names: Vec::new(),
        width: None,
        height: None,
        radius: None,
    };

    for line in content.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        let Some((key, value)) = line.split_once(':') else {
            continue;
        };
        let key = key.trim().to_lowercase();
        let value = value.trim();

        match key.as_str() {
            "title" => {
                data.title = Some(value.to_string());
            }
            "x" | "labels" => {
                data.labels = parse_string_array(value);
            }
            "values" | "y" => {
                data.series_names.push("y".to_string());
                data.series.push(parse_number_array(value));
            }
            "width" => {
                data.width = value.parse().ok();
            }
            "height" => {
                data.height = value.parse().ok();
            }
            "radius" => {
                data.radius = value.parse().ok();
            }
            other => {
                // y2, y3, etc.
                if other.starts_with('y') && other.len() > 1 {
                    data.series_names.push(other.to_string());
                    data.series.push(parse_number_array(value));
                }
            }
        }
    }

    data
}

/// Parse "[item1, item2, item3]" into Vec<String>.
fn parse_string_array(s: &str) -> Vec<String> {
    let s = s.trim();
    let s = s.strip_prefix('[').unwrap_or(s);
    let s = s.strip_suffix(']').unwrap_or(s);
    s.split(',')
        .map(|item| item.trim().trim_matches('"').trim_matches('\'').to_string())
        .filter(|item| !item.is_empty())
        .collect()
}

/// Parse "[1.0, 2.5, 3]" into Vec<f64>.
fn parse_number_array(s: &str) -> Vec<f64> {
    let s = s.trim();
    let s = s.strip_prefix('[').unwrap_or(s);
    let s = s.strip_suffix(']').unwrap_or(s);
    s.split(',')
        .filter_map(|item| item.trim().parse::<f64>().ok())
        .collect()
}
