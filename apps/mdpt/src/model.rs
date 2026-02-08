use crate::highlight::{CodeHighlighter, HighlightedLine};
use crate::parser::parse_markdown;
use crate::slide::{Presentation, SlideElement};
use crate::slide_builder::{build_slide_page, CODE_FG_HL, CODE_LINE_BG};
use rust_pixel::{
    context::Context,
    event::{Event, KeyCode},
    game::Model,
    get_game_config,
    render::Buffer,
    render::effect::{BufferTransition, TransitionType},
    render::style::Color,
    ui::UIPage,
    util::Rect,
};
use std::collections::HashMap;

pub const MDPTW: u16 = 80;
pub const MDPTH: u16 = 25;

/// Transition state for slide navigation effects
pub struct TransitionState {
    pub active: bool,
    pub from_slide: usize,
    pub to_slide: usize,
    pub progress: f32,
    pub duration: f32,
    pub transition: Box<dyn BufferTransition>,
}

impl TransitionState {
    pub fn new() -> Self {
        Self {
            active: false,
            from_slide: 0,
            to_slide: 0,
            progress: 0.0,
            duration: 0.5,
            transition: TransitionType::WipeLeft.create(),
        }
    }

    pub fn start(&mut self, from: usize, to: usize, transition_type: TransitionType) {
        self.active = true;
        self.from_slide = from;
        self.to_slide = to;
        self.progress = 0.0;
        self.transition = transition_type.create();
    }

    pub fn update(&mut self, dt: f32) -> bool {
        if self.active {
            self.progress += dt / self.duration;
            if self.progress >= 1.0 {
                self.progress = 1.0;
                self.active = false;
                return true;
            }
        }
        false
    }
}

pub struct MdptModel {
    pub presentation: Presentation,
    pub current_slide: usize,
    pub current_step: usize,
    pub md_file: String,
    pub highlighter: CodeHighlighter,
    pub highlight_cache: HashMap<(usize, usize), Vec<HighlightedLine>>,
    /// Current slide UIPage (rebuilt on navigation)
    pub current_page: Option<UIPage>,
    /// Previous slide UIPage (for transitions)
    pub prev_page: Option<UIPage>,
    /// Transition state
    pub transition: TransitionState,
    /// Output buffer for transition blending
    pub output_buffer: Buffer,
    /// Track last rendered state to avoid unnecessary rebuilds
    last_rendered: (usize, usize),
    /// Available transition types
    transition_types: Vec<TransitionType>,
    transition_idx: usize,
}

impl MdptModel {
    pub fn new() -> Self {
        let args: Vec<String> = std::env::args().collect();
        let md_file = if args.len() > 1 {
            args[1].clone()
        } else {
            String::new()
        };

        Self {
            presentation: Presentation::new(),
            current_slide: 0,
            current_step: 0,
            md_file,
            highlighter: CodeHighlighter::new(),
            highlight_cache: HashMap::new(),
            current_page: None,
            prev_page: None,
            transition: TransitionState::new(),
            output_buffer: Buffer::empty(Rect::new(0, 0, MDPTW, MDPTH)),
            last_rendered: (usize::MAX, usize::MAX),
            transition_types: vec![
                TransitionType::SlideLeft,
                TransitionType::WipeLeft,
                TransitionType::Dissolve(42),
                TransitionType::SlideUp,
                TransitionType::WipeDown,
            ],
            transition_idx: 0,
        }
    }

    pub fn total_slides(&self) -> usize {
        self.presentation.slides.len()
    }

    pub fn current_step_count(&self) -> usize {
        if let Some(slide) = self.presentation.slides.get(self.current_slide) {
            slide.step_count()
        } else {
            1
        }
    }

    fn load_presentation(&mut self) {
        let md_path = if self.md_file.is_empty() {
            let project_path = &get_game_config().project_path;
            format!("{}/assets/demo.md", project_path)
        } else {
            self.md_file.clone()
        };

        match std::fs::read_to_string(&md_path) {
            Ok(contents) => {
                self.presentation = parse_markdown(&contents);
                self.current_slide = 0;
                self.current_step = 0;
                self.build_highlight_cache();
                self.rebuild_current_page();
                log::info!(
                    "Loaded presentation: {} slides from {}",
                    self.presentation.slides.len(),
                    md_path
                );
            }
            Err(e) => {
                log::error!("Failed to load {}: {}", md_path, e);
                self.presentation = parse_markdown(&format!(
                    "# mdpt\n\nFailed to load: {}\n\nError: {}",
                    md_path, e
                ));
                self.rebuild_current_page();
            }
        }
    }

    fn build_highlight_cache(&mut self) {
        self.highlight_cache.clear();
        let code_theme = if self.presentation.front_matter.code_theme.is_empty() {
            "base16-ocean.dark"
        } else {
            &self.presentation.front_matter.code_theme
        };

        // Override syntect theme bg/fg with our constants
        let bg = if let Color::Rgba(r, g, b, a) = CODE_LINE_BG {
            Some((r, g, b, a))
        } else {
            None
        };
        let fg = if let Color::Rgba(r, g, b, a) = CODE_FG_HL {
            Some((r, g, b, a))
        } else {
            None
        };
        self.highlighter.override_theme_colors(code_theme, bg, fg);

        for (si, slide) in self.presentation.slides.iter().enumerate() {
            for (ei, elem) in slide.elements.iter().enumerate() {
                if let SlideElement::CodeBlock {
                    language, code, ..
                } = elem
                {
                    let lines = self.highlighter.highlight(code, language, code_theme);
                    self.highlight_cache.insert((si, ei), lines);
                }
            }
        }
    }

    fn rebuild_current_page(&mut self) {
        if let Some(slide) = self.presentation.slides.get(self.current_slide) {
            let page = build_slide_page(
                slide,
                self.current_slide,
                self.current_step,
                &self.highlight_cache,
                &self.presentation.front_matter,
                MDPTW,
                MDPTH,
            );
            self.current_page = Some(page);
            self.last_rendered = (self.current_slide, self.current_step);
        }
    }

    fn next_transition_type(&mut self) -> TransitionType {
        let t = self.transition_types[self.transition_idx].clone();
        self.transition_idx = (self.transition_idx + 1) % self.transition_types.len();
        t
    }

    fn navigate_slide(&mut self, new_slide: usize, new_step: usize) {
        let old_slide = self.current_slide;
        let slide_changed = new_slide != old_slide;

        if slide_changed && !self.transition.active {
            // Determine transition direction before updating state
            let going_forward = new_slide > old_slide;
            let transition_type = if going_forward {
                self.next_transition_type()
            } else {
                // Reverse direction for going back
                match self.next_transition_type() {
                    TransitionType::WipeLeft => TransitionType::WipeRight,
                    TransitionType::WipeDown => TransitionType::WipeUp,
                    TransitionType::SlideLeft => TransitionType::SlideRight,
                    TransitionType::SlideUp => TransitionType::SlideDown,
                    other => other,
                }
            };

            // Save current page as prev for transition
            self.prev_page = self.current_page.take();

            // Now update state and rebuild
            self.current_slide = new_slide;
            self.current_step = new_step;
            self.rebuild_current_page();

            if self.prev_page.is_some() {
                self.transition.start(old_slide, new_slide, transition_type);
            }
        } else if !slide_changed && new_step != self.current_step {
            // Step change within same slide - just rebuild, no transition
            self.current_step = new_step;
            self.rebuild_current_page();
        }
    }

    /// Get the rendered buffer for the current frame.
    pub fn get_rendered_buffer(&mut self) -> &Buffer {
        // Ensure current page is up to date
        if self.last_rendered != (self.current_slide, self.current_step) {
            self.rebuild_current_page();
        }

        if self.transition.active {
            if let (Some(prev), Some(curr)) = (&mut self.prev_page, &mut self.current_page) {
                let _ = prev.render();
                let _ = curr.render();

                self.transition.transition.transition(
                    prev.buffer(),
                    curr.buffer(),
                    &mut self.output_buffer,
                    self.transition.progress,
                );
                return &self.output_buffer;
            }
        }

        // Normal rendering
        if let Some(page) = &mut self.current_page {
            let _ = page.render();
            return page.buffer();
        }

        &self.output_buffer
    }
}

impl Model for MdptModel {
    fn init(&mut self, _context: &mut Context) {
        self.load_presentation();
    }

    fn handle_input(&mut self, context: &mut Context, dt: f32) {
        let es = context.input_events.clone();
        for e in &es {
            if let Event::Key(key) = e {
                if !self.transition.active {
                    match key.code {
                        KeyCode::Right | KeyCode::PageDown | KeyCode::Char(' ') => {
                            let step_count = self.current_step_count();
                            if self.current_step + 1 < step_count {
                                self.navigate_slide(self.current_slide, self.current_step + 1);
                            } else if self.current_slide + 1 < self.total_slides() {
                                self.navigate_slide(self.current_slide + 1, 0);
                            }
                        }
                        KeyCode::Left | KeyCode::PageUp => {
                            if self.current_step > 0 {
                                self.navigate_slide(self.current_slide, self.current_step - 1);
                            } else if self.current_slide > 0 {
                                self.navigate_slide(self.current_slide - 1, 0);
                            }
                        }
                        KeyCode::Home => {
                            self.navigate_slide(0, 0);
                        }
                        KeyCode::End => {
                            if self.total_slides() > 0 {
                                self.navigate_slide(self.total_slides() - 1, 0);
                            }
                        }
                        _ => {}
                    }
                }
            }
        }
        context.input_events.clear();

        // Update transition
        if self.transition.update(dt) {
            // Transition completed
            self.prev_page = None;
        }

        // Update current page animations
        if !self.transition.active {
            if let Some(page) = &mut self.current_page {
                let _ = page.update(dt);
            }
        }
    }

    fn handle_auto(&mut self, _context: &mut Context, _dt: f32) {}
    fn handle_event(&mut self, _context: &mut Context, _dt: f32) {}
    fn handle_timer(&mut self, _context: &mut Context, _dt: f32) {}
}
