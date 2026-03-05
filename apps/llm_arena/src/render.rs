#![allow(unused_imports)]
use crate::model::{ArenaState, LlmArenaModel, Terrain, MAP_HEIGHT, MAP_WIDTH, SCREEN_HEIGHT, SCREEN_WIDTH};
use rust_pixel::context::Context;
use rust_pixel::game::Render;
use rust_pixel::render::scene::Scene;
use rust_pixel::render::sprite::Sprite;
use rust_pixel::render::style::Color;
use rust_pixel::ui::{BorderStyle, Panel, Widget};
use rust_pixel::util::Rect;

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
const HIGHLIGHT_COLOR: Color = Color::Yellow;
const LOG_COLOR: Color = Color::Indexed(250);

pub struct LlmArenaRender {
    pub scene: Scene,
    pub map_panel: Panel,
    pub info_panel: Panel,
    pub log_panel: Panel,
    pub status_panel: Panel,
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
        }
    }

    /// Draw map PETSCII graphics to sprite (overlay on map panel)
    /// Uses map_scale for zoom: higher scale = more zoom (smaller visible area)
    fn draw_map_gfx(&mut self, model: &LlmArenaModel) {
        let gfx = self.scene.get_sprite("map_gfx");
        let buf = &mut gfx.content;
        buf.reset();

        let world = &model.world;
        let scale = model.map_scale;

        // Visible world area (smaller when zoomed in)
        let visible_w = MAP_GFX_W as f32 / scale;
        let visible_h = MAP_GFX_H as f32 / scale;

        // World coords of top-left corner of viewport
        let world_x0 = model.viewport.0 - visible_w / 2.0;
        let world_y0 = model.viewport.1 - visible_h / 2.0;

        // Helper: convert world coords to sprite coords
        let to_sprite = |wx: f32, wy: f32| -> (i32, i32) {
            let sx = ((wx - world_x0) * scale) as i32;
            let sy = ((wy - world_y0) * scale) as i32;
            (sx, sy)
        };

        // Helper: check if sprite coords are in bounds
        let in_bounds = |sx: i32, sy: i32| -> bool {
            sx >= 0 && sx < MAP_GFX_W as i32 && sy >= 0 && sy < MAP_GFX_H as i32
        };

        // Draw terrain and out-of-bounds areas
        for x in 0..MAP_GFX_W {
            for y in 0..MAP_GFX_H {
                let wx = world_x0 + x as f32 / scale;
                let wy = world_y0 + y as f32 / scale;
                if wx < 0.0 || wy < 0.0 || wx >= MAP_WIDTH as f32 || wy >= MAP_HEIGHT as f32 {
                    buf.set_graph_sym(x, y, 0, 102, Color::DarkGray);
                } else {
                    // Get terrain at this world position
                    let tx = wx as usize;
                    let ty = wy as usize;
                    if ty < world.terrain.len() && tx < world.terrain[ty].len() {
                        let (sym, color) = terrain_symbol(world.terrain[ty][tx]);
                        buf.set_graph_sym(x, y, 0, sym, color);
                    }
                }
            }
        }

        // Resource points (scaled size)
        let rp_size = (scale.ceil() as i32).max(1);
        for rp in &world.resource_points {
            let (sx, sy) = to_sprite(rp.position.0, rp.position.1);
            let (sym, color) = match rp.resource_type {
                0 => (89, Color::Yellow),   // Food
                1 => (90, Color::Cyan),     // Gems
                2 => (83, Color::Red),      // Ammo
                _ => (88, Color::Green),
            };
            // Draw scaled block
            for dx in 0..rp_size {
                for dy in 0..rp_size {
                    if in_bounds(sx + dx, sy + dy) {
                        buf.set_graph_sym((sx + dx) as u16, (sy + dy) as u16, 0, sym, color);
                    }
                }
            }
        }

        // Battles (scaled)
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

/// Map terrain type to PETSCII symbol and color
fn terrain_symbol(terrain: Terrain) -> (u8, Color) {
    match terrain {
        Terrain::Grass => {
            // Use various grass-like symbols: dots, commas
            (46, Color::Indexed(22))  // '.' dark green
        }
        Terrain::Forest => {
            // Tree symbol (spade or club)
            (30, Color::Indexed(28))  // dark green tree
        }
        Terrain::Mountain => {
            // Mountain/triangle symbol
            (30, Color::Indexed(240))  // gray mountain
        }
        Terrain::Water => {
            // Wave-like symbol
            (126, Color::Indexed(21))  // blue water ~
        }
        Terrain::Desert => {
            // Sandy dots
            (46, Color::Indexed(220))  // yellow sand
        }
        Terrain::Swamp => {
            // Swampy pattern
            (126, Color::Indexed(23))  // dark cyan swamp
        }
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
