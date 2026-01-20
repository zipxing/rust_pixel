//! PixelGameContext - GameContext implementation for rust_pixel engine
//!
//! This module provides the concrete implementation of GameContext trait
//! that bridges BASIC scripts with rust_pixel's rendering system.
//!
//! # Design
//!
//! Instead of directly holding a reference to Panel (which causes lifetime issues),
//! this implementation collects draw commands that can be applied to a Panel later.

use std::collections::HashMap;
use crate::game_context::GameContext;

/// A single draw command that can be applied to a Panel
#[derive(Debug, Clone)]
pub enum DrawCommand {
    /// Draw a character at position with colors
    Plot { x: i32, y: i32, ch: char, fg: u8, bg: u8 },
    /// Clear the screen
    Clear,
}

/// Sprite data managed by BASIC scripts
#[derive(Debug, Clone)]
pub struct SpriteData {
    pub id: u32,
    pub x: i32,
    pub y: i32,
    pub ch: char,
    pub fg: u8,
    pub bg: u8,
    pub hidden: bool,
}

/// GameContext implementation that collects draw commands
///
/// # Usage
///
/// ```ignore
/// // Create context
/// let mut ctx = PixelGameContext::new();
///
/// // Execute BASIC code (fills ctx with draw commands)
/// bridge.call_draw(&mut ctx)?;
///
/// // Apply commands to Panel
/// for cmd in ctx.drain_commands() {
///     match cmd {
///         DrawCommand::Plot { x, y, ch, fg, bg } => {
///             sprite.set_color_str(x, y, ch, fg, bg);
///         }
///         DrawCommand::Clear => { ... }
///     }
/// }
/// ```
pub struct PixelGameContext {
    /// Collected draw commands
    commands: Vec<DrawCommand>,

    /// Sprite management
    sprites: HashMap<u32, SpriteData>,

    /// Input state
    last_key: u32,
    key_states: HashMap<String, bool>,
    mouse_x: i32,
    mouse_y: i32,
    mouse_buttons: u8,
}

impl Default for PixelGameContext {
    fn default() -> Self {
        Self::new()
    }
}

impl PixelGameContext {
    /// Create a new PixelGameContext
    pub fn new() -> Self {
        Self {
            commands: Vec::new(),
            sprites: HashMap::new(),
            last_key: 0,
            key_states: HashMap::new(),
            mouse_x: 0,
            mouse_y: 0,
            mouse_buttons: 0,
        }
    }

    /// Drain all collected draw commands
    ///
    /// Returns an iterator over all commands and clears the internal buffer.
    pub fn drain_commands(&mut self) -> impl Iterator<Item = DrawCommand> + '_ {
        self.commands.drain(..)
    }

    /// Get all collected draw commands (without consuming)
    pub fn commands(&self) -> &[DrawCommand] {
        &self.commands
    }

    /// Clear all collected commands
    pub fn clear_commands(&mut self) {
        self.commands.clear();
    }

    /// Get all sprites (for rendering)
    pub fn sprites(&self) -> &HashMap<u32, SpriteData> {
        &self.sprites
    }

    /// Get a mutable reference to sprites
    pub fn sprites_mut(&mut self) -> &mut HashMap<u32, SpriteData> {
        &mut self.sprites
    }

    // ========== Input State Management ==========

    /// Update the last pressed key (called by game engine)
    pub fn update_key(&mut self, key: u32) {
        self.last_key = key;
    }

    /// Update key state (called by game engine)
    pub fn set_key_state(&mut self, key: String, pressed: bool) {
        self.key_states.insert(key, pressed);
    }

    /// Update mouse position (called by game engine)
    pub fn update_mouse(&mut self, x: i32, y: i32, buttons: u8) {
        self.mouse_x = x;
        self.mouse_y = y;
        self.mouse_buttons = buttons;
    }

    /// Clear all key states
    pub fn clear_key_states(&mut self) {
        self.key_states.clear();
        self.last_key = 0;
    }
}

impl GameContext for PixelGameContext {
    // ========== Graphics Methods ==========

    fn plot(&mut self, x: i32, y: i32, ch: char, fg: u8, bg: u8) {
        self.commands.push(DrawCommand::Plot { x, y, ch, fg, bg });
    }

    fn cls(&mut self) {
        self.commands.push(DrawCommand::Clear);
    }

    fn line(&mut self, x0: i32, y0: i32, x1: i32, y1: i32, ch: char) {
        // Bresenham's line algorithm
        let dx = (x1 - x0).abs();
        let dy = (y1 - y0).abs();
        let sx = if x0 < x1 { 1 } else { -1 };
        let sy = if y0 < y1 { 1 } else { -1 };
        let mut err = dx - dy;

        let mut x = x0;
        let mut y = y0;

        loop {
            self.plot(x, y, ch, 15, 0); // White on black

            if x == x1 && y == y1 {
                break;
            }

            let e2 = 2 * err;
            if e2 > -dy {
                err -= dy;
                x += sx;
            }
            if e2 < dx {
                err += dx;
                y += sy;
            }
        }
    }

    fn box_draw(&mut self, x: i32, y: i32, w: i32, h: i32, style: u8) {
        // Draw box using line characters
        let (tl, tr, bl, br, h_line, v_line) = match style {
            1 => ('┌', '┐', '└', '┘', '─', '│'), // Single line
            2 => ('╔', '╗', '╚', '╝', '═', '║'), // Double line
            _ => ('+', '+', '+', '+', '-', '|'), // ASCII
        };

        // Top and bottom edges
        for i in 0..w {
            let ch = if i == 0 { tl } else if i == w - 1 { tr } else { h_line };
            self.plot(x + i, y, ch, 15, 0);

            let ch = if i == 0 { bl } else if i == w - 1 { br } else { h_line };
            self.plot(x + i, y + h - 1, ch, 15, 0);
        }

        // Left and right edges
        for i in 1..h - 1 {
            self.plot(x, y + i, v_line, 15, 0);
            self.plot(x + w - 1, y + i, v_line, 15, 0);
        }
    }

    fn circle(&mut self, cx: i32, cy: i32, r: i32, ch: char) {
        // Midpoint circle algorithm
        let mut x = 0;
        let mut y = r;
        let mut d = 1 - r;

        let mut plot_points = |cx: i32, cy: i32, x: i32, y: i32| {
            self.plot(cx + x, cy + y, ch, 15, 0);
            self.plot(cx - x, cy + y, ch, 15, 0);
            self.plot(cx + x, cy - y, ch, 15, 0);
            self.plot(cx - x, cy - y, ch, 15, 0);
            self.plot(cx + y, cy + x, ch, 15, 0);
            self.plot(cx - y, cy + x, ch, 15, 0);
            self.plot(cx + y, cy - x, ch, 15, 0);
            self.plot(cx - y, cy - x, ch, 15, 0);
        };

        plot_points(cx, cy, x, y);

        while x < y {
            x += 1;
            if d < 0 {
                d += 2 * x + 1;
            } else {
                y -= 1;
                d += 2 * (x - y) + 1;
            }
            plot_points(cx, cy, x, y);
        }
    }

    // ========== Sprite Methods ==========

    fn sprite_create(&mut self, id: u32, x: i32, y: i32, ch: char) {
        if let Some(sprite_data) = self.sprites.get_mut(&id) {
            sprite_data.x = x;
            sprite_data.y = y;
            sprite_data.ch = ch;
        } else {
            self.sprites.insert(id, SpriteData {
                id,
                x,
                y,
                ch,
                fg: 15,
                bg: 0,
                hidden: false,
            });
        }
    }

    fn sprite_move(&mut self, id: u32, dx: i32, dy: i32) {
        if let Some(sprite_data) = self.sprites.get_mut(&id) {
            sprite_data.x += dx;
            sprite_data.y += dy;
        }
    }

    fn sprite_pos(&mut self, id: u32, x: i32, y: i32) {
        if let Some(sprite_data) = self.sprites.get_mut(&id) {
            sprite_data.x = x;
            sprite_data.y = y;
        }
    }

    fn sprite_hide(&mut self, id: u32, hidden: bool) {
        if let Some(sprite_data) = self.sprites.get_mut(&id) {
            sprite_data.hidden = hidden;
        }
    }

    fn sprite_color(&mut self, id: u32, fg: u8, bg: u8) {
        if let Some(sprite_data) = self.sprites.get_mut(&id) {
            sprite_data.fg = fg;
            sprite_data.bg = bg;
        }
    }

    fn sprite_x(&self, id: u32) -> Option<i32> {
        self.sprites.get(&id).map(|s| s.x)
    }

    fn sprite_y(&self, id: u32) -> Option<i32> {
        self.sprites.get(&id).map(|s| s.y)
    }

    fn sprite_hit(&self, id1: u32, id2: u32) -> bool {
        if let (Some(s1), Some(s2)) = (self.sprites.get(&id1), self.sprites.get(&id2)) {
            s1.x == s2.x && s1.y == s2.y && !s1.hidden && !s2.hidden
        } else {
            false
        }
    }

    // ========== Input Methods ==========

    fn inkey(&self) -> u32 {
        self.last_key
    }

    fn key(&self, key_name: &str) -> bool {
        self.key_states.get(key_name).copied().unwrap_or(false)
    }

    fn mouse_x(&self) -> i32 {
        self.mouse_x
    }

    fn mouse_y(&self) -> i32 {
        self.mouse_y
    }

    fn mouse_button(&self) -> u8 {
        self.mouse_buttons
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sprite_data_creation() {
        let sprite = SpriteData {
            id: 1,
            x: 10,
            y: 20,
            ch: '@',
            fg: 15,
            bg: 0,
            hidden: false,
        };

        assert_eq!(sprite.id, 1);
        assert_eq!(sprite.x, 10);
        assert_eq!(sprite.y, 20);
        assert_eq!(sprite.ch, '@');
    }

    #[test]
    fn test_input_state_management() {
        let mut ctx = PixelGameContext::new();

        ctx.update_key(65); // 'A'
        assert_eq!(ctx.inkey(), 65);

        ctx.set_key_state("SPACE".to_string(), true);
        assert!(ctx.key("SPACE"));
        assert!(!ctx.key("ENTER"));

        ctx.update_mouse(100, 200, 1);
        assert_eq!(ctx.mouse_x(), 100);
        assert_eq!(ctx.mouse_y(), 200);
        assert_eq!(ctx.mouse_button(), 1);
    }

    #[test]
    fn test_plot_collects_commands() {
        let mut ctx = PixelGameContext::new();

        ctx.plot(10, 20, '@', 15, 0);
        ctx.plot(5, 5, '#', 10, 1);

        assert_eq!(ctx.commands().len(), 2);

        match &ctx.commands()[0] {
            DrawCommand::Plot { x, y, ch, fg, bg } => {
                assert_eq!(*x, 10);
                assert_eq!(*y, 20);
                assert_eq!(*ch, '@');
                assert_eq!(*fg, 15);
                assert_eq!(*bg, 0);
            }
            _ => panic!("Expected Plot command"),
        }
    }

    #[test]
    fn test_cls_collects_clear_command() {
        let mut ctx = PixelGameContext::new();

        ctx.plot(10, 20, '@', 15, 0);
        ctx.cls();

        assert_eq!(ctx.commands().len(), 2);
        assert!(matches!(ctx.commands()[1], DrawCommand::Clear));
    }

    #[test]
    fn test_drain_commands() {
        let mut ctx = PixelGameContext::new();

        ctx.plot(10, 20, '@', 15, 0);
        ctx.cls();

        let commands: Vec<_> = ctx.drain_commands().collect();
        assert_eq!(commands.len(), 2);
        assert!(ctx.commands().is_empty());
    }

    #[test]
    fn test_sprite_operations() {
        let mut ctx = PixelGameContext::new();

        // Create sprite
        ctx.sprite_create(1, 10, 20, '@');
        assert_eq!(ctx.sprite_x(1), Some(10));
        assert_eq!(ctx.sprite_y(1), Some(20));

        // Move sprite
        ctx.sprite_move(1, 5, -3);
        assert_eq!(ctx.sprite_x(1), Some(15));
        assert_eq!(ctx.sprite_y(1), Some(17));

        // Set absolute position
        ctx.sprite_pos(1, 100, 50);
        assert_eq!(ctx.sprite_x(1), Some(100));
        assert_eq!(ctx.sprite_y(1), Some(50));

        // Non-existent sprite
        assert_eq!(ctx.sprite_x(999), None);
        assert_eq!(ctx.sprite_y(999), None);
    }

    #[test]
    fn test_sprite_hit_detection() {
        let mut ctx = PixelGameContext::new();

        ctx.sprite_create(1, 10, 20, '@');
        ctx.sprite_create(2, 10, 20, '#');
        ctx.sprite_create(3, 15, 25, '*');

        // Same position - hit
        assert!(ctx.sprite_hit(1, 2));

        // Different position - no hit
        assert!(!ctx.sprite_hit(1, 3));

        // Hidden sprite - no hit
        ctx.sprite_hide(1, true);
        assert!(!ctx.sprite_hit(1, 2));
    }
}
