#![allow(unused_imports)]
use crate::model::{LlmArenaModel, ArenaState, MAP_HEIGHT, MAP_WIDTH, SCREEN_HEIGHT, SCREEN_WIDTH};
use rust_pixel::context::Context;
use rust_pixel::game::Render;
use rust_pixel::render::scene::Scene;
use rust_pixel::render::sprite::Sprite;
use rust_pixel::render::style::Color;

// Layer sizes (in cells)
const MAP_LAYER_W: u16 = 80;
const MAP_LAYER_H: u16 = 35;
const PANEL_W: u16 = 40;

pub struct LlmArenaRender {
    pub scene: Scene,
}

impl Default for LlmArenaRender {
    fn default() -> Self {
        Self::new()
    }
}

impl LlmArenaRender {
    pub fn new() -> Self {
        let mut scene = Scene::new();

        // Create map layer for world rendering
        let map_sprite = Sprite::new(0, 0, MAP_LAYER_W, MAP_LAYER_H);
        scene.add_sprite(map_sprite, "map");

        // Create info panel on the right
        let panel_sprite = Sprite::new(MAP_LAYER_W, 0, PANEL_W, MAP_LAYER_H);
        scene.add_sprite(panel_sprite, "panel");

        // Create status bar at bottom
        let status_sprite = Sprite::new(0, MAP_LAYER_H, SCREEN_WIDTH, 5);
        scene.add_sprite(status_sprite, "status");

        Self { scene }
    }

    fn draw_map(&mut self, model: &LlmArenaModel) {
        let map_sprite = self.scene.get_sprite("map");
        let buf = &mut map_sprite.content;
        buf.reset();

        let world = &model.world;

        // Calculate viewport offset
        let vx = model.viewport.0 - (MAP_LAYER_W as f32 / 2.0);
        let vy = model.viewport.1 - (MAP_LAYER_H as f32 / 2.0);

        // Draw map boundaries
        for x in 0..MAP_LAYER_W {
            for y in 0..MAP_LAYER_H {
                let world_x = vx + x as f32;
                let world_y = vy + y as f32;

                if world_x < 0.0 || world_y < 0.0
                    || world_x >= MAP_WIDTH as f32
                    || world_y >= MAP_HEIGHT as f32
                {
                    // Out of bounds - dark area
                    buf.set_graph_sym(x, y, 0, 102, Color::DarkGray);
                }
            }
        }

        // Draw resource points with different symbols
        for rp in &world.resource_points {
            let sx = (rp.position.0 - vx) as i32;
            let sy = (rp.position.1 - vy) as i32;

            if sx >= 0 && sx < MAP_LAYER_W as i32 && sy >= 0 && sy < MAP_LAYER_H as i32 {
                let (sym, color) = match rp.resource_type {
                    0 => (89, Color::Yellow),   // food - diamond
                    1 => (90, Color::Gray),     // materials - spade
                    2 => (83, Color::Red),      // ammo - heart
                    _ => (88, Color::Green),    // med - club
                };
                buf.set_graph_sym(sx as u16, sy as u16, 0, sym, color);
            }
        }

        // Draw battles as explosion symbols
        for battle in &world.battles {
            let sx = (battle.position.0 - vx) as i32;
            let sy = (battle.position.1 - vy) as i32;

            if sx >= 0 && sx < MAP_LAYER_W as i32 && sy >= 0 && sy < MAP_LAYER_H as i32 {
                // Use star symbol for battle
                buf.set_graph_sym(sx as u16, sy as u16, 0, 42, Color::Yellow);
            }
        }

        // Draw armies as 2x2 blocks using PETSCII block characters
        for army in &world.armies {
            if army.troops == 0 {
                continue;
            }

            let sx = (army.position.0 - vx) as i32;
            let sy = (army.position.1 - vy) as i32;

            if sx >= 0 && sx < MAP_LAYER_W as i32 - 1 && sy >= 0 && sy < MAP_LAYER_H as i32 - 1 {
                let color = match army.faction_id {
                    0 => Color::Red,
                    1 => Color::Blue,
                    2 => Color::Green,
                    3 => Color::Yellow,
                    4 => Color::Magenta,
                    5 => Color::Cyan,
                    _ => Color::White,
                };

                // Draw 2x2 block using filled block character (160)
                for dx in 0..2i32 {
                    for dy in 0..2i32 {
                        let px = (sx + dx) as u16;
                        let py = (sy + dy) as u16;
                        buf.set_graph_sym(px, py, 0, 160, color);
                    }
                }

                // Draw direction indicator if moving
                if army.target.is_some() && army.engaged_lock == 0 {
                    // Use arrow symbol at center
                    let cx = (sx + 1) as u16;
                    let cy = sy as u16;
                    if cx < MAP_LAYER_W && cy < MAP_LAYER_H {
                        buf.set_graph_sym(cx, cy, 0, 62, Color::White); // right arrow
                    }
                }
            }
        }
    }

    fn draw_panel(&mut self, model: &LlmArenaModel) {
        let panel_sprite = self.scene.get_sprite("panel");
        let buf = &mut panel_sprite.content;
        buf.reset();

        let world = &model.world;

        // Title with border
        buf.set_color_str(1, 0, "══════ FACTIONS ══════", Color::White, Color::Reset);

        let mut row = 2u16;
        for faction in &world.factions {
            let status_sym = if faction.is_alive { 81 } else { 87 }; // filled vs empty circle
            let color = match faction.id {
                0 => Color::Red,
                1 => Color::Blue,
                2 => Color::Green,
                3 => Color::Yellow,
                _ => Color::White,
            };

            // Count troops
            let total_troops: u32 = world
                .armies
                .iter()
                .filter(|a| a.faction_id == faction.id)
                .map(|a| a.troops)
                .sum();

            let army_count = world.armies.iter().filter(|a| a.faction_id == faction.id && a.troops > 0).count();

            buf.set_graph_sym(1, row, 0, status_sym, color);
            buf.set_color_str(3, row, &faction.name, color, Color::Reset);
            row += 1;

            let info = format!("  Armies:{} Troops:{}", army_count, total_troops);
            buf.set_color_str(1, row, &info, Color::White, Color::Reset);
            row += 2;
        }

        // Battle info
        row += 1;
        buf.set_color_str(1, row, "═══════ BATTLES ═══════", Color::White, Color::Reset);
        row += 2;

        for (i, battle) in world.battles.iter().enumerate() {
            if i >= 5 {
                buf.set_color_str(1, row, "  ... more battles", Color::Gray, Color::Reset);
                break;
            }
            let info = format!(
                "Battle #{}: {} units",
                battle.id,
                battle.participants.len()
            );
            buf.set_color_str(1, row, &info, Color::Yellow, Color::Reset);
            row += 1;

            // Show casualties
            for p in &battle.participants {
                let faction_name = world.factions.iter()
                    .find(|f| f.id == p.faction_id)
                    .map(|f| f.name.as_str())
                    .unwrap_or("?");
                let detail = format!("  {}: {} (-{})", faction_name, p.current_troops, p.casualties);
                buf.set_color_str(1, row, &detail, Color::Gray, Color::Reset);
                row += 1;
            }
            row += 1;
        }
    }

    fn draw_status(&mut self, model: &LlmArenaModel) {
        let status_sprite = self.scene.get_sprite("status");
        let buf = &mut status_sprite.content;
        buf.reset();

        // Draw border using PETSCII line character
        for x in 0..SCREEN_WIDTH {
            buf.set_graph_sym(x, 0, 0, 64, Color::White); // horizontal line
        }

        // Status info
        let state_str = match model.state {
            ArenaState::Init => "INIT",
            ArenaState::Running => "RUNNING",
            ArenaState::Paused => "PAUSED",
            ArenaState::GameOver => "GAME OVER",
        };

        let status = format!(
            " Tick:{} | Speed:{}x | {} | [Space]:Pause [+/-]:Speed",
            model.world.tick, model.speed, state_str
        );
        buf.set_color_str(0, 1, &status, Color::Cyan, Color::Reset);

        // Controls info
        buf.set_color_str(0, 2, " [Arrows]:Pan viewport", Color::Green, Color::Reset);

        // Winner info
        if model.state == ArenaState::GameOver {
            if let Some(winner_id) = model.world.is_game_over() {
                let winner_name = if winner_id == 255 {
                    "DRAW".to_string()
                } else {
                    model.world.factions.iter()
                        .find(|f| f.id == winner_id)
                        .map(|f| f.name.clone())
                        .unwrap_or("Unknown".to_string())
                };
                buf.set_color_str(0, 3, &format!(" WINNER: {}", winner_name), Color::Yellow, Color::Reset);
            }
        }
    }
}

impl Render for LlmArenaRender {
    type Model = LlmArenaModel;

    fn init(&mut self, context: &mut Context, _model: &mut Self::Model) {
        context.adapter.init(
            SCREEN_WIDTH,
            SCREEN_HEIGHT,
            1.0,
            1.0,
            "LLM Arena".to_string(),
        );
        self.scene.init(context);
    }

    fn handle_event(&mut self, _context: &mut Context, _model: &mut Self::Model, _dt: f32) {}

    fn handle_timer(&mut self, _context: &mut Context, _model: &mut Self::Model, _dt: f32) {}

    fn draw(&mut self, ctx: &mut Context, model: &mut Self::Model, _dt: f32) {
        self.draw_map(model);
        self.draw_panel(model);
        self.draw_status(model);

        self.scene.draw(ctx).unwrap();
    }
}
