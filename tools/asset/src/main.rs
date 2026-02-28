//! # RustPixel Asset Packer
//!
//! A command-line tool for packing images into a texture atlas and generating
//! corresponding `.pix` files for use with the RustPixel engine.
//!
//! ## Packing Modes
//!
//! - **Linear (default)**: Slices images into 16×16 cells and fills blocks sequentially.
//!   Existing PETSCII/TUI/emoji/CJK content is preserved. New cells are appended
//!   starting from the first free block.
//! - **BinPack**: Uses MaxRects bin packing to fit whole images as rectangles.
//!
//! ## 4096×4096 Texture Layout
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
//! # Linear mode (default): slice images into 16×16 cells, fill blocks sequentially
//! cargo pixel r asset t -r <input_folder> <output_folder> --symbol-map assets/pix/symbol_map.json
//!
//! # BinPack mode: pack whole images using MaxRects algorithm
//! cargo pixel r asset t -r <input_folder> <output_folder> --mode binpack
//! ```
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
    pub const SYMBOL_HEIGHT: u32 = 16;

}

/// Texture region layout constants (matching symbol_map.rs)
mod layout {
    use super::config::*;

    // Sprite region: blocks 0-159 (10 rows × 16 cols)
    pub const SPRITE_BLOCK_ROWS: u32 = 10;
    pub const SPRITE_BLOCK_COLS: u32 = 16;
    pub const SPRITE_BLOCKS: u32 = SPRITE_BLOCK_ROWS * SPRITE_BLOCK_COLS; // 160
    pub const SPRITE_Y_END: u32 = SPRITE_BLOCK_ROWS * 16 * SYMBOL_HEIGHT; // 2560

    // Block dimensions
    pub const BLOCK_HEIGHT: u32 = 16 * SYMBOL_HEIGHT; // 256

    /// Get packing region for sprite blocks (starting from a specific block)
    /// Returns (x, y, width, height) of available packing area
    pub fn sprite_packing_region(start_block: u32) -> (u32, u32, u32, u32) {
        let block_row = start_block / SPRITE_BLOCK_COLS;
        let y_start = block_row * BLOCK_HEIGHT;
        let height = SPRITE_Y_END - y_start;
        (0, y_start, ATLAS_WIDTH, height)
    }

}

// ============================================================================
// Symbol Map JSON Structures
// ============================================================================

/// Represents a region in the symbol map
#[derive(Debug, Deserialize)]
#[allow(dead_code)]
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

/// Parses symbol_map.json for region stats display
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
    }

    // Sort stats by start_block for display
    occupancy.region_stats.sort_by_key(|(_, start, _, _)| *start);

    Ok(occupancy)
}

/// Scan the base texture to detect which sprite blocks have non-transparent content.
/// A block is considered occupied if ANY pixel within it has alpha > 0.
fn detect_occupied_blocks_from_texture(atlas: &RgbaImage) -> BlockOccupancy {
    let mut occupancy = BlockOccupancy::default();

    for block_idx in 0..layout::SPRITE_BLOCKS {
        let block_col = block_idx % 16;
        let block_row = block_idx / 16;
        let bx = block_col * 256;
        let by = block_row * 256;

        let mut has_content = false;
        'scan: for py in 0..256u32 {
            for px in 0..256u32 {
                let x = bx + px;
                let y = by + py;
                if x < atlas.width() && y < atlas.height() {
                    let pixel = atlas.get_pixel(x, y);
                    if pixel[3] > 0 {
                        has_content = true;
                        break 'scan;
                    }
                }
            }
        }

        if has_content {
            occupancy.occupied_sprite_blocks.insert(block_idx);
        }
    }

    occupancy.total_sprite_symbols = occupancy.occupied_sprite_blocks.len() * 256;
    occupancy
}

/// Packing mode
#[derive(Clone, Copy, Debug, PartialEq)]
enum PackingMode {
    /// Slice images into 16×16 cells, fill blocks linearly (default)
    Linear,
    /// MaxRects bin packing (whole images)
    BinPack,
}

/// Packing region mode (for binpack mode)
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

// ============================================================================
// Linear Packing
// ============================================================================

/// Linear packer: fills blocks sequentially with 16×16 cells
struct LinearPacker {
    current_block: u32,
    current_sym: u32, // 0-255 within current block
    max_block: u32,   // exclusive upper bound (160 for sprite region)
}

impl LinearPacker {
    fn new(start_block: u32) -> Self {
        Self {
            current_block: start_block,
            current_sym: 0,
            max_block: layout::SPRITE_BLOCKS,
        }
    }

    /// Get next available (block, sym) slot, advancing the cursor
    fn next_slot(&mut self) -> Option<(u32, u32)> {
        if self.current_block >= self.max_block {
            return None;
        }
        let result = (self.current_block, self.current_sym);
        self.current_sym += 1;
        if self.current_sym >= 256 {
            self.current_sym = 0;
            self.current_block += 1;
        }
        Some(result)
    }

    /// Convert (block, sym) to pixel coordinates in the 4096×4096 atlas
    fn to_pixel(block: u32, sym: u32) -> (u32, u32) {
        let block_col = block % 16;
        let block_row = block / 16;
        let sym_col = sym % 16;
        let sym_row = sym / 16;
        let px = block_col * 256 + sym_col * 16;
        let py = block_row * 256 + sym_row * 16;
        (px, py)
    }

    /// How many blocks have been (partially) used
    fn blocks_used(&self, start_block: u32) -> u32 {
        if self.current_sym > 0 {
            self.current_block - start_block + 1
        } else {
            self.current_block - start_block
        }
    }
}

/// Result of linearly packing one image
struct LinearImageResult {
    /// Original filename
    path: String,
    /// Image width in cells
    width_cells: u32,
    /// Image height in cells
    height_cells: u32,
    /// Per-cell (block_idx, sym_idx) mapping, row-major: [row][col]
    cell_mappings: Vec<Vec<(u32, u32)>>,
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
    mode: PackingMode,
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
    eprintln!("    cargo pixel r asset t -r -- <INPUT_FOLDER> <OUTPUT_FOLDER> [OPTIONS]");
    eprintln!();
    eprintln!("ARGS:");
    eprintln!("    <INPUT_FOLDER>     Folder containing images to pack");
    eprintln!("    <OUTPUT_FOLDER>    Folder where output files will be written");
    eprintln!();
    eprintln!("OPTIONS:");
    eprintln!("    --mode <MODE>      Packing mode: 'linear' (default) or 'binpack'");
    eprintln!("                       linear: Slice images into 16x16 cells, fill blocks");
    eprintln!("                               sequentially. Preserves existing content.");
    eprintln!("                       binpack: MaxRects bin packing (whole images).");
    eprintln!("    --symbols <PATH>   Path to base symbols.png texture");
    eprintln!("                       (default: assets/pix/symbols.png)");
    eprintln!("    --symbol-map <PATH>");
    eprintln!("                       Path to symbol_map.json for auto block detection");
    eprintln!("                       When provided, automatically selects first free block");
    eprintln!("    --region <MODE>    (binpack only) Packing region: 'sprite' or 'full'");
    eprintln!("    --start-block <N>  Start packing from block N (overrides auto-detect)");
    eprintln!("                       Blocks 0-159 are in sprite region");
    eprintln!("    --scale <FACTOR>   Scale factor for images (default: 1.0)");
    eprintln!();
    eprintln!("EXAMPLES:");
    eprintln!("    # Linear mode (default): slice and fill blocks sequentially");
    eprintln!("    asset ./sprites ./output --symbol-map assets/pix/symbol_map.json");
    eprintln!();
    eprintln!("    # Linear mode with explicit start block");
    eprintln!("    asset ./sprites ./output --start-block 4");
    eprintln!();
    eprintln!("    # BinPack mode: whole-image rectangle packing");
    eprintln!("    asset ./sprites ./output --mode binpack --start-block 4");

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
    let mut mode = PackingMode::Linear;
    let mut region = PackingRegion::Sprite;
    let mut start_block: Option<u32> = None;
    let mut scale_factor = 1.0f32;
    let mut symbols_path = config::SYMBOLS_TEXTURE_PATH.to_string();
    let mut symbol_map_path: Option<String> = None;

    // Parse optional arguments
    let mut i = 3;
    while i < args.len() {
        match args[i].as_str() {
            "--mode" => {
                if i + 1 < args.len() {
                    mode = match args[i + 1].to_lowercase().as_str() {
                        "linear" => PackingMode::Linear,
                        "binpack" => PackingMode::BinPack,
                        _ => {
                            eprintln!("Error: Invalid mode '{}'. Use 'linear' or 'binpack'.", args[i + 1]);
                            process::exit(1);
                        }
                    };
                    i += 2;
                } else {
                    eprintln!("Error: --mode requires a value");
                    process::exit(1);
                }
            }
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
        mode,
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

/// Linear packing: slice images into 16×16 cells, fill atlas blocks sequentially
fn linear_pack_images(
    images: Vec<(String, DynamicImage)>,
    packer: &mut LinearPacker,
    atlas: &mut RgbaImage,
    scale_factor: f32,
) -> Vec<LinearImageResult> {
    let mut results = Vec::new();

    for (filename, img) in images {
        let (orig_width, orig_height) = img.dimensions();

        // Apply scale factor first
        let scaled = if scale_factor != 1.0 {
            let sw = ((orig_width as f32 * scale_factor) as u32).max(config::GRID_SIZE);
            let sh = ((orig_height as f32 * scale_factor) as u32).max(config::GRID_SIZE);
            let (sw, sh) = adjust_size_to_grid(sw, sh);
            img.resize_exact(sw, sh, FilterType::Lanczos3)
        } else {
            img
        };

        let (w, h) = scaled.dimensions();
        let (aw, ah) = adjust_size_to_grid(w, h);

        // Pad to grid multiples if needed
        let padded = if aw != w || ah != h {
            let mut p = DynamicImage::new_rgba8(aw, ah);
            p.copy_from(&scaled, 0, 0).expect("Failed to pad image");
            p
        } else {
            scaled
        };

        let cells_w = aw / 16;
        let cells_h = ah / 16;
        let mut cell_mappings: Vec<Vec<(u32, u32)>> = Vec::new();
        let mut ok = true;

        for row in 0..cells_h {
            let mut row_map = Vec::new();
            for col in 0..cells_w {
                if let Some((block, sym)) = packer.next_slot() {
                    // Copy 16×16 cell from source to atlas
                    let src_x = col * 16;
                    let src_y = row * 16;
                    let (dst_x, dst_y) = LinearPacker::to_pixel(block, sym);

                    for py in 0..16 {
                        for px in 0..16 {
                            let pixel = padded.get_pixel(src_x + px, src_y + py);
                            atlas.put_pixel(dst_x + px, dst_y + py, pixel);
                        }
                    }

                    row_map.push((block, sym));
                } else {
                    eprintln!("Warning: No space for '{}' (ran out of blocks)", filename);
                    ok = false;
                    break;
                }
            }
            if !ok {
                break;
            }
            cell_mappings.push(row_map);
        }

        if ok {
            println!(
                "  Packed '{}': {}x{} -> {}x{} cells",
                filename, aw, ah, cells_w, cells_h
            );
            results.push(LinearImageResult {
                path: filename,
                width_cells: cells_w,
                height_cells: cells_h,
                cell_mappings,
            });
        }
    }

    results
}

/// Generate .pix files for linearly-packed images
fn generate_linear_pix_files(
    results: &[LinearImageResult],
    output_dir: &str,
) -> Result<(), String> {
    fs::create_dir_all(output_dir)
        .map_err(|e| format!("Failed to create output directory '{}': {}", output_dir, e))?;

    for result in results {
        let output_path = Path::new(output_dir)
            .join(&result.path)
            .with_extension("pix");

        let mut file = File::create(&output_path)
            .map_err(|e| format!("Failed to create file '{}': {}", output_path.display(), e))?;

        // Header
        writeln!(
            file,
            "width={},height={},texture=255",
            result.width_cells, result.height_cells
        )
        .map_err(|e| format!("Write error: {}", e))?;

        // Each cell: symidx,fgcolor,texidx,bgcolor
        for row in &result.cell_mappings {
            for &(block, sym) in row {
                write!(file, "{},{},{},{} ", sym, 15, block, 0)
                    .map_err(|e| format!("Write error: {}", e))?;
            }
            writeln!(file).map_err(|e| format!("Write error: {}", e))?;
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
    println!("Mode:          {:?}", args.mode);
    println!("Symbols:       {}", args.symbols_path);
    if let Some(ref map_path) = args.symbol_map_path {
        println!("Symbol map:    {}", map_path);
    }
    println!("Scale factor:  {}", args.scale_factor);
    println!();

    // Print symbol map info if provided
    if let Some(ref map_path) = args.symbol_map_path {
        match parse_symbol_map(map_path) {
            Ok(occ) => {
                occ.print_summary();
                println!();
            }
            Err(e) => {
                eprintln!("Warning: {}", e);
            }
        }
    }

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

    // Create output directory
    if let Err(e) = fs::create_dir_all(&args.output_folder) {
        eprintln!("Error: Failed to create output directory: {}", e);
        process::exit(1);
    }

    // Load base texture (used by both modes)
    let mut atlas = RgbaImage::new(config::ATLAS_WIDTH, config::ATLAS_HEIGHT);
    if let Ok(base) = image::open(&args.symbols_path) {
        let (bw, bh) = base.dimensions();
        println!("Loaded base texture: {}x{}", bw, bh);
        if let Err(e) = atlas.copy_from(&base, 0, 0) {
            eprintln!("Warning: Failed to copy base texture: {}", e);
        }
    } else {
        println!("Note: Base texture not found at '{}', creating blank atlas", args.symbols_path);
    }

    // Detect occupied blocks by scanning actual pixel content
    let start_block = match args.start_block {
        Some(block) => {
            println!("Using explicit start block: {}", block);
            block
        }
        None => {
            let tex_occ = detect_occupied_blocks_from_texture(&atlas);
            let auto_block = tex_occ.first_free_sprite_block();
            println!(
                "Scanned texture: {}/{} sprite blocks occupied (non-transparent pixels)",
                tex_occ.occupied_sprite_blocks.len(),
                layout::SPRITE_BLOCKS
            );
            if !tex_occ.occupied_sprite_blocks.is_empty() {
                let mut blocks: Vec<u32> = tex_occ.occupied_sprite_blocks.iter().copied().collect();
                blocks.sort();
                println!("  Occupied blocks: {:?}", blocks);
            }
            println!("Auto-detected start block: {}", auto_block);
            auto_block
        }
    };
    println!();

    match args.mode {
        PackingMode::Linear => {
            // ── Linear mode: slice into 16×16 cells, fill blocks sequentially ──
            let mut packer = LinearPacker::new(start_block);
            let results = linear_pack_images(images, &mut packer, &mut atlas, args.scale_factor);

            if results.is_empty() {
                eprintln!("Error: No images could be packed");
                process::exit(1);
            }

            // Save atlas
            let atlas_path = format!("{}/texture_atlas.png", args.output_folder);
            if let Err(e) = atlas.save(&atlas_path) {
                eprintln!("Error: Failed to save texture atlas: {}", e);
                process::exit(1);
            }
            println!("Saved texture atlas: {}", atlas_path);

            // Generate .pix files
            if let Err(e) = generate_linear_pix_files(&results, &args.output_folder) {
                eprintln!("Error: {}", e);
                process::exit(1);
            }

            let total_cells: u32 = results.iter()
                .map(|r| r.width_cells * r.height_cells)
                .sum();

            println!();
            println!("════════════════════════════════════════════════════");
            println!("Linear packing completed!");
            println!("────────────────────────────────────────────────────");
            println!("• Generated {} .pix files", results.len());
            println!("• Total cells packed: {}", total_cells);
            println!("• Blocks used: {} (blocks {}-{})",
                packer.blocks_used(start_block),
                start_block,
                start_block + packer.blocks_used(start_block).saturating_sub(1));
            println!("• Texture atlas: {}", atlas_path);
            let remaining = layout::SPRITE_BLOCKS - start_block - packer.blocks_used(start_block);
            println!("• Remaining sprite blocks: {}/{}", remaining, layout::SPRITE_BLOCKS);
        }

        PackingMode::BinPack => {
            // ── BinPack mode: MaxRects rectangle packing ──
            println!("Region:        {:?}", args.region);

            let packing_region = match args.region {
                PackingRegion::Sprite => layout::sprite_packing_region(start_block),
                PackingRegion::Full => (0, 0, config::ATLAS_WIDTH, config::ATLAS_HEIGHT),
            };

            println!("Packing region: x={}, y={}, w={}, h={}",
                packing_region.0, packing_region.1, packing_region.2, packing_region.3);
            println!();

            let mut bin = MaxRectsBin::from_region(packing_region);
            let image_rects = pack_images(images, &mut bin, args.scale_factor);

            if image_rects.is_empty() {
                eprintln!("Error: No images could be packed");
                process::exit(1);
            }

            println!();
            println!("Successfully packed {} images", image_rects.len());

            // Copy packed images onto atlas (base already loaded)
            for image_rect in &image_rects {
                atlas.copy_from(
                    &image_rect.image,
                    image_rect.rect.x,
                    image_rect.rect.y
                ).unwrap_or_else(|e| eprintln!("Warning: {}", e));
            }

            let atlas_path = format!("{}/texture_atlas.png", args.output_folder);
            if let Err(e) = atlas.save(&atlas_path) {
                eprintln!("Error: Failed to save texture atlas: {}", e);
                process::exit(1);
            }
            println!("Saved texture atlas: {}", atlas_path);

            if let Err(e) = generate_pix_files(&image_rects, &args.output_folder) {
                eprintln!("Error: {}", e);
                process::exit(1);
            }

            println!();
            println!("════════════════════════════════════════════════════");
            println!("BinPack completed!");
            println!("────────────────────────────────────────────────────");
            println!("• Generated {} .pix files", image_rects.len());
            println!("• Texture atlas: {} (4096x4096)", atlas_path);
            println!("• Blocks used: starting from block {}", start_block);
        }
    }
}
