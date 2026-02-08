// RustPixel
// copyright zipxing@hotmail.com 2022～2025

//! Defines styles such as color, bold or italics.
//! Only foreground color is supported in Graphics mode, as background color is used for texture.

#[cfg(not(any(graphics_backend, mobile, wasm)))]
use crate::render::image::io_error;
use bitflags::bitflags;
#[cfg(not(any(graphics_backend, mobile, wasm)))]
use crossterm::{
    queue,
    style::{Attribute as CAttribute, SetAttribute},
};
use serde::{Deserialize, Serialize};

mod color;
pub use color::*;

mod color_pro;
pub use color_pro::*;

bitflags! {
    #[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
    pub struct Modifier: u16 {
        const BOLD              = 0b0000_0000_0001;
        const DIM               = 0b0000_0000_0010;
        const ITALIC            = 0b0000_0000_0100;
        const UNDERLINED        = 0b0000_0000_1000;
        const SLOW_BLINK        = 0b0000_0001_0000;
        const RAPID_BLINK       = 0b0000_0010_0000;
        const REVERSED          = 0b0000_0100_0000;
        const HIDDEN            = 0b0000_1000_0000;
        const CROSSED_OUT       = 0b0001_0000_0000;
        /// Rendering hint: keep slot width fixed even when scale_x >= 1.0.
        /// Character scales visually but doesn't expand its grid slot.
        const FIXED_SLOT        = 0b0010_0000_0000;
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Style {
    pub fg: Option<Color>,
    pub bg: Option<Color>,
    pub add_modifier: Modifier,
    pub sub_modifier: Modifier,
    /// Per-cell X scale (None = don't change existing cell scale)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub scale_x: Option<f32>,
    /// Per-cell Y scale (None = don't change existing cell scale)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub scale_y: Option<f32>,
}

impl Default for Style {
    fn default() -> Style {
        Style {
            fg: None,
            bg: None,
            add_modifier: Modifier::empty(),
            sub_modifier: Modifier::empty(),
            scale_x: None,
            scale_y: None,
        }
    }
}

impl Style {
    /// Returns a `Style` resetting all properties.
    pub fn reset() -> Style {
        Style {
            fg: Some(Color::Reset),
            bg: Some(Color::Reset),
            add_modifier: Modifier::empty(),
            sub_modifier: Modifier::all(),
            scale_x: Some(1.0),
            scale_y: Some(1.0),
        }
    }

    pub fn fg(mut self, color: Color) -> Style {
        self.fg = Some(color);
        self
    }

    pub fn bg(mut self, color: Color) -> Style {
        self.bg = Some(color);
        self
    }

    pub fn add_modifier(mut self, modifier: Modifier) -> Style {
        self.sub_modifier.remove(modifier);
        self.add_modifier.insert(modifier);
        self
    }

    pub fn remove_modifier(mut self, modifier: Modifier) -> Style {
        self.add_modifier.remove(modifier);
        self.sub_modifier.insert(modifier);
        self
    }

    pub fn scale(mut self, sx: f32, sy: f32) -> Style {
        self.scale_x = Some(sx);
        self.scale_y = Some(sy);
        self
    }

    pub fn scale_uniform(mut self, s: f32) -> Style {
        self.scale_x = Some(s);
        self.scale_y = Some(s);
        self
    }

    /// Scale visually but keep horizontal slot width fixed (no spacing change).
    pub fn scale_fixed_slot(self, sx: f32, sy: f32) -> Style {
        self.scale(sx, sy).add_modifier(Modifier::FIXED_SLOT)
    }

    pub fn patch(mut self, other: Style) -> Style {
        self.fg = other.fg.or(self.fg);
        self.bg = other.bg.or(self.bg);

        self.add_modifier.remove(other.sub_modifier);
        self.add_modifier.insert(other.add_modifier);
        self.sub_modifier.remove(other.add_modifier);
        self.sub_modifier.insert(other.sub_modifier);

        self.scale_x = other.scale_x.or(self.scale_x);
        self.scale_y = other.scale_y.or(self.scale_y);

        self
    }
}

#[derive(Debug)]
#[cfg(not(any(graphics_backend, mobile, wasm)))]
pub struct ModifierDiff {
    pub from: Modifier,
    pub to: Modifier,
}

#[cfg(not(any(graphics_backend, mobile, wasm)))]
impl ModifierDiff {
    pub fn queue<W>(&self, mut w: W) -> std::io::Result<()>
    where
        W: std::io::Write,
    {
        //use crossterm::Attribute;
        let removed = self.from - self.to;
        if removed.contains(Modifier::REVERSED) {
            io_error(queue!(w, SetAttribute(CAttribute::NoReverse)))?;
        }
        if removed.contains(Modifier::BOLD) {
            io_error(queue!(w, SetAttribute(CAttribute::NormalIntensity)))?;
            if self.to.contains(Modifier::DIM) {
                io_error(queue!(w, SetAttribute(CAttribute::Dim)))?;
            }
        }
        if removed.contains(Modifier::ITALIC) {
            io_error(queue!(w, SetAttribute(CAttribute::NoItalic)))?;
        }
        if removed.contains(Modifier::UNDERLINED) {
            io_error(queue!(w, SetAttribute(CAttribute::NoUnderline)))?;
        }
        if removed.contains(Modifier::DIM) {
            io_error(queue!(w, SetAttribute(CAttribute::NormalIntensity)))?;
        }
        if removed.contains(Modifier::CROSSED_OUT) {
            io_error(queue!(w, SetAttribute(CAttribute::NotCrossedOut)))?;
        }
        if removed.contains(Modifier::SLOW_BLINK) || removed.contains(Modifier::RAPID_BLINK) {
            io_error(queue!(w, SetAttribute(CAttribute::NoBlink)))?;
        }

        let added = self.to - self.from;
        if added.contains(Modifier::REVERSED) {
            io_error(queue!(w, SetAttribute(CAttribute::Reverse)))?;
        }
        if added.contains(Modifier::BOLD) {
            io_error(queue!(w, SetAttribute(CAttribute::Bold)))?;
        }
        if added.contains(Modifier::ITALIC) {
            io_error(queue!(w, SetAttribute(CAttribute::Italic)))?;
        }
        if added.contains(Modifier::UNDERLINED) {
            io_error(queue!(w, SetAttribute(CAttribute::Underlined)))?;
        }
        if added.contains(Modifier::DIM) {
            io_error(queue!(w, SetAttribute(CAttribute::Dim)))?;
        }
        if added.contains(Modifier::CROSSED_OUT) {
            io_error(queue!(w, SetAttribute(CAttribute::CrossedOut)))?;
        }
        if added.contains(Modifier::SLOW_BLINK) {
            io_error(queue!(w, SetAttribute(CAttribute::SlowBlink)))?;
        }
        if added.contains(Modifier::RAPID_BLINK) {
            io_error(queue!(w, SetAttribute(CAttribute::RapidBlink)))?;
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn styles() -> Vec<Style> {
        vec![
            Style::default(),
            Style::default().fg(Color::Yellow),
            Style::default().bg(Color::Yellow),
            Style::default().add_modifier(Modifier::BOLD),
            Style::default().remove_modifier(Modifier::BOLD),
            Style::default().add_modifier(Modifier::ITALIC),
            Style::default().remove_modifier(Modifier::ITALIC),
            Style::default().add_modifier(Modifier::ITALIC | Modifier::BOLD),
            Style::default().remove_modifier(Modifier::ITALIC | Modifier::BOLD),
        ]
    }

    #[test]
    fn combined_patch_gives_same_result_as_individual_patch() {
        let styles = styles();
        for &a in &styles {
            for &b in &styles {
                for &c in &styles {
                    for &d in &styles {
                        let combined = a.patch(b.patch(c.patch(d)));

                        assert_eq!(
                            Style::default().patch(a).patch(b).patch(c).patch(d),
                            Style::default().patch(combined)
                        );
                    }
                }
            }
        }
    }
}
