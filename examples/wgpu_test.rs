// RustPixel
// copyright zipxing@hotmail.com 2022~2024

//! Simple test for WGPU adapter

use rust_pixel::{
    event::Event,
    render::{
        adapter::{wgpu::WgpuAdapter, Adapter},
        buffer::Buffer,
        sprite::Sprites,
    },
    util::Rect,
};
use std::time::Duration;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Simple logging setup
    println!("Starting WGPU test...");
    
    let mut adapter = WgpuAdapter::new("wgpu_test", ".");
    
    // Initialize the adapter
    adapter.init(80, 25, 1.0, 1.0, "WGPU Test Window".to_string());
    
    println!("WGPU Adapter initialized");
    println!("Cell size: {}x{}", adapter.cell_width(), adapter.cell_height());
    
    // Create a simple buffer for testing
    let mut buffer = Buffer::empty(Rect::new(0, 0, 80, 25));
    let mut sprites = Vec::<Sprites>::new();
    let mut events = Vec::<Event>::new();
    
    // Simple test loop
    for frame in 0..60 {
        // Poll events
        let should_exit = adapter.poll_event(Duration::from_millis(16), &mut events);
        if should_exit {
            break;
        }
        
        // Handle events
        for event in &events {
            println!("Event: {:?}", event);
        }
        events.clear();
        
        // Simple rendering test
        if let Err(e) = adapter.draw_all_to_screen(&buffer, &buffer, &mut sprites, frame) {
            eprintln!("Render error: {}", e);
            break;
        }
        
        if frame % 10 == 0 {
            println!("Frame: {}", frame);
        }
    }
    
    println!("Test completed");
    Ok(())
} 