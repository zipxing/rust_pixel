//! # RustPixel Asset Packer
//!
//! A command-line tool for packing multiple images into a texture atlas and generating
//! corresponding `.pix` files for use with the RustPixel engine.
//!
//! ## Features
//!
//! - **Image Packing**: Uses MaxRects bin packing algorithm for efficient space utilization
//! - **Size Optimization**: Automatically adjusts image sizes to multiples of 16 pixels
//! - **Texture Atlas Generation**: Creates a 4096x4096 texture atlas with region support
//! - **PIX File Generation**: Generates `.pix` metadata files for each packed image
//! - **Region-aware Packing**: Supports packing into Sprite region (blocks 0-159)
//!
//! ## 4096x4096 Texture Layout
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────┐
//! │ SPRITE Region (y: 0-2559, 2560px height)                   │
//! │ • 160 blocks (10 rows × 16 columns)                         │
//! │ • Block size: 256×256px (16×16 symbols at 16×16px)         │
//! │ • Total: 40,960 sprites                                     │
//! ├─────────────────────────────────────────────────────────────┤
//! │ TUI + EMOJI Region (y: 2560-3071, 512px height)            │
//! │ • TUI (x: 0-2559): 10 blocks, 16×32px symbols              │
//! │ • Emoji (x: 2560-4095): 6 blocks, 32×32px symbols          │
//! ├─────────────────────────────────────────────────────────────┤
//! │ CJK Region (y: 3072-4095, 1024px height)                   │
//! │ • 64 blocks (16 cols × 4 rows), 32×32px symbols            │
//! └─────────────────────────────────────────────────────────────┘
//! ```
//!
//! ## Usage
//!
//! ```bash
//! cargo pixel r asset t -r <input_folder> <output_folder> [--region sprite|full] [--start-block N]
//! ```
//!
//! ## Input Requirements
//!
//! - Input folder should contain image files (PNG, JPEG, etc.)
//! - Images will be automatically resized and packed efficiently
//! - The tool requires `assets/pix/symbols.png` to exist as base texture
//!
//! ## Output
//!
//! - `texture_atlas.png`: Combined texture atlas containing all input images
//! - `*.pix`: Metadata files for each input image with texture coordinates

use image::imageops::FilterType;
use image::GenericImage;
use image::{DynamicImage, GenericImageView, RgbaImage};
use serde::Deserialize;
use std::collections::HashSet;
use std::env;
use std::fs::{self, File};
use std::io::Write;
use std::path::Path;
use std::process;

/// Application configuration and constants for 4096x4096 texture atlas
mod config {
    /// Texture atlas width in pixels (4096 for new layout)
    pub const ATLAS_WIDTH: u32 = 4096;
    /// Texture atlas height in pixels
    pub const ATLAS_HEIGHT: u32 = 4096;
    /// Grid size for texture coordinate calculations (symbol base size)
    pub const GRID_SIZE: u32 = 16;
    /// Path to the base symbols texture
    pub const SYMBOLS_TEXTURE_PATH: &str = "assets/pix/symbols.png";

    /// Base symbol dimensions
    pub const SYMBOL_WIDTH: u32 = 16;
    pub const SYMBOL_HEIGHT: u32 = 16;

    /// Texture grid dimensions (256x256 grid of base symbols)
    pub const GRID_COLS: u32 = 256;
    pub const GRID_ROWS: u32 = 256;
}

/// Texture region layout constants (matching symbol_map.rs)
mod layout {
    use super::config::*;

    // Sprite region: blocks 0-159 (10 rows × 16 cols)
    pub const SPRITE_BLOCK_ROWS: u32 = 10;
    pub const SPRITE_BLOCK_COLS: u32 = 16;
    pub const SPRITE_BLOCKS: u32 = SPRITE_BLOCK_ROWS * SPRITE_BLOCK_COLS; // 160
    pub const SPRITE_Y_START: u32 = 0;
    pub const SPRITE_Y_END: u32 = SPRITE_BLOCK_ROWS * 16 * SYMBOL_HEIGHT; // 2560
    pub const SPRITE_SYMBOLS_PER_BLOCK: u32 = 256; // 16x16

    // Block dimensions
    pub const BLOCK_WIDTH: u32 = 16 * SYMBOL_WIDTH;   // 256
    pub const BLOCK_HEIGHT: u32 = 16 * SYMBOL_HEIGHT; // 256

    // TUI region: blocks 160-169 (y: 2560-3071, x: 0-2559)
    pub const TUI_BLOCK_START: u32 = 160;
    pub const TUI_BLOCKS: u32 = 10;
    pub const TUI_Y_START: u32 = SPRITE_Y_END; // 2560
    pub const TUI_X_END: u32 = TUI_BLOCKS * BLOCK_WIDTH; // 2560
    pub const TUI_SYMBOL_HEIGHT: u32 = SYMBOL_HEIGHT * 2; // 32

    // Emoji region: blocks 170-175 (y: 2560-3071, x: 2560-4095)
    pub const EMOJI_BLOCK_START: u32 = 170;
    pub const EMOJI_BLOCKS: u32 = 6;
    pub const EMOJI_X_START: u32 = TUI_X_END; // 2560
    pub const EMOJI_SYMBOL_SIZE: u32 = 32;

    // CJK region: blocks 176-239 (y: 3072-4095)
    pub const CJK_BLOCK_START: u32 = 176;
    pub const CJK_Y_START: u32 = 3072;
    pub const CJK_SYMBOL_SIZE: u32 = 32;

    /// Get packing region for sprite blocks (starting from a specific block)
    /// Returns (x, y, width, height) of available packing area
    pub fn sprite_packing_region(start_block: u32) -> (u32, u32, u32, u32) {
        let block_row = start_block / SPRITE_BLOCK_COLS;
        let y_start = block_row * BLOCK_HEIGHT;
        let height = SPRITE_Y_END - y_start;
        (0, y_start, ATLAS_WIDTH, height)
    }

    /// Get full sprite region for packing
    pub fn full_sprite_region() -> (u32, u32, u32, u32) {
        (0, SPRITE_Y_START, ATLAS_WIDTH, SPRITE_Y_END)
    }
}

// ============================================================================
// Symbol Map JSON Structures
// ============================================================================

/// Represents a region in the symbol map
#[derive(Debug, Deserialize)]
struct SymbolRegion {
    #[serde(rename = "type")]
    region_type: String,
    block_range: [u32; 2],
    char_size: [u32; 2],
    chars_per_block: u32,
    #[serde(default)]
    symbols: SymbolsList,
    #[serde(default)]
    extras: std::collections::HashMap<String, [u32; 2]>,
}

/// Symbols can be either a string or an array of strings
#[derive(Debug, Deserialize, Default)]
#[serde(untagged)]
enum SymbolsList {
    #[default]
    Empty,
    String(String),
    Array(Vec<String>),
}

impl SymbolsList {
    fn len(&self) -> usize {
        match self {
            SymbolsList::Empty => 0,
            SymbolsList::String(s) => s.chars().count(),
            SymbolsList::Array(arr) => arr.len(),
        }
    }
}

/// Root structure of symbol_map.json
#[derive(Debug, Deserialize)]
struct SymbolMap {
    #[allow(dead_code)]
    version: u32,
    #[allow(dead_code)]
    texture_size: u32,
    regions: std::collections::HashMap<String, SymbolRegion>,
}

/// Block occupancy information
#[derive(Debug, Default)]
struct BlockOccupancy {
    /// Set of occupied block indices in sprite region
    occupied_sprite_blocks: HashSet<u32>,
    /// Total symbols in sprite region
    total_sprite_symbols: usize,
    /// Per-region statistics
    region_stats: Vec<(String, u32, u32, usize)>, // (name, start_block, end_block, symbol_count)
}

impl BlockOccupancy {
    /// Calculates the first free block in sprite region
    fn first_free_sprite_block(&self) -> u32 {
        for block in 0..layout::SPRITE_BLOCKS {
            if !self.occupied_sprite_blocks.contains(&block) {
                return block;
            }
        }
        layout::SPRITE_BLOCKS // All occupied
    }

    /// Calculates how many symbols are needed to consider a block "occupied"
    fn is_block_occupied(&self, block: u32) -> bool {
        self.occupied_sprite_blocks.contains(&block)
    }

    /// Displays block usage summary
    fn print_summary(&self) {
        println!("Block Occupancy Summary:");
        println!("────────────────────────");
        for (name, start, end, symbols) in &self.region_stats {
            println!("  {}: blocks {}-{}, {} symbols", name, start, end, symbols);
        }
        println!();
        println!("Sprite region: {}/{} blocks occupied",
            self.occupied_sprite_blocks.len(), layout::SPRITE_BLOCKS);
        println!("First free block: {}", self.first_free_sprite_block());
    }
}

/// Parses symbol_map.json and calculates block occupancy
fn parse_symbol_map(path: &str) -> Result<BlockOccupancy, String> {
    let content = fs::read_to_string(path)
        .map_err(|e| format!("Failed to read symbol map '{}': {}", path, e))?;

    let symbol_map: SymbolMap = serde_json::from_str(&content)
        .map_err(|e| format!("Failed to parse symbol map '{}': {}", path, e))?;

    let mut occupancy = BlockOccupancy::default();

    for (region_name, region) in &symbol_map.regions {
        let [start_block, end_block] = region.block_range;
        let symbol_count = region.symbols.len() + region.extras.len();

        occupancy.region_stats.push((
            region_name.clone(),
            start_block,
            end_block,
            symbol_count,
        ));

        // Mark sprite region blocks as occupied if they have symbols
        if region_name == "sprite" && symbol_count > 0 {
            // Block 0 is occupied for basic PETSCII symbols
            occupancy.occupied_sprite_blocks.insert(0);
            occupancy.total_sprite_symbols = symbol_count;
        }
    }

    // Sort stats by start_block for display
    occupancy.region_stats.sort_by_key(|(_, start, _, _)| *start);

    Ok(occupancy)
}

/// Packing region mode
#[derive(Clone, Copy, Debug, PartialEq)]
enum PackingRegion {
    /// Pack into sprite region only (y: 0-2559)
    Sprite,
    /// Pack into full atlas (all 4096x4096)
    Full,
}

impl PackingRegion {
    fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "sprite" => Some(PackingRegion::Sprite),
            "full" => Some(PackingRegion::Full),
            _ => None,
        }
    }
}

/// Represents a rectangular area in 2D space
#[derive(Clone, Copy, Debug)]
struct Rectangle {
    /// X coordinate of the rectangle's top-left corner
    x: u32,
    /// Y coordinate of the rectangle's top-left corner
    y: u32,
    /// Width of the rectangle
    width: u32,
    /// Height of the rectangle
    height: u32,
}

/// MaxRects bin packing algorithm implementation
///
/// This algorithm efficiently packs rectangles into a larger container rectangle
/// by maintaining a list of free rectangular areas and splitting them as needed.
struct MaxRectsBin {
    /// List of available rectangular areas for packing
    free_rects: Vec<Rectangle>,
    /// List of already used rectangular areas
    used_rects: Vec<Rectangle>,
    /// Y offset for the packing region
    y_offset: u32,
}

impl MaxRectsBin {
    /// Creates a new bin with the specified dimensions and offset
    ///
    /// # Arguments
    /// * `width` - Total width of the packing area
    /// * `height` - Total height of the packing area
    /// * `y_offset` - Y offset to apply to all packed rectangles
    ///
    /// # Returns
    /// A new MaxRectsBin instance with a single free rectangle covering the entire area
    fn new(width: u32, height: u32, y_offset: u32) -> Self {
        let initial_rect = Rectangle {
            x: 0,
            y: 0,
            width,
            height,
        };
        MaxRectsBin {
            free_rects: vec![initial_rect],
            used_rects: Vec::new(),
            y_offset,
        }
    }

    /// Creates a new bin from region parameters
    fn from_region(region: (u32, u32, u32, u32)) -> Self {
        let (x, y, width, height) = region;
        let initial_rect = Rectangle {
            x,
            y: 0,
            width,
            height,
        };
        MaxRectsBin {
            free_rects: vec![initial_rect],
            used_rects: Vec::new(),
            y_offset: y,
        }
    }

    /// Attempts to insert a rectangle with the given dimensions into the bin
    ///
    /// # Arguments
    /// * `width` - Width of the rectangle to insert
    /// * `height` - Height of the rectangle to insert
    ///
    /// # Returns
    /// Some(Rectangle) with the position where the rectangle was placed, or None if no space available
    fn insert(&mut self, width: u32, height: u32) -> Option<Rectangle> {
        if let Some(best_rect) = self.find_position_for_new_node_best_area_fit(width, height) {
            let new_node = Rectangle {
                x: best_rect.x,
                y: best_rect.y,
                width,
                height,
            };
            self.place_rectangle(new_node);
            // Return rectangle with Y offset applied
            Some(Rectangle {
                x: new_node.x,
                y: new_node.y + self.y_offset,
                width: new_node.width,
                height: new_node.height,
            })
        } else {
            None
        }
    }

    /// Finds the best position for a new rectangle using best-area-fit heuristic
    fn find_position_for_new_node_best_area_fit(
        &self,
        width: u32,
        height: u32,
    ) -> Option<Rectangle> {
        let mut best_area_fit = u32::MAX;
        let mut best_rect = None;

        for rect in &self.free_rects {
            if width <= rect.width && height <= rect.height {
                let area_fit = rect.width * rect.height - width * height;
                if area_fit < best_area_fit {
                    best_area_fit = area_fit;
                    best_rect = Some(Rectangle {
                        x: rect.x,
                        y: rect.y,
                        width,
                        height,
                    });
                }
            }
        }

        best_rect
    }

    /// Places a rectangle in the bin and updates the free rectangle list
    fn place_rectangle(&mut self, rect: Rectangle) {
        self.used_rects.push(rect);

        // Split free rectangles that overlap with the placed rectangle
        let mut i = 0;
        while i < self.free_rects.len() {
            if self.split_free_node(self.free_rects[i], rect) {
                self.free_rects.remove(i);
            } else {
                i += 1;
            }
        }

        // Remove redundant free rectangles
        self.prune_free_list();
    }

    /// Splits a free rectangle around a used rectangle
    fn split_free_node(&mut self, free_rect: Rectangle, used_rect: Rectangle) -> bool {
        // If rectangles don't overlap, no splitting needed
        if !self.is_overlapping(free_rect, used_rect) {
            return false;
        }

        let mut new_rects = Vec::new();

        // Top area
        if used_rect.y > free_rect.y && used_rect.y < free_rect.y + free_rect.height {
            new_rects.push(Rectangle {
                x: free_rect.x,
                y: free_rect.y,
                width: free_rect.width,
                height: used_rect.y - free_rect.y,
            });
        }

        // Bottom area
        if used_rect.y + used_rect.height < free_rect.y + free_rect.height {
            new_rects.push(Rectangle {
                x: free_rect.x,
                y: used_rect.y + used_rect.height,
                width: free_rect.width,
                height: free_rect.y + free_rect.height - (used_rect.y + used_rect.height),
            });
        }

        // Left area
        if used_rect.x > free_rect.x && used_rect.x < free_rect.x + free_rect.width {
            new_rects.push(Rectangle {
                x: free_rect.x,
                y: free_rect.y,
                width: used_rect.x - free_rect.x,
                height: free_rect.height,
            });
        }

        // Right area
        if used_rect.x + used_rect.width < free_rect.x + free_rect.width {
            new_rects.push(Rectangle {
                x: used_rect.x + used_rect.width,
                y: free_rect.y,
                width: free_rect.x + free_rect.width - (used_rect.x + used_rect.width),
                height: free_rect.height,
            });
        }

        // Add all new rectangles to the free list
        for new_rect in new_rects {
            self.free_rects.push(new_rect);
        }

        true
    }

    /// Checks if two rectangles overlap
    fn is_overlapping(&self, a: Rectangle, b: Rectangle) -> bool {
        !(a.x + a.width <= b.x
            || a.x >= b.x + b.width
            || a.y + a.height <= b.y
            || a.y >= b.y + b.height)
    }

    /// Removes redundant rectangles from the free list
    fn prune_free_list(&mut self) {
        let mut i = 0;
        while i < self.free_rects.len() {
            let mut j = i + 1;
            while j < self.free_rects.len() {
                if self.is_contained_in(self.free_rects[i], self.free_rects[j]) {
                    self.free_rects.remove(i);
                    if i > 0 {
                        i -= 1;
                    }
                    break;
                } else if self.is_contained_in(self.free_rects[j], self.free_rects[i]) {
                    self.free_rects.remove(j);
                } else {
                    j += 1;
                }
            }
            i += 1;
        }
    }

    /// Checks if rectangle a is completely contained within rectangle b
    fn is_contained_in(&self, a: Rectangle, b: Rectangle) -> bool {
        a.x >= b.x
            && a.y >= b.y
            && a.x + a.width <= b.x + b.width
            && a.y + a.height <= b.y + b.height
    }
}

/// Adjusts dimensions to be multiples of GRID_SIZE (16 pixels)
///
/// This ensures proper alignment for texture coordinate calculations
/// in the RustPixel engine's grid-based system.
fn adjust_size_to_grid(width: u32, height: u32) -> (u32, u32) {
    let adjusted_width = ((width + config::GRID_SIZE - 1) / config::GRID_SIZE) * config::GRID_SIZE;
    let adjusted_height = ((height + config::GRID_SIZE - 1) / config::GRID_SIZE) * config::GRID_SIZE;
    (adjusted_width, adjusted_height)
}

/// Represents an image with its placement information in the atlas
struct ImageRect {
    /// Original filename of the image
    path: String,
    /// The processed image data
    image: DynamicImage,
    /// Rectangle defining the image's position in the atlas
    rect: Rectangle,
}

/// Command-line arguments
struct Args {
    input_folder: String,
    output_folder: String,
    region: PackingRegion,
    start_block: Option<u32>,  // None means auto-detect from symbol map
    scale_factor: f32,
    symbols_path: String,       // Path to symbols.png
    symbol_map_path: Option<String>,  // Optional path to symbol_map.json
}

/// Displays usage information and exits the program
fn print_usage_and_exit() -> ! {
    eprintln!("RustPixel Asset Packer (4096x4096 Texture Atlas)");
    eprintln!();
    eprintln!("USAGE:");
    eprintln!("    cargo pixel r asset t -r <INPUT_FOLDER> <OUTPUT_FOLDER> [OPTIONS]");
    eprintln!();
    eprintln!("ARGS:");
    eprintln!("    <INPUT_FOLDER>     Folder containing images to pack");
    eprintln!("    <OUTPUT_FOLDER>    Folder where output files will be written");
    eprintln!();
    eprintln!("OPTIONS:");
    eprintln!("    --symbols <PATH>   Path to base symbols.png texture");
    eprintln!("                       (default: assets/pix/symbols.png)");
    eprintln!("    --symbol-map <PATH>");
    eprintln!("                       Path to symbol_map.json for auto block detection");
    eprintln!("                       When provided, automatically selects first free block");
    eprintln!("    --region <MODE>    Packing region: 'sprite' (default) or 'full'");
    eprintln!("                       sprite: Pack into sprite region (y: 0-2559)");
    eprintln!("                       full: Pack into entire 4096x4096 atlas");
    eprintln!("    --start-block <N>  Start packing from block N (overrides auto-detect)");
    eprintln!("                       Blocks 0-159 are in sprite region");
    eprintln!("    --scale <FACTOR>   Scale factor for images (default: 1.0)");
    eprintln!("                       Use 0.5 to scale to half size");
    eprintln!();
    eprintln!("TEXTURE LAYOUT:");
    eprintln!("    ┌──────────────────────────────────────────────────┐");
    eprintln!("    │ SPRITE (y: 0-2559) - 160 blocks, 16x16px symbols │");
    eprintln!("    ├──────────────────────────────────────────────────┤");
    eprintln!("    │ TUI (x: 0-2559) + EMOJI (x: 2560-4095)          │");
    eprintln!("    │ y: 2560-3071, 16x32px / 32x32px symbols         │");
    eprintln!("    ├──────────────────────────────────────────────────┤");
    eprintln!("    │ CJK (y: 3072-4095) - 64 blocks, 32x32px symbols │");
    eprintln!("    └──────────────────────────────────────────────────┘");
    eprintln!();
    eprintln!("OUTPUT:");
    eprintln!("    texture_atlas.png  - Combined texture atlas (4096x4096)");
    eprintln!("    *.pix              - Metadata files for each input image");
    eprintln!();
    eprintln!("EXAMPLES:");
    eprintln!("    # Basic usage with auto block detection");
    eprintln!("    asset ./sprites ./output --symbol-map assets/pix/symbol_map.json");
    eprintln!();
    eprintln!("    # Specify both symbols.png and symbol_map.json");
    eprintln!("    asset ./sprites ./output --symbols assets/pix/symbols.png \\");
    eprintln!("          --symbol-map assets/pix/symbol_map.json");
    eprintln!();
    eprintln!("    # Manual start block (overrides auto-detect)");
    eprintln!("    asset ./icons ./output --start-block 80 --scale 0.5");
    eprintln!();
    eprintln!("    # Pack into full atlas");
    eprintln!("    asset ./images ./output --region full");

    process::exit(1);
}

/// Parse command-line arguments
fn parse_args() -> Args {
    let args: Vec<String> = env::args().collect();

    // Check for help argument
    if args.len() > 1 && (args[1] == "--help" || args[1] == "-h" || args[1] == "help") {
        print_usage_and_exit();
    }

    if args.len() < 3 {
        print_usage_and_exit();
    }

    let input_folder = args[1].clone();
    let output_folder = args[2].clone();
    let mut region = PackingRegion::Sprite;
    let mut start_block: Option<u32> = None;
    let mut scale_factor = 1.0f32;
    let mut symbols_path = config::SYMBOLS_TEXTURE_PATH.to_string();
    let mut symbol_map_path: Option<String> = None;

    // Parse optional arguments
    let mut i = 3;
    while i < args.len() {
        match args[i].as_str() {
            "--symbols" => {
                if i + 1 < args.len() {
                    symbols_path = args[i + 1].clone();
                    i += 2;
                } else {
                    eprintln!("Error: --symbols requires a path");
                    process::exit(1);
                }
            }
            "--symbol-map" => {
                if i + 1 < args.len() {
                    symbol_map_path = Some(args[i + 1].clone());
                    i += 2;
                } else {
                    eprintln!("Error: --symbol-map requires a path");
                    process::exit(1);
                }
            }
            "--region" => {
                if i + 1 < args.len() {
                    region = PackingRegion::from_str(&args[i + 1])
                        .unwrap_or_else(|| {
                            eprintln!("Error: Invalid region '{}'. Use 'sprite' or 'full'.", args[i + 1]);
                            process::exit(1);
                        });
                    i += 2;
                } else {
                    eprintln!("Error: --region requires a value");
                    process::exit(1);
                }
            }
            "--start-block" => {
                if i + 1 < args.len() {
                    start_block = Some(args[i + 1].parse().unwrap_or_else(|_| {
                        eprintln!("Error: Invalid block number '{}'", args[i + 1]);
                        process::exit(1);
                    }));
                    i += 2;
                } else {
                    eprintln!("Error: --start-block requires a value");
                    process::exit(1);
                }
            }
            "--scale" => {
                if i + 1 < args.len() {
                    scale_factor = args[i + 1].parse().unwrap_or_else(|_| {
                        eprintln!("Error: Invalid scale factor '{}'", args[i + 1]);
                        process::exit(1);
                    });
                    i += 2;
                } else {
                    eprintln!("Error: --scale requires a value");
                    process::exit(1);
                }
            }
            _ => {
                eprintln!("Warning: Unknown option '{}'", args[i]);
                i += 1;
            }
        }
    }

    Args {
        input_folder,
        output_folder,
        region,
        start_block,
        scale_factor,
        symbols_path,
        symbol_map_path,
    }
}

/// Loads and processes images from the input folder
fn load_images_from_folder(folder_path: &str) -> Result<Vec<(String, DynamicImage)>, String> {
    let paths = fs::read_dir(folder_path)
        .map_err(|e| format!("Failed to read directory '{}': {}", folder_path, e))?;

    let mut images = Vec::new();

    for path in paths {
        let file_path = path
            .map_err(|e| format!("Error reading directory entry: {}", e))?
            .path();

        if file_path.is_file() {
            // Check if it's an image file
            let ext = file_path.extension()
                .and_then(|e| e.to_str())
                .map(|e| e.to_lowercase());

            match ext.as_deref() {
                Some("png") | Some("jpg") | Some("jpeg") | Some("bmp") | Some("gif") => {
                    let file_name = file_path
                        .file_name()
                        .and_then(|n| n.to_str())
                        .ok_or_else(|| format!("Invalid filename: {:?}", file_path))?
                        .to_string();

                    println!("Processing: {}", file_path.display());

                    match image::open(&file_path) {
                        Ok(img) => images.push((file_name, img)),
                        Err(e) => eprintln!("Warning: Failed to load image '{}': {}", file_path.display(), e),
                    }
                }
                _ => {
                    // Skip non-image files silently
                }
            }
        }
    }

    if images.is_empty() {
        return Err(format!("No valid images found in folder '{}'", folder_path));
    }

    Ok(images)
}

/// Processes and packs images into the atlas
fn pack_images(images: Vec<(String, DynamicImage)>, bin: &mut MaxRectsBin, scale_factor: f32) -> Vec<ImageRect> {
    let mut image_rects = Vec::new();

    for (filename, img) in images {
        let (orig_width, orig_height) = img.dimensions();
        let (adjusted_width, adjusted_height) = adjust_size_to_grid(orig_width, orig_height);

        // Pad image to adjusted size if necessary
        let padded_image = if adjusted_width != orig_width || adjusted_height != orig_height {
            let mut padded_image = DynamicImage::new_rgba8(adjusted_width, adjusted_height);
            padded_image.copy_from(&img, 0, 0)
                .expect("Failed to copy image to padded buffer");
            padded_image
        } else {
            img
        };

        // Apply scale factor
        let final_width = ((adjusted_width as f32 * scale_factor) as u32).max(config::GRID_SIZE);
        let final_height = ((adjusted_height as f32 * scale_factor) as u32).max(config::GRID_SIZE);

        // Ensure dimensions are multiples of GRID_SIZE
        let (final_width, final_height) = adjust_size_to_grid(final_width, final_height);

        let final_image = if scale_factor != 1.0 {
            padded_image.resize_exact(
                final_width,
                final_height,
                FilterType::Lanczos3,
            )
        } else {
            padded_image
        };

        // Try to pack the image
        match bin.insert(final_width, final_height) {
            Some(rect) => {
                image_rects.push(ImageRect {
                    path: filename.clone(),
                    image: final_image,
                    rect,
                });
                println!("  Packed '{}' at ({}, {}) size {}x{}",
                    filename, rect.x, rect.y, rect.width, rect.height);
            }
            None => {
                eprintln!("Warning: No space available for image '{}'", filename);
            }
        }
    }

    image_rects
}

/// Creates the texture atlas with all packed images
fn create_texture_atlas(image_rects: &[ImageRect], symbols_texture_path: &str) -> Result<RgbaImage, String> {
    // Create the atlas canvas (4096x4096)
    let mut atlas = RgbaImage::new(config::ATLAS_WIDTH, config::ATLAS_HEIGHT);

    // Try to load the base symbols texture if it exists
    if let Ok(base_texture) = image::open(symbols_texture_path) {
        let (base_w, base_h) = base_texture.dimensions();
        println!("Loaded base texture: {}x{}", base_w, base_h);

        // Copy the base texture
        if let Err(e) = atlas.copy_from(&base_texture, 0, 0) {
            eprintln!("Warning: Failed to copy base texture: {}", e);
        }
    } else {
        println!("Note: Base texture not found at '{}', creating blank atlas", symbols_texture_path);
    }

    // Copy all packed images to their positions
    for image_rect in image_rects {
        atlas.copy_from(
            &image_rect.image,
            image_rect.rect.x,
            image_rect.rect.y
        ).map_err(|e| format!("Failed to copy image '{}' to atlas: {}", image_rect.path, e))?;
    }

    Ok(atlas)
}

/// Generates .pix metadata files for each packed image
///
/// PIX file format for 4096x4096 texture:
/// - Header: width=W,height=H,texture=255
/// - Each cell: symidx,color,texidx,modifier
///   - symidx: symbol index within block (0-255)
///   - texidx: block index (0-159 for sprite region)
fn generate_pix_files(image_rects: &[ImageRect], output_dir: &str) -> Result<(), String> {
    // Create output directory if it doesn't exist
    fs::create_dir_all(output_dir)
        .map_err(|e| format!("Failed to create output directory '{}': {}", output_dir, e))?;

    for image_rect in image_rects {
        // Calculate grid coordinates
        let x0 = image_rect.rect.x / config::GRID_SIZE;
        let y0 = image_rect.rect.y / config::GRID_SIZE;
        let w = image_rect.rect.width / config::GRID_SIZE;
        let h = image_rect.rect.height / config::GRID_SIZE;

        // Create output file path
        let output_path = Path::new(output_dir)
            .join(&image_rect.path)
            .with_extension("pix");

        let mut file = File::create(&output_path)
            .map_err(|e| format!("Failed to create file '{}': {}", output_path.display(), e))?;

        // Write header with dimensions and texture ID
        writeln!(file, "width={},height={},texture=255", w, h)
            .map_err(|e| format!("Failed to write header to '{}': {}", output_path.display(), e))?;

        // Write texture coordinate data
        // For 4096x4096 with 16x16 base symbols:
        // - Grid is 256x256 cells
        // - Block is 16x16 cells (256x256 pixels)
        // - Block index = (y / 16) * 16 + (x / 16)
        // - Symbol index within block = (y % 16) * 16 + (x % 16)
        for row in 0..h {
            for col in 0..w {
                let x = x0 + col;
                let y = y0 + row;

                // Calculate block index (0-255 in 16x16 grid of blocks)
                let block_col = x / 16;
                let block_row = y / 16;
                let texidx = block_row * 16 + block_col;

                // Calculate symbol index within block (0-255)
                let sym_col = x % 16;
                let sym_row = y % 16;
                let symidx = sym_row * 16 + sym_col;

                // Format: symidx, color, texidx, modifier
                write!(file, "{},{},{},{} ", symidx, 15, texidx, 0)
                    .map_err(|e| format!("Failed to write coordinates to '{}': {}", output_path.display(), e))?;
            }
            writeln!(file)
                .map_err(|e| format!("Failed to write newline to '{}': {}", output_path.display(), e))?;
        }

        println!("Generated: {}", output_path.display());
    }

    Ok(())
}

/// Main entry point for the asset packer
fn main() {
    let args = parse_args();

    println!("RustPixel Asset Packer (4096x4096)");
    println!("══════════════════════════════════");
    println!("Input folder:  {}", args.input_folder);
    println!("Output folder: {}", args.output_folder);
    println!("Symbols:       {}", args.symbols_path);
    if let Some(ref map_path) = args.symbol_map_path {
        println!("Symbol map:    {}", map_path);
    }
    println!("Region:        {:?}", args.region);
    println!("Scale factor:  {}", args.scale_factor);
    println!();

    // Parse symbol map if provided to detect occupied blocks
    let occupancy = if let Some(ref map_path) = args.symbol_map_path {
        match parse_symbol_map(map_path) {
            Ok(occ) => {
                occ.print_summary();
                println!();
                Some(occ)
            }
            Err(e) => {
                eprintln!("Warning: {}", e);
                eprintln!("Continuing without block occupancy detection...");
                println!();
                None
            }
        }
    } else {
        None
    };

    // Determine start block: use explicit arg, or auto-detect from symbol map, or default to 0
    let start_block = match args.start_block {
        Some(block) => {
            println!("Using explicit start block: {}", block);
            block
        }
        None => {
            if let Some(ref occ) = occupancy {
                let auto_block = occ.first_free_sprite_block();
                println!("Auto-detected start block: {} (first free block)", auto_block);
                auto_block
            } else {
                println!("Using default start block: 0");
                0
            }
        }
    };
    println!();

    // Load images from input folder
    let images = match load_images_from_folder(&args.input_folder) {
        Ok(images) => images,
        Err(e) => {
            eprintln!("Error: {}", e);
            process::exit(1);
        }
    };

    println!("Loaded {} images", images.len());
    println!();

    // Initialize the bin packing algorithm based on region
    let packing_region = match args.region {
        PackingRegion::Sprite => layout::sprite_packing_region(start_block),
        PackingRegion::Full => (0, 0, config::ATLAS_WIDTH, config::ATLAS_HEIGHT),
    };

    println!("Packing region: x={}, y={}, w={}, h={}",
        packing_region.0, packing_region.1, packing_region.2, packing_region.3);
    println!();

    let mut bin = MaxRectsBin::from_region(packing_region);

    // Pack all images
    let image_rects = pack_images(images, &mut bin, args.scale_factor);

    if image_rects.is_empty() {
        eprintln!("Error: No images could be packed");
        process::exit(1);
    }

    println!();
    println!("Successfully packed {} images", image_rects.len());

    // Create the texture atlas using specified symbols path
    let atlas = match create_texture_atlas(&image_rects, &args.symbols_path) {
        Ok(atlas) => atlas,
        Err(e) => {
            eprintln!("Error: {}", e);
            process::exit(1);
        }
    };

    // Create output directory if needed
    if let Err(e) = fs::create_dir_all(&args.output_folder) {
        eprintln!("Error: Failed to create output directory: {}", e);
        process::exit(1);
    }

    // Save the texture atlas
    let atlas_path = format!("{}/texture_atlas.png", args.output_folder);
    if let Err(e) = atlas.save(&atlas_path) {
        eprintln!("Error: Failed to save texture atlas '{}': {}", atlas_path, e);
        process::exit(1);
    }
    println!("Saved texture atlas: {}", atlas_path);

    // Generate .pix files
    if let Err(e) = generate_pix_files(&image_rects, &args.output_folder) {
        eprintln!("Error: {}", e);
        process::exit(1);
    }

    println!();
    println!("════════════════════════════════════════════════════");
    println!("Asset packing completed successfully!");
    println!("────────────────────────────────────────────────────");
    println!("• Generated {} .pix files", image_rects.len());
    println!("• Texture atlas: {} (4096x4096)", atlas_path);
    println!("• Blocks used: starting from block {}", start_block);
    if let Some(occ) = occupancy {
        let remaining = layout::SPRITE_BLOCKS - occ.occupied_sprite_blocks.len() as u32;
        println!("• Remaining sprite blocks: {}/{}", remaining, layout::SPRITE_BLOCKS);
    }
}
