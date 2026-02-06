use rust_pixel::game::Model;
use rust_pixel::context::Context;
use rust_pixel::ui::*;
use rust_pixel::ui::layout::Alignment;
use rust_pixel::render::style::{Color, Modifier, Style};
use rust_pixel::render::effect::{BufferTransition, TransitionType};
use rust_pixel::render::Buffer;
use rust_pixel::util::Rect;
use rust_pixel::event::{Event, KeyCode};
use log::info;

pub const UI_DEMO_WIDTH: usize = 80;
pub const UI_DEMO_HEIGHT: usize = 30;

/// Transition state for multi-page navigation
pub struct TransitionState {
    pub active: bool,
    pub from_page: usize,
    pub to_page: usize,
    pub progress: f32,
    pub duration: f32,
    pub transition: Box<dyn BufferTransition>,
}

impl TransitionState {
    pub fn new() -> Self {
        Self {
            active: false,
            from_page: 0,
            to_page: 0,
            progress: 0.0,
            duration: 0.35,
            transition: TransitionType::WipeLeft.create(),
        }
    }

    pub fn start(&mut self, from: usize, to: usize, transition_type: TransitionType) {
        self.active = true;
        self.from_page = from;
        self.to_page = to;
        self.progress = 0.0;
        self.transition = transition_type.create();
    }

    pub fn update(&mut self, dt: f32) -> bool {
        if self.active {
            self.progress += dt / self.duration;
            if self.progress >= 1.0 {
                self.progress = 1.0;
                self.active = false;
                return true; // Transition completed
            }
        }
        false
    }
}

// Model - handles UI state and logic with multi-page support
pub struct UiDemoModel {
    pub pages: Vec<UIPage>,
    pub current_page: usize,
    pub transition: TransitionState,
    pub output_buffer: Buffer,
    pub transition_types: Vec<TransitionType>,
    pub current_transition_idx: usize,
}

impl UiDemoModel {
    pub fn new() -> Self {
        let width = UI_DEMO_WIDTH as u16;
        let height = UI_DEMO_HEIGHT as u16;

        // Create multiple pages
        let mut pages = Vec::new();

        // Page 1: Basic widgets demo
        let mut page1 = UIPage::new(width, height);
        page1.set_root_widget(Box::new(create_page1_interface()));
        page1.start();

        // Page 2: Animation and effects demo
        let mut page2 = UIPage::new(width, height);
        page2.set_root_widget(Box::new(create_page2_interface()));
        page2.start();

        // Page 3: Advanced components demo
        let mut page3 = UIPage::new(width, height);
        page3.set_root_widget(Box::new(create_page3_interface()));
        page3.start();

        pages.push(page1);
        pages.push(page2);
        pages.push(page3);

        // Available transition types
        let transition_types = vec![
            TransitionType::WipeLeft,
            TransitionType::WipeRight,
            TransitionType::SlideLeft,
            TransitionType::SlideRight,
            TransitionType::Dissolve(42),
            TransitionType::BlindsHorizontal(4),
            TransitionType::BlindsVertical(6),
            TransitionType::Checkerboard(4),
            TransitionType::Typewriter,
        ];

        Self {
            pages,
            current_page: 0,
            transition: TransitionState::new(),
            output_buffer: Buffer::empty(Rect::new(0, 0, width, height)),
            transition_types,
            current_transition_idx: 0,
        }
    }

    fn next_transition_type(&mut self) -> TransitionType {
        let t = self.transition_types[self.current_transition_idx].clone();
        self.current_transition_idx = (self.current_transition_idx + 1) % self.transition_types.len();
        t
    }

    fn go_to_page(&mut self, target: usize) {
        if target != self.current_page && target < self.pages.len() && !self.transition.active {
            let transition_type = self.next_transition_type();
            info!("Transition: {} -> {} using {:?}", self.current_page, target, transition_type);
            self.transition.start(self.current_page, target, transition_type);
        }
    }

    /// Get the current rendered buffer (either single page or transition blend)
    pub fn get_rendered_buffer(&mut self) -> &Buffer {
        if self.transition.active {
            // Render both pages to their buffers
            let _ = self.pages[self.transition.from_page].render();
            let _ = self.pages[self.transition.to_page].render();

            // Apply transition effect
            let from_buf = self.pages[self.transition.from_page].buffer();
            let to_buf = self.pages[self.transition.to_page].buffer();

            self.transition.transition.transition(
                from_buf,
                to_buf,
                &mut self.output_buffer,
                self.transition.progress,
            );

            &self.output_buffer
        } else {
            // Render current page
            let _ = self.pages[self.current_page].render();
            self.pages[self.current_page].buffer()
        }
    }
}

impl Model for UiDemoModel {
    fn init(&mut self, _ctx: &mut Context) {
        info!("UI Demo model initialized with {} pages", self.pages.len());
    }

    fn handle_timer(&mut self, _ctx: &mut Context, _dt: f32) {}

    fn handle_event(&mut self, _ctx: &mut Context, _dt: f32) {}

    fn handle_input(&mut self, ctx: &mut Context, dt: f32) {
        // Handle page navigation
        for event in &ctx.input_events {
            if let Event::Key(key_event) = event {
                match key_event.code {
                    KeyCode::Left | KeyCode::Char('h') => {
                        if self.current_page > 0 {
                            self.go_to_page(self.current_page - 1);
                        }
                    }
                    KeyCode::Right | KeyCode::Char('l') => {
                        if self.current_page < self.pages.len() - 1 {
                            self.go_to_page(self.current_page + 1);
                        }
                    }
                    KeyCode::Char('1') => self.go_to_page(0),
                    KeyCode::Char('2') => self.go_to_page(1),
                    KeyCode::Char('3') => self.go_to_page(2),
                    _ => {}
                }
            }
        }

        // Forward input events to current page's UI
        if !self.transition.active {
            for event in &ctx.input_events {
                self.pages[self.current_page].handle_input_event(event.clone());
            }
        }

        ctx.input_events.clear();

        // Update transition
        if self.transition.update(dt) {
            // Transition completed
            self.current_page = self.transition.to_page;
        }

        // Update current page
        if !self.transition.active {
            let _ = self.pages[self.current_page].update(dt);
        }
    }

    fn handle_auto(&mut self, _ctx: &mut Context, _dt: f32) {
        // Rendering is handled in the Render trait now
    }
}

// ============== Page Creation Functions ==============

fn create_page1_interface() -> Panel {
    let mut main_panel = Panel::new()
        .with_bounds(Rect::new(0, 0, UI_DEMO_WIDTH as u16, UI_DEMO_HEIGHT as u16))
        .with_border(BorderStyle::Double)
        .with_title("Page 1: Basic Widgets")
        .with_layout(Box::new(LinearLayout::vertical().with_spacing(1).with_alignment(Alignment::Start)));

    // Navigation hint
    let nav_hint = Label::new("← → or 1/2/3 to switch pages | Transition cycles automatically")
        .with_style(Style::default().fg(Color::Cyan).bg(Color::Black));
    main_panel.add_child(Box::new(nav_hint));

    // Spotlight animation demo
    let spotlight = Label::new("★ SPOTLIGHT ANIMATION DEMO ★")
        .with_style(Style::default().fg(Color::Rgba(200, 200, 200, 255)).bg(Color::Reset))
        .with_spotlight(
            Style::default().fg(Color::Rgba(80, 200, 255, 255)).bg(Color::Reset),
            12, 0.55,
        );
    main_panel.add_child(Box::new(spotlight));

    // Button demo
    let button = Button::new(" Click Me! ")
        .with_style(Style::default().fg(Color::White).bg(Color::Blue))
        .on_click(|| println!("Button clicked!"));
    main_panel.add_child(Box::new(button));

    // TextBox demo
    let textbox = TextBox::new()
        .with_placeholder("Type something here...")
        .with_style(Style::default().fg(Color::Green).bg(Color::Black));
    main_panel.add_child(Box::new(textbox));

    // List demo
    let mut list = List::new()
        .with_selection_mode(SelectionMode::Single)
        .with_style(Style::default().fg(Color::Cyan).bg(Color::Black));
    list.add_text_item("📁 Documents");
    list.add_text_item("🎵 Music Files");
    list.add_text_item("📷 Pictures");
    list.add_text_item("📹 Videos");
    list.add_text_item("⚙️ Settings");
    main_panel.add_child(Box::new(list));

    // ProgressBar demo
    let progress = ProgressBar::new()
        .with_value(0.65)
        .with_fill_style(Style::default().fg(Color::White).bg(Color::Green))
        .with_bar_style(Style::default().fg(Color::Gray).bg(Color::Black));
    main_panel.add_child(Box::new(progress));

    main_panel.layout();
    main_panel
}

fn create_page2_interface() -> Panel {
    let mut main_panel = Panel::new()
        .with_bounds(Rect::new(0, 0, UI_DEMO_WIDTH as u16, UI_DEMO_HEIGHT as u16))
        .with_border(BorderStyle::Double)
        .with_title("Page 2: Text Animations")
        .with_layout(Box::new(LinearLayout::vertical().with_spacing(1).with_alignment(Alignment::Start)));

    // Navigation hint
    let nav_hint = Label::new("← → or 1/2/3 to switch pages | Each transition is different!")
        .with_style(Style::default().fg(Color::Yellow).bg(Color::Black));
    main_panel.add_child(Box::new(nav_hint));

    // Wave animation demo
    let wave = Label::new("~~~ WAVE ANIMATION ~~~")
        .with_style(Style::default().fg(Color::Rgba(255, 200, 80, 255)).bg(Color::Reset))
        .with_wave(0.4, 6.0, 0.15);
    main_panel.add_child(Box::new(wave));

    // FadeIn animation demo
    let fade_in = Label::new(">>> FADE IN EFFECT <<<")
        .with_style(Style::default().fg(Color::Rgba(100, 255, 150, 255)).bg(Color::Reset))
        .with_fade_in(8, true);
    main_panel.add_child(Box::new(fade_in));

    // Typewriter animation demo
    let typewriter = Label::new("TYPEWRITER MODE... TYPING CHARACTER BY CHARACTER...")
        .with_style(Style::default().fg(Color::Rgba(255, 150, 200, 255)).bg(Color::Reset))
        .with_typewriter(6, true, true);
    main_panel.add_child(Box::new(typewriter));

    // Style modifier effects
    let bold = Label::new("BOLD Text - Increased intensity")
        .with_style(Style::default()
            .fg(Color::Cyan)
            .bg(Color::Black)
            .add_modifier(Modifier::BOLD));
    main_panel.add_child(Box::new(bold));

    let italic = Label::new("ITALIC Text - Slanted style")
        .with_style(Style::default()
            .fg(Color::Yellow)
            .bg(Color::Black)
            .add_modifier(Modifier::ITALIC));
    main_panel.add_child(Box::new(italic));

    let underlined = Label::new("UNDERLINED Text - Line at bottom")
        .with_style(Style::default()
            .fg(Color::Green)
            .bg(Color::Black)
            .add_modifier(Modifier::UNDERLINED));
    main_panel.add_child(Box::new(underlined));

    let reversed = Label::new("REVERSED Text - FG/BG swapped")
        .with_style(Style::default()
            .fg(Color::White)
            .bg(Color::Magenta)
            .add_modifier(Modifier::REVERSED));
    main_panel.add_child(Box::new(reversed));

    let bold_italic = Label::new("BOLD+ITALIC - Combined effect")
        .with_style(Style::default()
            .fg(Color::Cyan)
            .bg(Color::Black)
            .add_modifier(Modifier::BOLD | Modifier::ITALIC));
    main_panel.add_child(Box::new(bold_italic));

    main_panel.layout();
    main_panel
}

fn create_page3_interface() -> Panel {
    let mut main_panel = Panel::new()
        .with_bounds(Rect::new(0, 0, UI_DEMO_WIDTH as u16, UI_DEMO_HEIGHT as u16))
        .with_border(BorderStyle::Double)
        .with_title("Page 3: Advanced Components")
        .with_layout(Box::new(LinearLayout::horizontal().with_spacing(2).with_alignment(Alignment::Start)));

    // Left column: Tree
    let mut left_panel = Panel::new()
        .with_bounds(Rect::new(0, 0, 35, 26))
        .with_border(BorderStyle::Single)
        .with_title("File Tree")
        .with_layout(Box::new(LinearLayout::vertical().with_spacing(0).with_alignment(Alignment::Start)));

    let mut tree = Tree::new()
        .with_lines(true)
        .with_style(Style::default().fg(Color::Magenta).bg(Color::Black));

    // Build tree structure
    let home_id = tree.add_root_node("🏠 Home");
    let projects_id = tree.add_root_node("💼 Projects");
    let docs_id = tree.add_root_node("📁 Documents");

    tree.add_child_node(home_id, "📷 Photos");
    tree.add_child_node(home_id, "🎵 Music");
    tree.add_child_node(home_id, "📹 Videos");

    if let Some(rust_id) = tree.add_child_node(projects_id, "🦀 rust_pixel") {
        tree.add_child_node(rust_id, "📄 Cargo.toml");
        tree.add_child_node(rust_id, "📂 src");
        tree.add_child_node(rust_id, "📂 apps");
    }

    tree.add_child_node(docs_id, "📝 notes.md");
    tree.add_child_node(docs_id, "📊 report.pdf");

    left_panel.add_child(Box::new(tree));

    // Right column: Controls
    let mut right_panel = Panel::new()
        .with_bounds(Rect::new(0, 0, 40, 26))
        .with_border(BorderStyle::Single)
        .with_title("Controls")
        .with_layout(Box::new(LinearLayout::vertical().with_spacing(1).with_alignment(Alignment::Start)));

    // Navigation hint
    let nav_hint = Label::new("← → or 1/2/3 to switch pages")
        .with_style(Style::default().fg(Color::Green).bg(Color::Black));
    right_panel.add_child(Box::new(nav_hint));

    // Checkbox demo
    let checkbox = Checkbox::new("Enable feature")
        .with_checked(true)
        .on_change(|checked| println!("Checkbox: {}", checked));
    right_panel.add_child(Box::new(checkbox));

    // ToggleSwitch demo
    let toggle = ToggleSwitch::new("Dark mode")
        .with_on(false)
        .on_change(|on| println!("Toggle: {}", on));
    right_panel.add_child(Box::new(toggle));

    // Slider demo
    let slider = Slider::new(0.0, 100.0)
        .with_value(50.0)
        .with_step(5.0)
        .on_change(|value| println!("Slider: {:.1}", value));
    right_panel.add_child(Box::new(slider));

    // Radio demo
    let radio = RadioGroup::new()
        .with_options(vec!["Option A".to_string(), "Option B".to_string(), "Option C".to_string()])
        .with_selected(0)
        .on_change(|index| println!("Radio: {}", index));
    right_panel.add_child(Box::new(radio));

    // Dropdown demo
    let dropdown = Dropdown::new()
        .with_options(vec!["Apple".to_string(), "Banana".to_string(), "Cherry".to_string()])
        .with_selected(0)
        .on_change(|index| println!("Dropdown: {}", index));
    right_panel.add_child(Box::new(dropdown));

    // Table demo
    let mut table = Table::new()
        .with_columns(vec![
            Column::new("Name", 12).align(ColumnAlign::Left),
            Column::new("Status", 10).align(ColumnAlign::Center),
        ])
        .with_header(true)
        .with_header_style(Style::default().fg(Color::Yellow).bg(Color::Black))
        .with_style(Style::default().fg(Color::White).bg(Color::Black));

    let rows: Vec<TableRow> = vec![
        TableRow::new(vec![TableCell::new("Alpha"), TableCell::new("Active")]),
        TableRow::new(vec![TableCell::new("Beta"), TableCell::new("Pending")]),
        TableRow::new(vec![TableCell::new("Gamma"), TableCell::new("Done")]),
    ];
    table.set_rows(rows);
    right_panel.add_child(Box::new(table));

    main_panel.add_child(Box::new(left_panel));
    main_panel.add_child(Box::new(right_panel));

    main_panel.layout();
    main_panel
}
