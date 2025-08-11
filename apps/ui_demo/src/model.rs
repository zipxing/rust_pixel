use rust_pixel::game::Model;
use rust_pixel::context::Context;
use rust_pixel::ui::*;
use rust_pixel::ui::layout::Alignment;
use rust_pixel::render::style::{Color, Style};
use rust_pixel::util::Rect;
use log::info;

pub const UI_DEMO_WIDTH: usize = 100;
pub const UI_DEMO_HEIGHT: usize = 30;

// Model - handles UI state and logic
pub struct UiDemoModel {
    pub ui_app: UIApp,
}

impl UiDemoModel {
    pub fn new() -> Self {
        let mut ui_app = UIApp::new(UI_DEMO_WIDTH as u16, UI_DEMO_HEIGHT as u16);
        
        // Create the main interface
        let root_panel = create_main_interface();
        ui_app.set_root_widget(Box::new(root_panel));
        ui_app.start();
        
        Self { ui_app }
    }
}

impl Model for UiDemoModel {
    fn init(&mut self, _ctx: &mut Context) {
        info!("UI Demo model initialized");
    }
    
    fn handle_timer(&mut self, _ctx: &mut Context, _dt: f32) {}
    
    fn handle_event(&mut self, _ctx: &mut Context, _dt: f32) {}
    
    fn handle_input(&mut self, ctx: &mut Context, dt: f32) {
        // Forward input events to UI
        for event in &ctx.input_events {
            self.ui_app.handle_input_event(event.clone());
        }
        
        // Clear input events to prevent reprocessing
        ctx.input_events.clear();
        
        // Update UI
        let _ = self.ui_app.update(dt);
    }
    
    fn handle_auto(&mut self, _ctx: &mut Context, _dt: f32) {
        // Render UI if needed
        if self.ui_app.should_render() {
            let _ = self.ui_app.render();
            self.ui_app.frame_complete();
        }
    }
}

fn create_main_interface() -> rust_pixel::ui::Panel {
    let mut main_panel = rust_pixel::ui::Panel::new()
        .with_bounds(Rect::new(0, 0, UI_DEMO_WIDTH as u16, UI_DEMO_HEIGHT as u16))
        .with_border(BorderStyle::Single)
        .with_title("UI Debug - Step 5: All Basic Widgets")
        .with_layout(Box::new(LinearLayout::horizontal().with_spacing(2).with_alignment(Alignment::Start)));
    
    // Left column: Simple widgets
    let mut left_panel = rust_pixel::ui::Panel::new()
        .with_bounds(Rect::new(0, 0, 48, 28))
        .with_border(BorderStyle::Single)
        .with_title("Basic Controls")
        .with_layout(Box::new(LinearLayout::vertical().with_spacing(1).with_alignment(Alignment::Start)));
    
    // Step 1: Label (working)
    let test_label = Label::new("Hello RustPixel UI Framework!")
        .with_style(Style::default().fg(Color::Yellow).bg(Color::Black));
    left_panel.add_child(Box::new(test_label));
    
    // Step 2: Button (working)
    let test_button = Button::new("Click Me!")
        .with_style(Style::default().fg(Color::White).bg(Color::Blue))
        .on_click(|| println!("Button clicked!"));
    left_panel.add_child(Box::new(test_button));
    
    // Step 3: TextBox (working)
    let mut test_textbox = TextBox::new()
        .with_placeholder("Type something here...")
        .with_style(Style::default().fg(Color::Green).bg(Color::Black))
        .on_changed(|text| println!("Text changed: {}", text));
    
    // Give the textbox focus for input testing
    test_textbox.set_focused(true);
    left_panel.add_child(Box::new(test_textbox));
    
    // Step 4: List (working)
    let mut test_list = List::new()
        .with_selection_mode(SelectionMode::Single)
        .with_style(Style::default().fg(Color::Cyan).bg(Color::Black))
        .on_selection_changed(|indices| {
            println!("List selection changed: {:?}", indices);
        });
    
    // Add some test items
    test_list.add_text_item("🎵 Music Files");
    test_list.add_text_item("📁 Documents");
    test_list.add_text_item("🖼️ Pictures");
    test_list.add_text_item("📹 Videos");
    test_list.add_text_item("⚙️ Settings");
    
    left_panel.add_child(Box::new(test_list));
    
    // Right column: Tree widget
    let mut right_panel = rust_pixel::ui::Panel::new()
        .with_bounds(Rect::new(0, 0, 48, 28))
        .with_border(BorderStyle::Single)
        .with_title("File Tree")
        .with_layout(Box::new(LinearLayout::vertical().with_spacing(0).with_alignment(Alignment::Start)));
    
    // Step 5: Tree for hierarchical data
    let mut test_tree = Tree::new()
        .with_lines(true)
        .with_style(Style::default().fg(Color::Magenta).bg(Color::Black))
        .on_selection_changed(|node_id| {
            if let Some(id) = node_id {
                println!("Tree node selected: {}", id);
            }
        })
        .on_node_expanded(|node_id, expanded| {
            println!("Tree node {} {}", node_id, if expanded { "expanded" } else { "collapsed" });
        });
    
    // Build sample tree structure
    create_sample_tree(&mut test_tree);
    right_panel.add_child(Box::new(test_tree));
    
    // Add panels to main layout
    main_panel.add_child(Box::new(left_panel));
    main_panel.add_child(Box::new(right_panel));
    
    main_panel
}

// Helper function to create sample tree structure
fn create_sample_tree(tree: &mut Tree) {
    // Root folders
    let home_id = tree.add_root_node("🏠 Home");
    let projects_id = tree.add_root_node("💼 Projects");
    let docs_id = tree.add_root_node("📁 Documents");
    
    // Home folder contents
    tree.add_child_node(home_id, "📷 Photos");
    tree.add_child_node(home_id, "🎵 Music");
    tree.add_child_node(home_id, "📹 Videos");
    
    // Projects folder contents
    if let Some(rust_id) = tree.add_child_node(projects_id, "🦀 rust_pixel") {
        tree.add_child_node(rust_id, "📄 Cargo.toml");
        tree.add_child_node(rust_id, "📂 src");
        tree.add_child_node(rust_id, "📂 apps");
    }
    
    if let Some(webdev_id) = tree.add_child_node(projects_id, "🌐 webdev") {
        tree.add_child_node(webdev_id, "📄 package.json");
        tree.add_child_node(webdev_id, "📂 src");
    }
    
    // Documents folder contents
    tree.add_child_node(docs_id, "📝 notes.md");
    tree.add_child_node(docs_id, "📊 report.pdf");
    tree.add_child_node(docs_id, "📋 todo.txt");
}