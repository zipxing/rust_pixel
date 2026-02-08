use crate::highlight::{CodeHighlighter, HighlightedLine};
use crate::parser::parse_markdown;
use crate::slide::{Presentation, SlideElement};
use crate::slide_builder::{build_slide_page, ImagePlacement, CODE_FG_HL, CODE_LINE_BG};
use rust_pixel::{
    context::Context,
    event::{Event, KeyCode},
    game::Model,
    get_game_config,
    render::Buffer,
    render::effect::{BufferTransition, GpuBlendEffect, GpuTransition, TransitionType},
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
            duration: 0.8,
            transition: TransitionType::WipeLeft.create(),
        }
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
    /// Image placements for sprite-based rendering (SSF/PIX)
    pub image_placements: Vec<ImagePlacement>,
    /// Track last rendered state to avoid unnecessary rebuilds
    last_rendered: (usize, usize),
    /// Available CPU transition types
    transition_types: Vec<TransitionType>,
    /// GPU transition effect state
    pub gpu_effect: GpuBlendEffect,
    /// Whether current transition uses GPU (true) or CPU (false)
    pub use_gpu_transition: bool,
    /// Unified transition index cycling through all CPU+GPU effects
    unified_transition_idx: usize,
}

impl MdptModel {
    pub fn new() -> Self {
        log::info!("[mdpt] MdptModel::new: start");
        #[cfg(not(target_arch = "wasm32"))]
        let md_file = {
            let args: Vec<String> = std::env::args().collect();
            if args.len() > 1 { args[1].clone() } else { String::new() }
        };
        #[cfg(target_arch = "wasm32")]
        let md_file = String::new();

        log::info!("[mdpt] MdptModel::new: creating highlighter...");
        let highlighter = CodeHighlighter::new();
        log::info!("[mdpt] MdptModel::new: highlighter created, building Self...");

        Self {
            presentation: Presentation::new(),
            current_slide: 0,
            current_step: 0,
            md_file,
            highlighter,
            highlight_cache: HashMap::new(),
            image_placements: Vec::new(),
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
            gpu_effect: GpuBlendEffect::default(),
            use_gpu_transition: true,
            unified_transition_idx: 0,
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
        log::info!("[mdpt] load_presentation: start");

        // WASM: load md from JS via wasm_set_app_data (use ?data=assets/demo.md URL param)
        #[cfg(target_arch = "wasm32")]
        let contents = rust_pixel::get_wasm_app_data()
            .unwrap_or("# mdpt\n\nOpen with `?data=assets/demo.md` URL parameter to load a presentation")
            .to_string();

        #[cfg(not(target_arch = "wasm32"))]
        let contents = {
            let md_path = if self.md_file.is_empty() {
                let project_path = &get_game_config().project_path;
                format!("{}/assets/demo.md", project_path)
            } else {
                self.md_file.clone()
            };

            match std::fs::read_to_string(&md_path) {
                Ok(c) => c,
                Err(e) => {
                    log::error!("Failed to load {}: {}", md_path, e);
                    format!("# mdpt\n\nFailed to load: {}\n\nError: {}", md_path, e)
                }
            }
        };

        log::info!("[mdpt] load_presentation: contents loaded, len={}", contents.len());

        self.presentation = parse_markdown(&contents);
        log::info!("[mdpt] load_presentation: parse_markdown done, {} slides", self.presentation.slides.len());

        self.current_slide = 0;
        self.current_step = 0;

        log::info!("[mdpt] load_presentation: calling build_highlight_cache...");
        self.build_highlight_cache();
        log::info!("[mdpt] load_presentation: build_highlight_cache done");

        log::info!("[mdpt] load_presentation: calling rebuild_current_page...");
        self.rebuild_current_page();
        log::info!("[mdpt] load_presentation: rebuild_current_page done");

        log::info!(
            "Loaded presentation: {} slides",
            self.presentation.slides.len(),
        );
    }

    fn build_highlight_cache(&mut self) {
        self.highlight_cache.clear();
        let code_theme = if self.presentation.front_matter.code_theme.is_empty() {
            "base16-ocean.dark"
        } else {
            &self.presentation.front_matter.code_theme
        };
        log::info!("[mdpt] build_highlight_cache: theme={}", code_theme);

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
        log::info!("[mdpt] build_highlight_cache: theme colors overridden");

        let mut code_block_count = 0;
        for (si, slide) in self.presentation.slides.iter().enumerate() {
            for (ei, elem) in slide.elements.iter().enumerate() {
                if let SlideElement::CodeBlock {
                    language, code, ..
                } = elem
                {
                    log::info!("[mdpt] build_highlight_cache: highlighting slide={} elem={} lang={}", si, ei, language);
                    let lines = self.highlighter.highlight(code, language, code_theme);
                    self.highlight_cache.insert((si, ei), lines);
                    code_block_count += 1;
                }
            }
        }
        log::info!("[mdpt] build_highlight_cache: done, {} code blocks highlighted", code_block_count);
    }

    fn rebuild_current_page(&mut self) {
        log::info!("[mdpt] rebuild_current_page: slide={} step={}", self.current_slide, self.current_step);
        if let Some(slide) = self.presentation.slides.get(self.current_slide) {
            log::info!("[mdpt] rebuild_current_page: building slide page with {} elements", slide.elements.len());
            let (page, images) = build_slide_page(
                slide,
                self.current_slide,
                self.current_step,
                &self.highlight_cache,
                &self.presentation.front_matter,
                MDPTW,
                MDPTH,
            );
            log::info!("[mdpt] rebuild_current_page: build_slide_page done, {} images", images.len());
            self.current_page = Some(page);
            self.image_placements = images;
            self.last_rendered = (self.current_slide, self.current_step);
            log::info!("[mdpt] rebuild_current_page: done");
        }
    }

    /// Pick next transition from the unified pool (5 CPU + 7 GPU).
    /// Returns (cpu_transition_type, is_gpu, gpu_transition).
    fn next_unified_transition(&mut self, going_forward: bool) {
        let cpu_count = self.transition_types.len(); // 5
        let gpu_count = GpuTransition::count();      // 7
        let total = cpu_count + gpu_count;            // 12

        // Check if front_matter locks to a specific GPU transition
        let fm_trans = &self.presentation.front_matter.transition;
        let fixed_gpu = match fm_trans.as_str() {
            "squares" => Some(GpuTransition::Squares),
            "heart" => Some(GpuTransition::Heart),
            "noise" => Some(GpuTransition::Noise),
            "rotate" => Some(GpuTransition::RotateZoom),
            "bounce" => Some(GpuTransition::Bounce),
            "dissolve" => Some(GpuTransition::Dispersion),
            "ripple" => Some(GpuTransition::Ripple),
            _ => None,
        };
        if let Some(gpu_trans) = fixed_gpu {
            self.use_gpu_transition = true;
            self.transition.duration = 1.0;
            self.gpu_effect = GpuBlendEffect::new(gpu_trans, 0.0);
            return;
        }

        let idx = self.unified_transition_idx;
        self.unified_transition_idx = (idx + 1) % total;

        if idx < cpu_count {
            // CPU transition
            self.use_gpu_transition = false;
            let mut t = self.transition_types[idx].clone();
            if !going_forward {
                t = match t {
                    TransitionType::WipeLeft => TransitionType::WipeRight,
                    TransitionType::WipeDown => TransitionType::WipeUp,
                    TransitionType::SlideLeft => TransitionType::SlideRight,
                    TransitionType::SlideUp => TransitionType::SlideDown,
                    other => other,
                };
            }
            self.transition.transition = t.create();
            self.transition.duration = 0.5;
            self.gpu_effect = GpuBlendEffect::default();
        } else {
            // GPU transition
            self.use_gpu_transition = true;
            self.transition.duration = 1.0;
            let gpu_idx = idx - cpu_count;
            let gpu_trans = GpuTransition::from_index(gpu_idx);
            self.gpu_effect = GpuBlendEffect::new(gpu_trans, 0.0);
        }
    }

    fn navigate_slide(&mut self, new_slide: usize, new_step: usize) {
        let old_slide = self.current_slide;
        let slide_changed = new_slide != old_slide;

        if slide_changed && !self.transition.active {
            let going_forward = new_slide > old_slide;

            // Pick next transition from unified CPU+GPU pool
            self.next_unified_transition(going_forward);

            // Save current page as prev for transition
            self.prev_page = self.current_page.take();

            // Now update state and rebuild
            self.current_slide = new_slide;
            self.current_step = new_step;
            self.rebuild_current_page();

            if self.prev_page.is_some() {
                self.transition.active = true;
                self.transition.from_slide = old_slide;
                self.transition.to_slide = new_slide;
                self.transition.progress = 0.0;
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
        log::info!("[mdpt] Model::init called, calling load_presentation...");
        self.load_presentation();
        log::info!("[mdpt] Model::init done");
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
        // Sync GPU effect progress with transition state
        self.gpu_effect.progress = self.transition.progress;

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
