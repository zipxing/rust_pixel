#![allow(unused_imports)]
use crate::model::{ArenaState, LlmArenaModel, Terrain, MAP_HEIGHT, MAP_WIDTH, SCREEN_HEIGHT, SCREEN_WIDTH};
use rust_pixel::context::Context;
use rust_pixel::game::Render;
use rust_pixel::render::scene::Scene;
use rust_pixel::render::sprite::Sprite;
use rust_pixel::render::style::Color;
use rust_pixel::ui::{BorderStyle, Panel, Widget};
use rust_pixel::util::Rect;

/// 2x2 resource icon: [[top-left, top-right], [bottom-left, bottom-right]]
/// Each cell is (block, sym_idx)
type ResourceIcon = [[(u8, u8); 2]; 2];

/// Resource icons loaded from pix files
struct ResourceIcons {
    food: ResourceIcon,    // 食物
    gems: ResourceIcon,    // 宝石
    ammo: ResourceIcon,    // 弹药 (fire)
    med: ResourceIcon,     // 医疗 (yaoshui)
}

impl Default for ResourceIcons {
    fn default() -> Self {
        // Fallback: PETSCII patterns
        Self {
            food: [[(0, 85), (0, 73)], [(0, 74), (0, 75)]],
            gems: [[(0, 78), (0, 77)], [(0, 76), (0, 122)]],
            ammo: [[(0, 81), (0, 82)], [(0, 83), (0, 84)]],
            med: [[(0, 91), (0, 93)], [(0, 91), (0, 93)]],
        }
    }
}

impl ResourceIcons {
    /// Load icons from pix files in the project assets directory
    fn load(project_path: &str) -> Self {
        let mut icons = Self::default();

        // Try to load each pix file
        if let Some(icon) = Self::parse_pix(&format!("{}/assets/food.pix", project_path)) {
            icons.food = icon;
        }
        if let Some(icon) = Self::parse_pix(&format!("{}/assets/baoshi.pix", project_path)) {
            icons.gems = icon;
        }
        if let Some(icon) = Self::parse_pix(&format!("{}/assets/fire.pix", project_path)) {
            icons.ammo = icon;
        }
        if let Some(icon) = Self::parse_pix(&format!("{}/assets/yaoshui.pix", project_path)) {
            icons.med = icon;
        }

        icons
    }

    /// Parse a 2x2 pix file and extract (block, sym) for each cell
    fn parse_pix(path: &str) -> Option<ResourceIcon> {
        let content = std::fs::read_to_string(path).ok()?;
        let lines: Vec<&str> = content.lines().collect();

        if lines.len() < 3 {
            return None;
        }

        // Skip header, parse 2 data lines
        let mut icon: ResourceIcon = [[(0, 0); 2]; 2];

        for (row, line) in lines[1..=2].iter().enumerate() {
            for (col, cell_str) in line.split_whitespace().enumerate().take(2) {
                // Format: sym,fg,block,bg
                let parts: Vec<&str> = cell_str.split(',').collect();
                if parts.len() >= 3 {
                    let sym = parts[0].parse::<u8>().unwrap_or(0);
                    let block = parts[2].parse::<u8>().unwrap_or(0);
                    icon[row][col] = (block, sym);
                }
            }
        }

        Some(icon)
    }

    /// Get icon for resource type (0=food, 1=gems, 2=ammo, 3=med)
    fn get(&self, resource_type: u8) -> &ResourceIcon {
        match resource_type {
            0 => &self.food,
            1 => &self.gems,
            2 => &self.ammo,
            _ => &self.med,
        }
    }
}

// Layout constants (cell coords in TUI buffer) — doubled for ratio 4.0
const MAP_W: u16 = 160;
const MAP_H: u16 = 66;
const PANEL_W: u16 = 80;
const PANEL_H: u16 = 32;
const LOG_H: u16 = 34;
const STATUS_H: u16 = 14;

// Map sprite: W = panel content width, H = panel content height * 2 (TUI行高是sprite的2倍)
// scale 1.0, no scaling needed
const MAP_GFX_W: u16 = MAP_W - 2;          // 158
const MAP_GFX_H: u16 = (MAP_H - 3) * 2;   // 126

// Colors
const LABEL_COLOR: Color = Color::Indexed(244);
const TERRAIN_ALPHA: u8 = 230;  // 地形透明度 (0=透明, 255=不透明)
const HIGHLIGHT_COLOR: Color = Color::Yellow;
const LOG_COLOR: Color = Color::Indexed(250);

// Resource icon scale (each tile is rendered at this scale)
const ICON_SCALE: f32 = 2.0;  // 每个tile放大2倍

// Battle ripple animation constants
const RIPPLE_MAX_RADIUS: f32 = 15.0;    // 最大扩散半径 (世界坐标)
const RIPPLE_SPEED: f32 = 0.5;          // 扩散速度
const RIPPLE_SPAWN_INTERVAL: u32 = 20;  // 每隔多少帧生成新波纹

/// Expanding ripple effect for battles
struct BattleRipple {
    center: (f32, f32),    // 世界坐标中心点
    radius: f32,           // 当前半径
    battle_id: u32,        // 关联的战斗ID
}

pub struct LlmArenaRender {
    pub scene: Scene,
    pub map_panel: Panel,
    pub info_panel: Panel,
    pub log_panel: Panel,
    pub status_panel: Panel,
    resource_icons: ResourceIcons,
    ripples: Vec<BattleRipple>,
    frame_count: u32,
}

impl Default for LlmArenaRender {
    fn default() -> Self {
        Self::new()
    }
}

impl LlmArenaRender {
    pub fn new() -> Self {
        let mut scene = Scene::new();

        // Map graphics sprite: scale 1.0, W matches TUI width, H = TUI height * 2
        let map_gfx = Sprite::new(0, 0, MAP_GFX_W, MAP_GFX_H);
        scene.add_sprite(map_gfx, "map_gfx");

        // Resource icons sprite: smaller buffer, scaled up to match map_gfx size
        // Buffer size = map size / scale, so after scaling it matches map_gfx
        let icons_w = (MAP_GFX_W as f32 / ICON_SCALE).ceil() as u16;
        let icons_h = (MAP_GFX_H as f32 / ICON_SCALE).ceil() as u16;
        let mut icons_gfx = Sprite::new(0, 0, icons_w, icons_h);
        icons_gfx.scale_x = ICON_SCALE;
        icons_gfx.scale_y = ICON_SCALE;
        scene.add_sprite(icons_gfx, "icons_gfx");

        // TUI Panels
        let mut map_panel = Panel::new()
            .with_bounds(Rect::new(0, 0, MAP_W, MAP_H))
            .with_border(BorderStyle::Rounded)
            .with_title(" 战场地图 ");
        map_panel.enable_canvas(MAP_W - 2, MAP_H - 3);

        let mut info_panel = Panel::new()
            .with_bounds(Rect::new(MAP_W, 0, PANEL_W, PANEL_H))
            .with_border(BorderStyle::Rounded)
            .with_title(" 阵营 ");
        info_panel.enable_canvas(PANEL_W - 2, PANEL_H - 3);

        let mut log_panel = Panel::new()
            .with_bounds(Rect::new(MAP_W, PANEL_H, PANEL_W, LOG_H))
            .with_border(BorderStyle::Rounded)
            .with_title(" 日志 ");
        log_panel.enable_canvas(PANEL_W - 2, LOG_H - 3);

        let mut status_panel = Panel::new()
            .with_bounds(Rect::new(0, MAP_H, SCREEN_WIDTH, STATUS_H))
            .with_border(BorderStyle::Rounded)
            .with_title(" 状态 ");
        status_panel.enable_canvas(SCREEN_WIDTH - 2, STATUS_H - 3);

        Self {
            scene,
            map_panel,
            info_panel,
            log_panel,
            status_panel,
            resource_icons: ResourceIcons::default(),
            ripples: Vec::new(),
            frame_count: 0,
        }
    }

    /// Load resource icons from pix files
    pub fn load_icons(&mut self, project_path: &str) {
        self.resource_icons = ResourceIcons::load(project_path);
    }

    /// Draw map PETSCII graphics to sprite (overlay on map panel)
    /// Uses map_scale for zoom: higher scale = more zoom (smaller visible area)
    fn draw_map_gfx(&mut self, model: &LlmArenaModel) {
        let world = &model.world;
        let scale = model.map_scale;

        // Visible world area (smaller when zoomed in)
        let visible_w = MAP_GFX_W as f32 / scale;
        let visible_h = MAP_GFX_H as f32 / scale;

        // World coords of top-left corner of viewport
        let world_x0 = model.viewport.0 - visible_w / 2.0;
        let world_y0 = model.viewport.1 - visible_h / 2.0;

        // Helper: convert world coords to map_gfx sprite coords
        let to_sprite = |wx: f32, wy: f32| -> (i32, i32) {
            let sx = ((wx - world_x0) * scale) as i32;
            let sy = ((wy - world_y0) * scale) as i32;
            (sx, sy)
        };

        // Helper: check if sprite coords are in bounds (for map_gfx)
        let in_bounds = |sx: i32, sy: i32| -> bool {
            sx >= 0 && sx < MAP_GFX_W as i32 && sy >= 0 && sy < MAP_GFX_H as i32
        };

        // === Draw terrain to map_gfx ===
        {
            let gfx = self.scene.get_sprite("map_gfx");
            let buf = &mut gfx.content;
            buf.reset();

            for x in 0..MAP_GFX_W {
                for y in 0..MAP_GFX_H {
                    let wx = world_x0 + x as f32 / scale;
                    let wy = world_y0 + y as f32 / scale;
                    if wx < 0.0 || wy < 0.0 || wx >= MAP_WIDTH as f32 || wy >= MAP_HEIGHT as f32 {
                        buf.set_graph_sym(x, y, 0, 102, Color::DarkGray);
                    } else {
                        let tx = wx as usize;
                        let ty = wy as usize;
                        if ty < world.terrain.len() && tx < world.terrain[ty].len() {
                            let (sym, color) = terrain_symbol(world.terrain[ty][tx]);
                            buf.set_graph_sym(x, y, 0, sym, color);
                        }
                    }
                }
            }
        }

        // === Draw resource icons to icons_gfx (separate scaled sprite) ===
        {
            let icons_gfx = self.scene.get_sprite("icons_gfx");
            let icons_buf = &mut icons_gfx.content;
            icons_buf.reset();

            let icons_w = icons_buf.area.width as i32;
            let icons_h = icons_buf.area.height as i32;

            // Helper: check bounds for icons buffer
            let in_icons_bounds = |sx: i32, sy: i32| -> bool {
                sx >= 0 && sx < icons_w && sy >= 0 && sy < icons_h
            };

            for rp in &world.resource_points {
                let (sx, sy) = to_sprite(rp.position.0, rp.position.1);
                // Convert to icons_gfx coords (divide by ICON_SCALE since sprite is scaled)
                let ix = (sx as f32 / ICON_SCALE) as i32;
                let iy = (sy as f32 / ICON_SCALE) as i32;

                let icon = self.resource_icons.get(rp.resource_type);
                // Draw 2x2 icon
                for row in 0..2 {
                    for col in 0..2 {
                        let px = ix + col as i32;
                        let py = iy + row as i32;
                        if in_icons_bounds(px, py) {
                            let (block, sym) = icon[row][col];
                            icons_buf.set_graph_sym(px as u16, py as u16, block, sym, Color::White);
                        }
                    }
                }
            }
        }

        // === Update battle ripples ===
        self.frame_count = self.frame_count.wrapping_add(1);

        // Spawn new ripples for active battles
        if self.frame_count % RIPPLE_SPAWN_INTERVAL == 0 {
            for battle in &world.battles {
                self.ripples.push(BattleRipple {
                    center: battle.position,
                    radius: 0.0,
                    battle_id: battle.id,
                });
            }
        }

        // Update existing ripples
        for ripple in &mut self.ripples {
            ripple.radius += RIPPLE_SPEED;
        }

        // Remove ripples that exceeded max radius OR whose battle ended
        let active_battle_ids: Vec<u32> = world.battles.iter().map(|b| b.id).collect();
        self.ripples.retain(|r| {
            r.radius < RIPPLE_MAX_RADIUS && active_battle_ids.contains(&r.battle_id)
        });

        // === Draw battles, armies, safe zone to map_gfx ===
        {
            let gfx = self.scene.get_sprite("map_gfx");
            let buf = &mut gfx.content;

            // Draw battle ripples (expanding circles)
            for ripple in &self.ripples {
                let num_points = ((ripple.radius * 8.0) as i32).max(16);
                // Alpha fades as ripple expands
                let alpha = ((1.0 - ripple.radius / RIPPLE_MAX_RADIUS) * 200.0) as u8;
                let color = Color::Rgba(255, 200, 0, alpha); // Orange-yellow

                for i in 0..num_points {
                    let angle = (i as f32) * std::f32::consts::TAU / (num_points as f32);
                    let wx = ripple.center.0 + angle.cos() * ripple.radius;
                    let wy = ripple.center.1 + angle.sin() * ripple.radius;
                    let (sx, sy) = to_sprite(wx, wy);
                    if in_bounds(sx, sy) {
                        buf.set_graph_sym(sx as u16, sy as u16, 0, 42, color); // '*' symbol
                    }
                }
            }

            // Battles (scaled) - center marker
            let battle_size = (scale.ceil() as i32).max(1);
            for battle in &world.battles {
                let (sx, sy) = to_sprite(battle.position.0, battle.position.1);
                for dx in 0..battle_size {
                    for dy in 0..battle_size {
                        if in_bounds(sx + dx, sy + dy) {
                            buf.set_graph_sym((sx + dx) as u16, (sy + dy) as u16, 0, 42, Color::Yellow);
                        }
                    }
                }
            }

            // Armies (scaled blocks)
            let army_base_size = 2; // Base 2x2 in world space
            let army_size = ((army_base_size as f32 * scale).ceil() as i32).max(2);
            for army in &world.armies {
                if army.troops == 0 {
                    continue;
                }
                let (sx, sy) = to_sprite(army.position.0, army.position.1);
                let color = faction_color(army.faction_id);

                // Draw scaled army block
                for dx in 0..army_size {
                    for dy in 0..army_size {
                        if in_bounds(sx + dx, sy + dy) {
                            buf.set_graph_sym((sx + dx) as u16, (sy + dy) as u16, 0, 160, color);
                        }
                    }
                }

                // Moving indicator (arrow)
                if army.target.is_some() && army.engaged_lock == 0 {
                    let cx = sx + army_size / 2;
                    let cy = sy;
                    if in_bounds(cx, cy) {
                        buf.set_graph_sym(cx as u16, cy as u16, 0, 62, Color::White);
                    }
                }
            }

            // Draw safe zone boundary (circle)
            let zone = &world.zone;
            let num_points = 120; // Points to draw the circle
            for i in 0..num_points {
                let angle = (i as f32) * std::f32::consts::TAU / (num_points as f32);
                let wx = zone.center.0 + angle.cos() * zone.radius;
                let wy = zone.center.1 + angle.sin() * zone.radius;
                let (sx, sy) = to_sprite(wx, wy);
                if in_bounds(sx, sy) {
                    // Red warning color for zone boundary
                    buf.set_graph_sym(sx as u16, sy as u16, 0, 42, Color::Red); // '*' symbol
                }
            }
        }
    }

    /// Draw legend to map_panel canvas (TUI layer, supports Chinese text)
    fn draw_map_legend(&mut self) {
        self.map_panel.clear_canvas();
        let ly = MAP_H - 4; // bottom row of canvas
        self.map_panel.set_str(1, ly, "食", Color::Yellow, Color::Reset);
        self.map_panel.set_str(3, ly, "宝", Color::Gray, Color::Reset);
        self.map_panel.set_str(5, ly, "弹", Color::Red, Color::Reset);
        self.map_panel.set_str(7, ly, "药", Color::Green, Color::Reset);
    }

    /// Draw faction info to info_panel canvas
    fn draw_info(&mut self, model: &LlmArenaModel) {
        self.info_panel.clear_canvas();
        let world = &model.world;
        let canvas_h = PANEL_H - 3;

        let mut row = 0u16;
        for faction in &world.factions {
            if row >= canvas_h {
                break;
            }
            let color = faction_color(faction.id);
            let status = if faction.is_alive { "●" } else { "○" };

            let total_troops: u32 = world
                .armies
                .iter()
                .filter(|a| a.faction_id == faction.id)
                .map(|a| a.troops)
                .sum();
            let army_count = world
                .armies
                .iter()
                .filter(|a| a.faction_id == faction.id && a.troops > 0)
                .count();

            self.info_panel.set_str(0, row, status, color, Color::Reset);
            let header = format!("{} {}军 {}兵", faction.name, army_count, total_troops);
            self.info_panel.set_str(2, row, &header, color, Color::Reset);
            row += 1;

            // Per-army details
            for army in world.armies.iter().filter(|a| a.faction_id == faction.id && a.troops > 0) {
                if row >= canvas_h {
                    break;
                }
                let detail = format!(
                    " #{} {}兵 粮{:.0} 弹{:.0} 宝{:.0}",
                    army.id, army.troops, army.supplies, army.ammo, army.gems
                );
                self.info_panel.set_str(1, row, &detail, LABEL_COLOR, Color::Reset);
                row += 1;
            }
            row += 1;
        }

        // Battles section
        if !world.battles.is_empty() && row < canvas_h - 1 {
            let divider = "─".repeat((PANEL_W - 4) as usize);
            self.info_panel.set_str(0, row, &divider, LABEL_COLOR, Color::Reset);
            self.info_panel.set_str(
                ((PANEL_W - 2) as usize / 2 - 3) as u16,
                row,
                " 战斗 ",
                HIGHLIGHT_COLOR,
                Color::Reset,
            );
            row += 1;

            for (i, battle) in world.battles.iter().enumerate() {
                if row >= canvas_h || i >= 3 {
                    break;
                }
                let info = format!("#{} 第{}轮", battle.id, battle.duration);
                self.info_panel.set_str(0, row, &info, HIGHLIGHT_COLOR, Color::Reset);
                row += 1;
                for p in &battle.participants {
                    if row >= canvas_h {
                        break;
                    }
                    let fname = world
                        .factions
                        .iter()
                        .find(|f| f.id == p.faction_id)
                        .map(|f| f.name.as_str())
                        .unwrap_or("?");
                    let detail = format!("  {}: {}(-{})", fname, p.current_troops, p.casualties);
                    self.info_panel.set_str(0, row, &detail, faction_color(p.faction_id), Color::Reset);
                    row += 1;
                }
            }
        }
    }

    /// Draw log lines to log_panel canvas
    fn draw_log(&mut self, model: &LlmArenaModel) {
        self.log_panel.clear_canvas();
        let canvas_h = (LOG_H - 3) as usize;
        let max_w = (PANEL_W - 4) as usize;
        let lines = &model.world.log.lines;

        let start = lines.len().saturating_sub(canvas_h);
        for (i, line) in lines.iter().skip(start).enumerate() {
            if i >= canvas_h {
                break;
            }
            let display: String = line.chars().take(max_w).collect();
            self.log_panel.set_str(0, i as u16, &display, LOG_COLOR, Color::Reset);
        }
    }

    /// Draw status info to status_panel canvas
    fn draw_status(&mut self, model: &LlmArenaModel) {
        self.status_panel.clear_canvas();

        let (state_name, state_color) = match model.state {
            ArenaState::Init => ("初始化", Color::Gray),
            ArenaState::Running => ("运行中", Color::Green),
            ArenaState::Paused => ("已暂停", Color::Yellow),
            ArenaState::GameOver => ("已结束", Color::Red),
        };

        let line1 = format!(
            "回合:{} | 速度:{}x | {} | 安全区:{:.0} | 缩放:{:.1}x",
            model.world.tick, model.speed, state_name, model.world.zone.radius, model.map_scale
        );
        self.status_panel.set_str(0, 0, &line1, state_color, Color::Reset);

        self.status_panel.set_str(
            0, 2,
            "[空格]暂停  [+/-]速度  [方向键]平移  [ [/] ]缩放",
            LABEL_COLOR,
            Color::Reset,
        );

        if model.state == ArenaState::GameOver {
            if let Some(winner_id) = model.world.is_game_over() {
                let winner_name = if winner_id == 255 {
                    "平局".to_string()
                } else {
                    model
                        .world
                        .factions
                        .iter()
                        .find(|f| f.id == winner_id)
                        .map(|f| f.name.clone())
                        .unwrap_or("未知".to_string())
                };
                let win_msg = format!("胜者: {}", winner_name);
                self.status_panel.set_str(50, 0, &win_msg, HIGHLIGHT_COLOR, Color::Reset);
            }
        }
    }
}

/// Map terrain type to PETSCII symbol and color (with alpha transparency)
fn terrain_symbol(terrain: Terrain) -> (u8, Color) {
    let a = TERRAIN_ALPHA;
    match terrain {
        Terrain::Grass => (46, Color::Rgba(0, 100, 0, a)),       // 深绿色草地
        Terrain::Forest => (30, Color::Rgba(0, 80, 0, a)),      // 更深绿色森林
        Terrain::Mountain => (30, Color::Rgba(128, 128, 128, a)), // 灰色山脉
        Terrain::Water => (126, Color::Rgba(0, 100, 180, a)),    // 蓝色水域
        Terrain::Desert => (46, Color::Rgba(180, 150, 80, a)),   // 黄褐色沙漠
        Terrain::Swamp => (126, Color::Rgba(60, 100, 80, a)),    // 暗绿色沼泽
    }
}

fn faction_color(id: u8) -> Color {
    match id {
        0 => Color::Red,
        1 => Color::Blue,
        2 => Color::Green,
        3 => Color::Yellow,
        4 => Color::Magenta,
        5 => Color::Cyan,
        _ => Color::White,
    }
}

impl Render for LlmArenaRender {
    type Model = LlmArenaModel;

    fn init(&mut self, context: &mut Context, _model: &mut Self::Model) {
        #[cfg(graphics_mode)]
        context.adapter.get_base().gr.set_use_tui_height(true);

        context.adapter.init(
            SCREEN_WIDTH,
            SCREEN_HEIGHT,
            4.0,
            4.0,
            "LLM Arena".to_string(),
        );
        self.scene.init(context);

        // Position map_gfx sprite at map panel's content area (1, 2) in cell coords
        self.scene
            .get_sprite("map_gfx")
            .set_cell_pos_with_tui(1, 2, true);

        // Position icons_gfx sprite at same location
        self.scene
            .get_sprite("icons_gfx")
            .set_cell_pos_with_tui(1, 2, true);

        // Load resource icons from pix files
        let project_path = &rust_pixel::get_game_config().project_path;
        self.load_icons(project_path);
    }

    fn handle_event(&mut self, _context: &mut Context, _model: &mut Self::Model, _dt: f32) {}

    fn handle_timer(&mut self, _context: &mut Context, _model: &mut Self::Model, _dt: f32) {}

    fn draw(&mut self, ctx: &mut Context, model: &mut Self::Model, _dt: f32) {
        // 1. Update panel canvases with model data
        self.draw_map_legend();
        self.draw_info(model);
        self.draw_log(model);
        self.draw_status(model);

        // 2. Render all TUI panels to scene tui_buffer
        let buffer = self.scene.tui_buffer_mut();
        let _ = self.map_panel.render(buffer, ctx);
        let _ = self.info_panel.render(buffer, ctx);
        let _ = self.log_panel.render(buffer, ctx);
        let _ = self.status_panel.render(buffer, ctx);

        // 3. Draw map PETSCII graphics to sprite (overlays on TUI)
        self.draw_map_gfx(model);

        // 4. Composite TUI + sprites and output to screen
        self.scene.draw(ctx).unwrap();
    }
}
