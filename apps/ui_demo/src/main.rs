// UI Framework Demo Application
// Demonstrates the rust_pixel UI framework with a simple file browser/gallery viewer

use rust_pixel::game::{Game, Model, Render};
use rust_pixel::context::Context;
use rust_pixel::ui::*;
use rust_pixel::ui::layout::Alignment;
use rust_pixel::render::style::{Color, Style};
use rust_pixel::render::panel::Panel as RenderPanel;
use rust_pixel::util::{Rect, get_project_path};

use log::info;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create the UI demo game (it will initialize logging)
    let model = UIDemoModel::new();
    let render = UIDemoRender::new();
    let mut game = Game::new(model, render, "ui_demo", &get_project_path());
    
    // Initialize and run
    game.init();
    let result = game.run();
    
    // Reset terminal state before exiting (important for terminal mode)
    game.render.panel.reset(&mut game.context);
    
    result?;
    Ok(())
}

// Model - handles UI state and logic
pub struct UIDemoModel {
    ui_app: UIApp,
}

impl UIDemoModel {
    pub fn new() -> Self {
        let mut ui_app = UIApp::new(100, 30);
        
        // Create the main interface
        let root_panel = create_main_interface();
        ui_app.set_root_widget(Box::new(root_panel));
        ui_app.start();
        
        Self { ui_app }
    }
}

impl Model for UIDemoModel {
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

// Render - displays the UI to the terminal
pub struct UIDemoRender {
    panel: RenderPanel,
}

impl UIDemoRender {
    pub fn new() -> Self {
        Self {
            panel: RenderPanel::new(),
        }
    }
}

impl Render for UIDemoRender {
    type Model = UIDemoModel;
    
    fn init(&mut self, ctx: &mut Context, _model: &mut UIDemoModel) {
        info!("UI Demo render initialized");
        // Initialize adapter for large terminal
        ctx.adapter.init(100, 30, 1.0, 1.0, String::new());
        // Initialize the panel to cover the full screen
        self.panel.init(ctx);
    }
    
    fn handle_event(&mut self, _ctx: &mut Context, _model: &mut UIDemoModel, _dt: f32) {
        // Events are handled in the model
    }
    
    fn handle_timer(&mut self, _ctx: &mut Context, _model: &mut UIDemoModel, _dt: f32) {
        // Timer events
    }
    
    fn draw(&mut self, ctx: &mut Context, model: &mut UIDemoModel, _dt: f32) {
        // This is the main drawing method
        self.update(ctx, model, _dt);
    }
    
    fn update(&mut self, ctx: &mut Context, model: &mut UIDemoModel, _dt: f32) {
        // Clear the current buffer
        let buffer = self.panel.current_buffer_mut();
        buffer.reset();
        
        // Copy UI buffer to render buffer
        let ui_buffer = model.ui_app.buffer();
        let ui_area = ui_buffer.area();
        
        // Copy each cell from UI buffer to render buffer
        for y in 0..ui_area.height.min(30) {
            for x in 0..ui_area.width.min(100) {
                let ui_cell = ui_buffer.get(x, y);
                if !ui_cell.is_blank() {
                    buffer.set_string(
                        x, y,
                        &ui_cell.symbol,
                        ui_cell.style(),
                    );
                }
            }
        }
        
        // Draw to screen
        let _ = self.panel.draw(ctx);
    }
}

fn create_main_interface() -> rust_pixel::ui::Panel {
    let mut main_panel = rust_pixel::ui::Panel::new()
        .with_bounds(Rect::new(0, 0, 100, 30))
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

fn create_file_tree_panel() -> rust_pixel::ui::Panel {
    let mut panel = rust_pixel::ui::Panel::new()
        .with_bounds(Rect::new(0, 0, 40, 28))
        .with_border(BorderStyle::Single)
        .with_title("📁 File Explorer")
        .with_layout(Box::new(LinearLayout::vertical()));
    
    // Add file tree with bright colors
    let mut tree = Tree::new()
        .with_lines(true)
        .with_style(Style::default().fg(Color::White).bg(Color::Black))
        .on_selection_changed(|node_id| {
            if let Some(id) = node_id {
                println!("Selected node: {}", id);
            }
        })
        .on_node_expanded(|node_id, expanded| {
            println!("Node {} {}", node_id, if expanded { "expanded" } else { "collapsed" });
        });
    
    // Populate tree with sample data
    populate_sample_tree(&mut tree);
    
    panel.add_child(Box::new(tree));
    panel
}

fn create_details_panel() -> rust_pixel::ui::Panel {
    let mut panel = rust_pixel::ui::Panel::new()
        .with_bounds(Rect::new(0, 0, 58, 28))
        .with_layout(Box::new(LinearLayout::vertical().with_spacing(1)));
    
    // Search box
    let search_panel = create_search_panel();
    panel.add_child(Box::new(search_panel));
    
    // File info panel
    let info_panel = create_file_info_panel();
    panel.add_child(Box::new(info_panel));
    
    // Preview panel
    let preview_panel = create_preview_panel();
    panel.add_child(Box::new(preview_panel));
    
    // Action buttons panel
    let actions_panel = create_actions_panel();
    panel.add_child(Box::new(actions_panel));
    
    panel
}

fn create_search_panel() -> rust_pixel::ui::Panel {
    let mut panel = rust_pixel::ui::Panel::new()
        .with_bounds(Rect::new(0, 0, 56, 4))
        .with_border(BorderStyle::Single)
        .with_title("🔍 Search")
        .with_layout(Box::new(LinearLayout::vertical()));
    
    let search_box = TextBox::new()
        .with_placeholder("Search files and folders...")
        .with_style(Style::default().fg(Color::Yellow).bg(Color::Black))
        .on_changed(|text| {
            println!("Search: {}", text);
        });
    
    panel.add_child(Box::new(search_box));
    panel
}

fn create_file_info_panel() -> rust_pixel::ui::Panel {
    let mut panel = rust_pixel::ui::Panel::new()
        .with_bounds(Rect::new(0, 0, 56, 8))
        .with_border(BorderStyle::Single)
        .with_title("📄 File Information")
        .with_layout(Box::new(LinearLayout::vertical()));
    
    // File name label with strong contrast
    let name_label = Label::new("📁 Name: rust_pixel_project/")
        .with_style(Style::default().fg(Color::Cyan).bg(Color::Black));
    panel.add_child(Box::new(name_label));
    
    // File size label
    let size_label = Label::new("📊 Size: 2.5 MB (2,547,832 bytes)")
        .with_style(Style::default().fg(Color::Green).bg(Color::Black));
    panel.add_child(Box::new(size_label));
    
    // File type label
    let type_label = Label::new("📂 Type: Rust Project Directory")
        .with_style(Style::default().fg(Color::Yellow).bg(Color::Black));
    panel.add_child(Box::new(type_label));
    
    // Modified date label
    let date_label = Label::new("🕒 Modified: 2024-01-15 14:30:25")
        .with_style(Style::default().fg(Color::Magenta).bg(Color::Black));
    panel.add_child(Box::new(date_label));
    
    // Permissions label
    let perm_label = Label::new("🔐 Permissions: drwxr-xr-x")
        .with_style(Style::default().fg(Color::White).bg(Color::Black));
    panel.add_child(Box::new(perm_label));
    
    panel
}

fn create_preview_panel() -> rust_pixel::ui::Panel {
    let mut panel = rust_pixel::ui::Panel::new()
        .with_bounds(Rect::new(0, 0, 56, 12))
        .with_border(BorderStyle::Single)
        .with_title("👁️  Preview")
        .with_layout(Box::new(LinearLayout::vertical()));
    
    // Preview content using a list for better display
    let mut preview_list = List::new()
        .with_selection_mode(SelectionMode::None)
        .with_style(Style::default().fg(Color::Cyan).bg(Color::Black));
    
    preview_list.add_text_item("// Cargo.toml");
    preview_list.add_text_item("[package]");
    preview_list.add_text_item("name = \"rust_pixel\"");
    preview_list.add_text_item("version = \"1.0.5\"");
    preview_list.add_text_item("edition = \"2021\"");
    preview_list.add_text_item("authors = [\"zipxing@hotmail.com\"]");
    preview_list.add_text_item("description = \"2D game engine\"");
    preview_list.add_text_item("");
    preview_list.add_text_item("[dependencies]");
    preview_list.add_text_item("crossterm = \"0.25\"");
    
    panel.add_child(Box::new(preview_list));
    panel
}

fn create_actions_panel() -> rust_pixel::ui::Panel {
    let mut panel = rust_pixel::ui::Panel::new()
        .with_bounds(Rect::new(0, 0, 56, 4))
        .with_border(BorderStyle::Single)
        .with_title("⚡ Actions")
        .with_layout(Box::new(LinearLayout::horizontal().with_spacing(2)));
    
    // Open button
    let open_btn = Button::new("📂 Open")
        .with_style(Style::default().fg(Color::White).bg(Color::Green))
        .on_click(|| println!("Open button clicked!"));
    panel.add_child(Box::new(open_btn));
    
    // Edit button
    let edit_btn = Button::new("✏️  Edit")
        .with_style(Style::default().fg(Color::White).bg(Color::Blue))
        .on_click(|| println!("Edit button clicked!"));
    panel.add_child(Box::new(edit_btn));
    
    // Copy button
    let copy_btn = Button::new("📋 Copy")
        .with_style(Style::default().fg(Color::White).bg(Color::Yellow))
        .on_click(|| println!("Copy button clicked!"));
    panel.add_child(Box::new(copy_btn));
    
    // Delete button
    let delete_btn = Button::new("🗑️  Delete")
        .with_style(Style::default().fg(Color::White).bg(Color::Red))
        .on_click(|| println!("Delete button clicked!"));
    panel.add_child(Box::new(delete_btn));
    
    // Refresh button
    let refresh_btn = Button::new("🔄 Refresh")
        .with_style(Style::default().fg(Color::Black).bg(Color::Gray))
        .on_click(|| println!("Refresh button clicked!"));
    panel.add_child(Box::new(refresh_btn));
    
    panel
}

fn create_status_panel() -> rust_pixel::ui::Panel {
    let mut panel = rust_pixel::ui::Panel::new()
        .with_bounds(Rect::new(0, 0, 98, 2))
        .with_border(BorderStyle::Single)
        .with_layout(Box::new(LinearLayout::horizontal().with_spacing(2)));
    
    // Status label with bright colors
    let status_label = Label::new("🟢 RustPixel UI Framework Running")
        .with_style(Style::default().fg(Color::Green).bg(Color::Black));
    panel.add_child(Box::new(status_label));
    
    // Version info
    let version_label = Label::new("v1.0.5")
        .with_style(Style::default().fg(Color::Yellow).bg(Color::Black));
    panel.add_child(Box::new(version_label));
    
    panel
}

fn populate_sample_tree(tree: &mut Tree) {
    // Root directories
    let home_id = tree.add_root_node("🏠 Home");
    let projects_id = tree.add_root_node("💼 Projects");
    let documents_id = tree.add_root_node("📁 Documents");
    let downloads_id = tree.add_root_node("⬇️  Downloads");
    
    // Home subdirectories
    if let Some(config_id) = tree.add_child_node(home_id, "⚙️  .config") {
        tree.add_child_node(config_id, "🔧 git/");
        tree.add_child_node(config_id, "🎨 nvim/");
        tree.add_child_node(config_id, "🐚 zsh/");
    }
    
    tree.add_child_node(home_id, "🖥️  Desktop");
    tree.add_child_node(home_id, "📸 Pictures");
    tree.add_child_node(home_id, "🎵 Music");
    
    // Projects
    if let Some(rust_id) = tree.add_child_node(projects_id, "🦀 rust_pixel") {
        tree.add_child_node(rust_id, "📦 Cargo.toml");
        tree.add_child_node(rust_id, "📄 README.md");
        if let Some(src_id) = tree.add_child_node(rust_id, "📂 src/") {
            tree.add_child_node(src_id, "🔧 main.rs");
            tree.add_child_node(src_id, "📚 lib.rs");
            tree.add_child_node(src_id, "🎮 game.rs");
            tree.add_child_node(src_id, "🖼️  render/");
            tree.add_child_node(src_id, "🎯 ui/");
        }
        if let Some(apps_id) = tree.add_child_node(rust_id, "📱 apps/") {
            tree.add_child_node(apps_id, "🃏 poker/");
            tree.add_child_node(apps_id, "🐍 snake/");
            tree.add_child_node(apps_id, "🏗️  tower/");
            tree.add_child_node(apps_id, "🎨 ui_demo/");
        }
    }
    
    if let Some(web_id) = tree.add_child_node(projects_id, "🌐 web_projects") {
        tree.add_child_node(web_id, "⚛️  react_app/");
        tree.add_child_node(web_id, "🅰️  angular_app/");
        tree.add_child_node(web_id, "💚 vue_app/");
    }
    
    tree.add_child_node(projects_id, "🐍 python_scripts/");
    tree.add_child_node(projects_id, "☕ java_projects/");
    tree.add_child_node(projects_id, "📱 mobile_apps/");
    
    // Documents
    tree.add_child_node(documents_id, "📝 notes.md");
    tree.add_child_node(documents_id, "📊 spreadsheet.xlsx");
    tree.add_child_node(documents_id, "📋 todo_list.txt");
    tree.add_child_node(documents_id, "💼 work_docs/");
    tree.add_child_node(documents_id, "🎓 study_materials/");
    
    // Downloads
    tree.add_child_node(downloads_id, "📦 software.zip");
    tree.add_child_node(downloads_id, "🎵 music_album.tar.gz");
    tree.add_child_node(downloads_id, "📖 ebook.pdf");
    tree.add_child_node(downloads_id, "🖼️  wallpapers.rar");
    tree.add_child_node(downloads_id, "⚙️  tools_collection/");
}



