use super::{AXIS_COLOR, CHART_COLORS, LABEL_COLOR};
use rust_pixel::render::buffer::Buffer;
use rust_pixel::render::style::{Color, Style};
use std::collections::HashMap;

/// Direction of the flowchart.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Direction {
    TopDown,
    LeftRight,
}

#[derive(Debug, Clone)]
pub struct MermaidNode {
    pub id: String,
    pub label: String,
}

#[derive(Debug, Clone)]
pub struct MermaidEdge {
    pub from: String,
    pub to: String,
    pub label: Option<String>,
}

#[derive(Debug, Clone)]
pub struct MermaidGraph {
    pub direction: Direction,
    pub nodes: Vec<MermaidNode>,
    pub edges: Vec<MermaidEdge>,
}

/// Parse mermaid graph source. Returns None if unsupported syntax.
pub fn parse_mermaid(content: &str) -> Option<MermaidGraph> {
    let mut lines = content.lines().map(|l| l.trim()).filter(|l| !l.is_empty());

    // First line must be "graph TD" or "graph LR"
    let first = lines.next()?;
    let direction = if first.starts_with("graph") {
        let dir_str = first.strip_prefix("graph")?.trim();
        match dir_str {
            "TD" | "TB" => Direction::TopDown,
            "LR" => Direction::LeftRight,
            _ => return None,
        }
    } else {
        return None;
    };

    let mut node_map: HashMap<String, String> = HashMap::new();
    let mut edges: Vec<MermaidEdge> = Vec::new();

    for line in lines {
        if let Some(edge) = parse_edge_line(line, &mut node_map) {
            edges.push(edge);
        } else {
            parse_node_def(line, &mut node_map);
        }
    }

    // Build node list preserving insertion order via edges
    let mut seen = Vec::new();
    for edge in &edges {
        if !seen.contains(&edge.from) {
            seen.push(edge.from.clone());
        }
        if !seen.contains(&edge.to) {
            seen.push(edge.to.clone());
        }
    }
    for id in node_map.keys() {
        if !seen.contains(id) {
            seen.push(id.clone());
        }
    }

    let nodes: Vec<MermaidNode> = seen
        .into_iter()
        .map(|id| {
            let label = node_map.get(&id).cloned().unwrap_or_else(|| id.clone());
            MermaidNode { id, label }
        })
        .collect();

    Some(MermaidGraph {
        direction,
        nodes,
        edges,
    })
}

fn parse_edge_line(line: &str, node_map: &mut HashMap<String, String>) -> Option<MermaidEdge> {
    let arrow_pos = line.find("-->")?;
    let left = line[..arrow_pos].trim();
    let right_part = line[arrow_pos + 3..].trim();

    let (label, right) = if right_part.starts_with('|') {
        if let Some(end) = right_part[1..].find('|') {
            let lbl = right_part[1..1 + end].to_string();
            let rest = right_part[2 + end..].trim();
            (Some(lbl), rest)
        } else {
            (None, right_part)
        }
    } else {
        (None, right_part)
    };

    let (from_id, _) = extract_node(left, node_map);
    let (to_id, _) = extract_node(right, node_map);

    if from_id.is_empty() || to_id.is_empty() {
        return None;
    }

    Some(MermaidEdge {
        from: from_id,
        to: to_id,
        label,
    })
}

fn parse_node_def(line: &str, node_map: &mut HashMap<String, String>) {
    let line = line.trim().trim_end_matches(';');
    extract_node(line, node_map);
}

fn extract_node(s: &str, node_map: &mut HashMap<String, String>) -> (String, String) {
    let s = s.trim().trim_end_matches(';');
    if s.is_empty() {
        return (String::new(), String::new());
    }

    for (i, c) in s.char_indices() {
        if c == '[' || c == '(' || c == '{' {
            let id = s[..i].trim().to_string();
            let close = match c {
                '[' => ']',
                '(' => ')',
                '{' => '}',
                _ => unreachable!(),
            };
            if let Some(end) = s[i + 1..].find(close) {
                let label = s[i + 1..i + 1 + end].to_string();
                node_map.insert(id.clone(), label.clone());
                return (id, label);
            }
        }
    }

    let id = s.to_string();
    if !node_map.contains_key(&id) {
        node_map.insert(id.clone(), id.clone());
    }
    let label = node_map.get(&id).cloned().unwrap_or_else(|| id.clone());
    (id, label)
}

/// Render a MermaidGraph to buffer.
pub fn render_mermaid(graph: &MermaidGraph, buf: &mut Buffer, x: u16, y: u16, w: u16, h: u16) {
    if graph.nodes.is_empty() {
        return;
    }

    let layers = assign_layers(graph);
    if layers.is_empty() {
        return;
    }

    match graph.direction {
        Direction::TopDown => render_td(graph, &layers, buf, x, y, w, h),
        Direction::LeftRight => render_lr(graph, &layers, buf, x, y, w, h),
    }
}

fn assign_layers(graph: &MermaidGraph) -> Vec<Vec<usize>> {
    let n = graph.nodes.len();
    let id_to_idx: HashMap<&str, usize> = graph
        .nodes
        .iter()
        .enumerate()
        .map(|(i, n)| (n.id.as_str(), i))
        .collect();

    let mut in_degree = vec![0usize; n];
    let mut adj: Vec<Vec<usize>> = vec![Vec::new(); n];

    for edge in &graph.edges {
        if let (Some(&from), Some(&to)) = (id_to_idx.get(edge.from.as_str()), id_to_idx.get(edge.to.as_str())) {
            adj[from].push(to);
            in_degree[to] += 1;
        }
    }

    let mut layers: Vec<Vec<usize>> = Vec::new();
    let mut assigned = vec![false; n];
    let mut queue: Vec<usize> = (0..n).filter(|&i| in_degree[i] == 0).collect();

    while !queue.is_empty() {
        for &idx in &queue {
            assigned[idx] = true;
        }
        layers.push(queue.clone());

        let mut next = Vec::new();
        for &idx in &queue {
            for &child in &adj[idx] {
                in_degree[child] -= 1;
                if in_degree[child] == 0 && !assigned[child] {
                    next.push(child);
                }
            }
        }
        queue = next;
    }

    let remaining: Vec<usize> = (0..n).filter(|i| !assigned[*i]).collect();
    if !remaining.is_empty() {
        layers.push(remaining);
    }

    layers
}

fn render_td(
    graph: &MermaidGraph,
    layers: &[Vec<usize>],
    buf: &mut Buffer,
    x: u16,
    y: u16,
    w: u16,
    h: u16,
) {
    let n_layers = layers.len();
    if n_layers == 0 {
        return;
    }

    let node_widths: Vec<u16> = graph
        .nodes
        .iter()
        .map(|n| n.label.len() as u16 + 4)
        .collect();

    let layer_h = (h / n_layers as u16).max(3);
    let node_h: u16 = 3;

    let id_to_idx: HashMap<&str, usize> = graph
        .nodes
        .iter()
        .enumerate()
        .map(|(i, n)| (n.id.as_str(), i))
        .collect();

    let mut positions: Vec<(u16, u16)> = vec![(0, 0); graph.nodes.len()];

    for (li, layer) in layers.iter().enumerate() {
        let layer_y = y + li as u16 * layer_h;
        let total_w: u16 = layer.iter().map(|&i| node_widths[i] + 2).sum::<u16>();
        let mut nx = x + w.saturating_sub(total_w) / 2;

        for &idx in layer {
            let nw = node_widths[idx];
            positions[idx] = (nx, layer_y);
            nx += nw + 2;
        }
    }

    for (idx, node) in graph.nodes.iter().enumerate() {
        let (nx, ny) = positions[idx];
        let nw = node_widths[idx];
        let color = CHART_COLORS[idx % CHART_COLORS.len()];
        draw_box(buf, nx, ny, nw, node_h, &node.label, color);
    }

    for edge in &graph.edges {
        let from_idx = id_to_idx.get(edge.from.as_str()).copied().unwrap_or(0);
        let to_idx = id_to_idx.get(edge.to.as_str()).copied().unwrap_or(0);

        let (fx, fy) = positions[from_idx];
        let fw = node_widths[from_idx];
        let (tx, ty) = positions[to_idx];
        let tw = node_widths[to_idx];

        let from_cx = fx + fw / 2;
        let from_y = fy + node_h;
        let to_cx = tx + tw / 2;
        let to_y = ty;

        draw_vertical_edge(buf, from_cx, from_y, to_cx, to_y, &edge.label);
    }
}

fn render_lr(
    graph: &MermaidGraph,
    layers: &[Vec<usize>],
    buf: &mut Buffer,
    x: u16,
    y: u16,
    w: u16,
    h: u16,
) {
    let n_layers = layers.len();
    if n_layers == 0 {
        return;
    }

    let node_widths: Vec<u16> = graph
        .nodes
        .iter()
        .map(|n| n.label.len() as u16 + 4)
        .collect();

    let max_nw = *node_widths.iter().max().unwrap_or(&8);
    let layer_w = (w / n_layers as u16).max(max_nw + 4);
    let node_h: u16 = 3;

    let id_to_idx: HashMap<&str, usize> = graph
        .nodes
        .iter()
        .enumerate()
        .map(|(i, n)| (n.id.as_str(), i))
        .collect();

    let mut positions: Vec<(u16, u16)> = vec![(0, 0); graph.nodes.len()];

    for (li, layer) in layers.iter().enumerate() {
        let layer_x = x + li as u16 * layer_w;
        let total_h = layer.len() as u16 * (node_h + 1);
        let mut ny = y + h.saturating_sub(total_h) / 2;

        for &idx in layer {
            positions[idx] = (layer_x, ny);
            ny += node_h + 1;
        }
    }

    for (idx, node) in graph.nodes.iter().enumerate() {
        let (nx, ny) = positions[idx];
        let nw = node_widths[idx];
        let color = CHART_COLORS[idx % CHART_COLORS.len()];
        draw_box(buf, nx, ny, nw, node_h, &node.label, color);
    }

    for edge in &graph.edges {
        let from_idx = id_to_idx.get(edge.from.as_str()).copied().unwrap_or(0);
        let to_idx = id_to_idx.get(edge.to.as_str()).copied().unwrap_or(0);

        let (fx, fy) = positions[from_idx];
        let fw = node_widths[from_idx];
        let (tx, ty) = positions[to_idx];

        let from_x = fx + fw;
        let from_y = fy + 1;
        let to_x = tx;
        let to_y = ty + 1;

        draw_horizontal_edge(buf, from_x, from_y, to_x, to_y, &edge.label);
    }
}

fn draw_box(buf: &mut Buffer, x: u16, y: u16, w: u16, _h: u16, label: &str, color: Color) {
    let style = Style::default().fg(color);
    let inner_w = w.saturating_sub(2) as usize;
    let top = format!("┌{}┐", "─".repeat(inner_w));
    let bot = format!("└{}┘", "─".repeat(inner_w));
    let label_padded = format!("{:^width$}", label, width = inner_w);
    let mid = format!("│{}│", label_padded);

    buf.set_string(x, y, &top, style);
    buf.set_string(x, y + 1, &mid, style);
    buf.set_string(x, y + 2, &bot, style);
}

fn draw_vertical_edge(
    buf: &mut Buffer,
    fx: u16,
    fy: u16,
    tx: u16,
    ty: u16,
    label: &Option<String>,
) {
    let axis_style = Style::default().fg(AXIS_COLOR);
    let label_style = Style::default().fg(LABEL_COLOR);

    if fy >= ty {
        return;
    }
    let mid_y = (fy + ty) / 2;

    for row in fy..ty {
        if row == ty - 1 {
            buf.set_string(fx, row, "↓", axis_style);
        } else {
            buf.set_string(fx, row, "│", axis_style);
        }
    }

    if fx != tx {
        let (lx, rx) = if fx < tx { (fx, tx) } else { (tx, fx) };
        for col in lx + 1..rx {
            buf.set_string(col, mid_y, "─", axis_style);
        }
        if fx < tx {
            buf.set_string(fx, mid_y, "└", axis_style);
            buf.set_string(tx, mid_y, "┐", axis_style);
        } else {
            buf.set_string(fx, mid_y, "┘", axis_style);
            buf.set_string(tx, mid_y, "┌", axis_style);
        }
        for row in mid_y + 1..ty {
            if row == ty - 1 {
                buf.set_string(tx, row, "↓", axis_style);
            } else {
                buf.set_string(tx, row, "│", axis_style);
            }
        }
    }

    if let Some(ref lbl) = label {
        let lx = fx + 1;
        buf.set_string(lx, mid_y, lbl, label_style);
    }
}

fn draw_horizontal_edge(
    buf: &mut Buffer,
    fx: u16,
    fy: u16,
    tx: u16,
    ty: u16,
    label: &Option<String>,
) {
    let axis_style = Style::default().fg(AXIS_COLOR);
    let label_style = Style::default().fg(LABEL_COLOR);

    if fx >= tx {
        return;
    }

    if fy == ty {
        for col in fx..tx {
            if col == tx - 1 {
                buf.set_string(col, fy, "→", axis_style);
            } else {
                buf.set_string(col, fy, "─", axis_style);
            }
        }
    } else {
        let mid_x = (fx + tx) / 2;
        for col in fx..mid_x {
            buf.set_string(col, fy, "─", axis_style);
        }
        let (top_y, bot_y) = if fy < ty { (fy, ty) } else { (ty, fy) };
        for row in top_y..=bot_y {
            buf.set_string(mid_x, row, "│", axis_style);
        }
        for col in mid_x + 1..tx {
            if col == tx - 1 {
                buf.set_string(col, ty, "→", axis_style);
            } else {
                buf.set_string(col, ty, "─", axis_style);
            }
        }
    }

    if let Some(ref lbl) = label {
        let mid_x = (fx + tx) / 2;
        buf.set_string(mid_x + 1, fy.min(ty), lbl, label_style);
    }
}
