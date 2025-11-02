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
    
    // Right column: Tabs + Tree/About
    let mut tabs = Tabs::new()
        .with_style(
            Style::default().fg(Color::Gray).bg(Color::Black),      // inactive tab
            Style::default().fg(Color::White).bg(Color::Blue)        // active tab
        );
    
    // Page 1: File Tree
    let mut tree_panel = rust_pixel::ui::Panel::new()
        .with_border(BorderStyle::Single)
        .with_title("File Tree")
        .with_layout(Box::new(LinearLayout::vertical().with_spacing(0).with_alignment(Alignment::Start)));
    
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
    create_sample_tree(&mut test_tree);
    tree_panel.add_child(Box::new(test_tree));
    
    // Page 2: About with Modal demo
    let about_panel = rust_pixel::ui::Panel::new()
        .with_border(BorderStyle::Single)
        .with_title("About & Modal Demo")
        .with_layout(Box::new(LinearLayout::vertical().with_spacing(1).with_alignment(Alignment::Start)));
    
    let mut about = about_panel;
    about.add_child(Box::new(Label::new("UI Components Demo")));
    
    // ProgressBar demo
    let progress = rust_pixel::ui::ProgressBar::new()
        .with_value(0.65)
        .with_fill_style(Style::default().fg(Color::White).bg(Color::Green))
        .with_bar_style(Style::default().fg(Color::Gray).bg(Color::Black));
    about.add_child(Box::new(progress));
    
    // Checkbox demo
    let checkbox = rust_pixel::ui::Checkbox::new("Enable feature")
        .with_checked(true)
        .on_change(|checked| {
            println!("Checkbox changed: {}", checked);
        });
    about.add_child(Box::new(checkbox));
    
    // ToggleSwitch demo
    let toggle = rust_pixel::ui::ToggleSwitch::new("Dark mode")
        .with_on(false)
        .on_change(|on| {
            println!("Toggle changed: {}", on);
        });
    about.add_child(Box::new(toggle));
    
    // Slider demo
    let slider = rust_pixel::ui::Slider::new(0.0, 100.0)
        .with_value(50.0)
        .with_step(5.0)
        .on_change(|value| {
            println!("Slider value: {:.1}", value);
        });
    about.add_child(Box::new(slider));

    // Page 3: Modal Demo
    let mut modal_demo_panel = rust_pixel::ui::Panel::new()
        .with_border(BorderStyle::Single)
        .with_title("Modal Demo")
        .with_layout(Box::new(LinearLayout::vertical().with_spacing(0).with_alignment(Alignment::Start)));
    
    // Create a modal dialog
    let mut demo_modal = rust_pixel::ui::Modal::new()
        .with_title("Example Dialog")
        .with_min_size(40, 12);
    
    // Add content to modal
    demo_modal.add_content(Box::new(Label::new("This is a modal dialog!")));
    demo_modal.add_content(Box::new(Label::new("It has a backdrop and centered content.")));
    demo_modal.add_content(Box::new(Label::new("")));
    demo_modal.add_content(Box::new(Label::new("Press ESC to close (not functional yet)")));
    
    // Add buttons to modal
    let ok_btn = Button::new("  OK  ")
        .with_style(Style::default().fg(Color::White).bg(Color::Green))
        .on_click(|| {
            println!("OK clicked!");
        });
    demo_modal.add_button(Box::new(ok_btn));
    
    let cancel_btn = Button::new("Cancel")
        .with_style(Style::default().fg(Color::White).bg(Color::Red))
        .on_click(|| {
            println!("Cancel clicked!");
        });
    demo_modal.add_button(Box::new(cancel_btn));
    
    modal_demo_panel.add_child(Box::new(demo_modal));

    // Page 4: More Components (Radio, Dropdown, Toast)
    let components_panel = rust_pixel::ui::Panel::new()
        .with_border(BorderStyle::Single)
        .with_title("More Components")
        .with_layout(Box::new(LinearLayout::vertical().with_spacing(1).with_alignment(Alignment::Start)));
    
    let mut components = components_panel;
    components.add_child(Box::new(Label::new("Radio & Dropdown Demo")));
    
    // Radio demo
    let radio = rust_pixel::ui::RadioGroup::new()
        .with_options(vec!["Option A".to_string(), "Option B".to_string(), "Option C".to_string()])
        .with_selected(0)
        .on_change(|index| {
            println!("Radio selected: {}", index);
        });
    components.add_child(Box::new(radio));
    
    // Dropdown demo
    let dropdown = rust_pixel::ui::Dropdown::new()
        .with_options(vec!["Apple".to_string(), "Banana".to_string(), "Cherry".to_string(), "Date".to_string()])
        .with_selected(0)
        .on_change(|index| {
            println!("Dropdown selected: {}", index);
        });
    components.add_child(Box::new(dropdown));
    
    // Toast demo (positioned at top)
    let toast = rust_pixel::ui::Toast::new("This is a notification!")
        .with_type(rust_pixel::ui::ToastType::Success)
        .with_duration(5.0);
    components.add_child(Box::new(toast));

    // Add pages to tabs
    tabs.add_tab("Tree", Box::new(tree_panel));
    tabs.add_tab("About", Box::new(about));
    tabs.add_tab("Components", Box::new(components));
    tabs.add_tab("Modal", Box::new(modal_demo_panel));

    // Add to main layout
    main_panel.add_child(Box::new(left_panel));
    main_panel.add_child(Box::new(tabs));
    
    // Trigger initial layout
    main_panel.layout();
    
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