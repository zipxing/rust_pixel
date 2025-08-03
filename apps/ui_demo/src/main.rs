// UI Framework Demo Application
// Demonstrates the rust_pixel UI framework with a simple file browser/gallery viewer

use rust_pixel::ui::*;
use rust_pixel::render::style::{Color, Style};
use rust_pixel::util::Rect;
use std::fs;
use std::path::{Path, PathBuf};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize the UI application
    let mut app = UIAppBuilder::new(100, 30)
        .with_title("RustPixel UI Demo - File Browser")
        .with_theme("dark")
        .with_frame_rate(30)
        .build()?;
    
    // Create the main interface
    let root_panel = create_main_interface();
    app.set_root_widget(Box::new(root_panel));
    
    // Note: In a real implementation, you would integrate this with rust_pixel's
    // main game loop. For now, this is a conceptual demonstration.
    println!("UI Framework Demo created successfully!");
    println!("Root widget configured with file browser interface.");
    
    Ok(())
}

fn create_main_interface() -> Panel {
    let mut main_panel = Panel::new()
        .with_bounds(Rect::new(0, 0, 100, 30))
        .with_border(BorderStyle::Single)
        .with_title("File Browser Demo")
        .with_layout(Box::new(LinearLayout::horizontal().with_spacing(1)));
    
    // Left panel - File tree
    let left_panel = create_file_tree_panel();
    main_panel.add_child(Box::new(left_panel));
    
    // Right panel - File details and preview
    let right_panel = create_details_panel();
    main_panel.add_child(Box::new(right_panel));
    
    main_panel
}

fn create_file_tree_panel() -> Panel {
    let mut panel = Panel::new()
        .with_bounds(Rect::new(0, 0, 40, 28))
        .with_border(BorderStyle::Single)
        .with_title("Files")
        .with_layout(Box::new(LinearLayout::vertical()));
    
    // Add file tree
    let mut tree = Tree::new()
        .with_lines(true)
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

fn create_details_panel() -> Panel {
    let mut panel = Panel::new()
        .with_bounds(Rect::new(0, 0, 58, 28))
        .with_layout(Box::new(LinearLayout::vertical().with_spacing(1)));
    
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

fn create_file_info_panel() -> Panel {
    let mut panel = Panel::new()
        .with_bounds(Rect::new(0, 0, 56, 8))
        .with_border(BorderStyle::Single)
        .with_title("File Information")
        .with_layout(Box::new(LinearLayout::vertical()));
    
    // File name label
    let name_label = Label::new("Name: example.txt")
        .with_style(Style::default().fg(Color::Yellow));
    panel.add_child(Box::new(name_label));
    
    // File size label
    let size_label = Label::new("Size: 1024 bytes")
        .with_style(Style::default().fg(Color::Green));
    panel.add_child(Box::new(size_label));
    
    // File type label
    let type_label = Label::new("Type: Text file")
        .with_style(Style::default().fg(Color::Cyan));
    panel.add_child(Box::new(type_label));
    
    // Modified date label
    let date_label = Label::new("Modified: 2024-01-15 10:30:00")
        .with_style(Style::default().fg(Color::Magenta));
    panel.add_child(Box::new(date_label));
    
    panel
}

fn create_preview_panel() -> Panel {
    let mut panel = Panel::new()
        .with_bounds(Rect::new(0, 0, 56, 15))
        .with_border(BorderStyle::Single)
        .with_title("Preview")
        .with_layout(Box::new(LinearLayout::vertical()));
    
    // Preview content
    let preview_content = Label::new("File preview will appear here...\n\nFor text files: content preview\nFor images: ASCII art representation\nFor directories: file listing")
        .with_wrap(true)
        .with_style(Style::default().fg(Color::White));
    
    panel.add_child(Box::new(preview_content));
    panel
}

fn create_actions_panel() -> Panel {
    let mut panel = Panel::new()
        .with_bounds(Rect::new(0, 0, 56, 4))
        .with_border(BorderStyle::Single)
        .with_title("Actions")
        .with_layout(Box::new(LinearLayout::horizontal().with_spacing(2)));
    
    // Open button
    let open_btn = Button::new("Open")
        .with_button_style(ButtonStyle::Normal)
        .on_click(|| println!("Open button clicked!"));
    panel.add_child(Box::new(open_btn));
    
    // Edit button
    let edit_btn = Button::new("Edit")
        .with_button_style(ButtonStyle::Normal)
        .on_click(|| println!("Edit button clicked!"));
    panel.add_child(Box::new(edit_btn));
    
    // Delete button
    let delete_btn = Button::new("Delete")
        .with_button_style(ButtonStyle::Normal)
        .on_click(|| println!("Delete button clicked!"));
    panel.add_child(Box::new(delete_btn));
    
    // Refresh button
    let refresh_btn = Button::new("Refresh")
        .with_button_style(ButtonStyle::Outlined)
        .on_click(|| println!("Refresh button clicked!"));
    panel.add_child(Box::new(refresh_btn));
    
    panel
}

fn populate_sample_tree(tree: &mut Tree) {
    // Root directories
    let home_id = tree.add_root_node("🏠 Home");
    let documents_id = tree.add_root_node("📁 Documents");
    let pictures_id = tree.add_root_node("🖼️ Pictures");
    let downloads_id = tree.add_root_node("⬇️ Downloads");
    
    // Home subdirectories
    if let Some(projects_id) = tree.add_child_node(home_id, "📂 Projects") {
        tree.add_child_node(projects_id, "🦀 rust_pixel");
        tree.add_child_node(projects_id, "🐍 python_scripts");
        tree.add_child_node(projects_id, "🌐 web_apps");
    }
    
    tree.add_child_node(home_id, "⚙️ .config");
    tree.add_child_node(home_id, "📄 README.md");
    
    // Documents
    tree.add_child_node(documents_id, "📝 notes.txt");
    tree.add_child_node(documents_id, "📊 spreadsheet.csv");
    tree.add_child_node(documents_id, "📋 todo.md");
    
    // Pictures
    if let Some(vacation_id) = tree.add_child_node(pictures_id, "🌴 Vacation 2024") {
        tree.add_child_node(vacation_id, "🏖️ beach1.jpg");
        tree.add_child_node(vacation_id, "🌅 sunset.png");
        tree.add_child_node(vacation_id, "🏔️ mountain.jpg");
    }
    
    tree.add_child_node(pictures_id, "📷 camera_roll");
    tree.add_child_node(pictures_id, "🎨 artwork");
    
    // Downloads
    tree.add_child_node(downloads_id, "📦 package.zip");
    tree.add_child_node(downloads_id, "🎵 music.mp3");
    tree.add_child_node(downloads_id, "📖 ebook.pdf");
}