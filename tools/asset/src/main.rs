//! # RustPixel Asset Packer
//!
//! A command-line tool for packing multiple images into a texture atlas and generating
//! corresponding `.pix` files for use with the RustPixel engine.
//!
//! ## Features
//!
//! - **Image Packing**: Uses MaxRects bin packing algorithm for efficient space utilization
//! - **Size Optimization**: Automatically adjusts image sizes to multiples of 8 pixels
//! - **Texture Atlas Generation**: Creates a single texture atlas from multiple images
//! - **PIX File Generation**: Generates `.pix` metadata files for each packed image
//! - **Symbol Integration**: Integrates with existing symbol textures
//!
//! ## Usage
//!
//! ```bash
//! cargo run --bin asset <input_folder> <output_folder>
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
use std::env;
use std::fs::{self, File};
use std::io::Write;
use std::path::Path;
use std::process;

/// Application configuration and constants
mod config {
    /// Default texture atlas width in pixels
    pub const ATLAS_WIDTH: u32 = 1024;
    /// Default texture atlas height in pixels (excluding symbols area)
    pub const ATLAS_HEIGHT: u32 = 1024 - 128;
    /// Symbol texture height offset
    pub const SYMBOL_HEIGHT_OFFSET: u32 = 128;
    /// Grid size for texture coordinate calculations
    pub const GRID_SIZE: u32 = 8;
    /// Path to the base symbols texture
    pub const SYMBOLS_TEXTURE_PATH: &str = "assets/pix/symbols.png";
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
}

impl MaxRectsBin {
    /// Creates a new bin with the specified dimensions
    ///
    /// # Arguments
    /// * `width` - Total width of the packing area
    /// * `height` - Total height of the packing area
    ///
    /// # Returns
    /// A new MaxRectsBin instance with a single free rectangle covering the entire area
    fn new(width: u32, height: u32) -> Self {
        let initial_rect = Rectangle {
            x: 0,
            y: 0,
            width,
            height,
        };
        MaxRectsBin {
            free_rects: vec![initial_rect],
            used_rects: Vec::new(),
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
            Some(new_node)
        } else {
            None
        }
    }

    /// Finds the best position for a new rectangle using best-area-fit heuristic
    ///
    /// This method finds the free rectangle that would have the smallest leftover area
    /// after placing the new rectangle.
    ///
    /// # Arguments
    /// * `width` - Width of the rectangle to place
    /// * `height` - Height of the rectangle to place
    ///
    /// # Returns
    /// The best position found, or None if no suitable position exists
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
    ///
    /// # Arguments
    /// * `rect` - The rectangle to place in the bin
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
    ///
    /// When a rectangle is placed, any overlapping free rectangles need to be split
    /// into smaller non-overlapping rectangles.
    ///
    /// # Arguments
    /// * `free_rect` - The free rectangle to potentially split
    /// * `used_rect` - The rectangle that was just placed
    ///
    /// # Returns
    /// true if the free rectangle was split (and should be removed from the list)
    fn split_free_node(&mut self, free_rect: Rectangle, used_rect: Rectangle) -> bool {
        // If rectangles don't overlap, no splitting needed
        if !self.is_overlapping(free_rect, used_rect) {
            return false;
        }

        let mut new_rects = Vec::new();

        // Create new rectangles for each non-overlapping area

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
    ///
    /// # Arguments
    /// * `a` - First rectangle
    /// * `b` - Second rectangle
    ///
    /// # Returns
    /// true if the rectangles overlap, false otherwise
    fn is_overlapping(&self, a: Rectangle, b: Rectangle) -> bool {
        !(a.x + a.width <= b.x
            || a.x >= b.x + b.width
            || a.y + a.height <= b.y
            || a.y >= b.y + b.height)
    }

    /// Removes redundant rectangles from the free list
    ///
    /// This method removes any free rectangles that are completely contained
    /// within other free rectangles, as they are redundant.
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
    ///
    /// # Arguments
    /// * `a` - Rectangle to check if contained
    /// * `b` - Container rectangle
    ///
    /// # Returns
    /// true if a is completely within b, false otherwise
    fn is_contained_in(&self, a: Rectangle, b: Rectangle) -> bool {
        a.x >= b.x
            && a.y >= b.y
            && a.x + a.width <= b.x + b.width
            && a.y + a.height <= b.y + b.height
    }
}

/// Adjusts dimensions to be multiples of 8 pixels
///
/// This ensures proper alignment for texture coordinate calculations
/// in the RustPixel engine's grid-based system.
///
/// # Arguments
/// * `width` - Original width
/// * `height` - Original height
///
/// # Returns
/// Tuple of (adjusted_width, adjusted_height) where both are multiples of 8
fn adjust_size_to_multiple_of_eight(width: u32, height: u32) -> (u32, u32) {
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

/// Displays usage information and exits the program
fn print_usage_and_exit() -> ! {
    eprintln!("RustPixel Asset Packer");
    eprintln!();
    eprintln!("USAGE:");
    eprintln!("    cargo pixel r asset t -r <INPUT_FOLDER> <OUTPUT_FOLDER>");
    eprintln!();
    eprintln!("ARGS:");
    eprintln!("    <INPUT_FOLDER>     Folder containing images to pack");
    eprintln!("    <OUTPUT_FOLDER>    Folder where output files will be written");
    eprintln!();
    eprintln!("DESCRIPTION:");
    eprintln!("    Packs multiple images into a texture atlas and generates .pix files");
    eprintln!("    for use with the RustPixel engine. Images are automatically resized");
    eprintln!("    and positioned for optimal packing efficiency.");
    eprintln!();
    eprintln!("OUTPUT:");
    eprintln!("    texture_atlas.png  - Combined texture atlas");
    eprintln!("    *.pix             - Metadata files for each input image");
    eprintln!();
    eprintln!("EXAMPLES:");
    eprintln!("    asset ./input_images ./output");
    eprintln!("    asset sprites/ assets/");
    
    process::exit(1);
}

/// Loads and processes images from the input folder
///
/// # Arguments
/// * `folder_path` - Path to the folder containing images
///
/// # Returns
/// Vector of (filename, image) tuples, or error message
fn load_images_from_folder(folder_path: &str) -> Result<Vec<(String, DynamicImage)>, String> {
    let paths = fs::read_dir(folder_path)
        .map_err(|e| format!("Failed to read directory '{}': {}", folder_path, e))?;

    let mut images = Vec::new();
    
    for path in paths {
        let file_path = path
            .map_err(|e| format!("Error reading directory entry: {}", e))?
            .path();
            
        if file_path.is_file() {
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
    }
    
    if images.is_empty() {
        return Err(format!("No valid images found in folder '{}'", folder_path));
    }
    
    Ok(images)
}

/// Processes and packs images into the atlas
///
/// # Arguments
/// * `images` - Vector of (filename, image) tuples
/// * `bin` - MaxRects bin for packing
///
/// # Returns
/// Vector of ImageRect structs with placement information
fn pack_images(images: Vec<(String, DynamicImage)>, bin: &mut MaxRectsBin) -> Vec<ImageRect> {
    let mut image_rects = Vec::new();
    
    for (filename, img) in images {
        let (orig_width, orig_height) = img.dimensions();
        let (adjusted_width, adjusted_height) = adjust_size_to_multiple_of_eight(orig_width, orig_height);

        // Pad image to adjusted size if necessary
        let padded_image = if adjusted_width != orig_width || adjusted_height != orig_height {
            let mut padded_image = DynamicImage::new_rgba8(adjusted_width, adjusted_height);
            padded_image.copy_from(&img, 0, 0)
                .expect("Failed to copy image to padded buffer");
            padded_image
        } else {
            img
        };

        // Resize to half size for better packing efficiency
        let final_image = padded_image.resize_exact(
            adjusted_width / 2,
            adjusted_height / 2,
            FilterType::Lanczos3,
        );

        // Try to pack the image
        match bin.insert(adjusted_width / 2, adjusted_height / 2) {
            Some(rect) => {
                image_rects.push(ImageRect {
                    path: filename,
                    image: final_image,
                    rect,
                });
                println!("  Packed at ({}, {}) with size {}x{}", 
                    rect.x, rect.y, rect.width, rect.height);
            }
            None => {
                eprintln!("Warning: No space available for image '{}'", filename);
            }
        }
    }
    
    image_rects
}

/// Creates the texture atlas with all packed images
///
/// # Arguments
/// * `image_rects` - Vector of packed images with position information
/// * `symbols_texture_path` - Path to the base symbols texture
///
/// # Returns
/// The combined texture atlas image
fn create_texture_atlas(image_rects: &[ImageRect], symbols_texture_path: &str) -> Result<RgbaImage, String> {
    // Load the base symbols texture
    let base_texture = image::open(symbols_texture_path)
        .map_err(|e| format!("Failed to load base texture '{}': {}", symbols_texture_path, e))?;

    // Create the atlas canvas
    let mut atlas = RgbaImage::new(
        config::ATLAS_WIDTH, 
        config::ATLAS_HEIGHT + config::SYMBOL_HEIGHT_OFFSET
    );
    
    // Copy the base texture (symbols) to the top
    atlas.copy_from(&base_texture, 0, 0)
        .map_err(|e| format!("Failed to copy base texture: {}", e))?;

    // Copy all packed images to their positions
    for image_rect in image_rects {
        atlas.copy_from(
            &image_rect.image, 
            image_rect.rect.x, 
            image_rect.rect.y + config::SYMBOL_HEIGHT_OFFSET
        ).map_err(|e| format!("Failed to copy image '{}' to atlas: {}", image_rect.path, e))?;
    }
    
    Ok(atlas)
}

/// Generates .pix metadata files for each packed image
///
/// # Arguments
/// * `image_rects` - Vector of packed images with position information
/// * `output_dir` - Output directory path
///
/// # Returns
/// Result indicating success or error message
fn generate_pix_files(image_rects: &[ImageRect], output_dir: &str) -> Result<(), String> {
    for image_rect in image_rects {
        // Calculate texture coordinates in grid units
        let x0 = image_rect.rect.x / config::GRID_SIZE;
        let y0 = image_rect.rect.y / config::GRID_SIZE;
        let w = image_rect.rect.width / config::GRID_SIZE;
        let h = image_rect.rect.height / config::GRID_SIZE;
        
        // Create output file path
        let output_path = Path::new(&format!("{}/{}", output_dir, image_rect.path))
            .with_extension("pix");
            
        let mut file = File::create(&output_path)
            .map_err(|e| format!("Failed to create file '{}': {}", output_path.display(), e))?;
        
        // Write header with dimensions and texture ID
        writeln!(file, "width={},height={},texture=255", w, h)
            .map_err(|e| format!("Failed to write header to '{}': {}", output_path.display(), e))?;

        // Write texture coordinate data
        for row in 0..h {
            for col in 0..w {
                let x = x0 + col;
                let y = y0 + row;
                
                // Calculate texture coordinates for the sprite system
                let s = (y % 16) * 16 + (x % 16);
                let t = (y / 16) * 8 + (x / 16);
                
                write!(file, "{},{},{},{} ", s, 15, t + 8, 0)
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
    let args: Vec<String> = env::args().collect();

    // Parse command line arguments
    let (input_folder, output_folder) = match args.len() {
        3 => (&args[1], &args[2]),
        _ => print_usage_and_exit(),
    };

    println!("RustPixel Asset Packer");
    println!("Input folder: {}", input_folder);
    println!("Output folder: {}", output_folder);
    println!();

    // Load images from input folder
    let images = match load_images_from_folder(input_folder) {
        Ok(images) => images,
        Err(e) => {
            eprintln!("Error: {}", e);
            process::exit(1);
        }
    };
    
    println!("Loaded {} images", images.len());

    // Initialize the bin packing algorithm
    let mut bin = MaxRectsBin::new(config::ATLAS_WIDTH, config::ATLAS_HEIGHT);
    
    // Pack all images
    let image_rects = pack_images(images, &mut bin);
    
    if image_rects.is_empty() {
        eprintln!("Error: No images could be packed");
        process::exit(1);
    }
    
    println!("Successfully packed {} images", image_rects.len());

    // Create the texture atlas
    let atlas = match create_texture_atlas(&image_rects, config::SYMBOLS_TEXTURE_PATH) {
        Ok(atlas) => atlas,
        Err(e) => {
            eprintln!("Error: {}", e);
            process::exit(1);
        }
    };

    // Save the texture atlas
    let atlas_path = format!("{}/texture_atlas.png", output_folder);
    if let Err(e) = atlas.save(&atlas_path) {
        eprintln!("Error: Failed to save texture atlas '{}': {}", atlas_path, e);
        process::exit(1);
    }
    println!("Saved texture atlas: {}", atlas_path);

    // Generate .pix files
    if let Err(e) = generate_pix_files(&image_rects, output_folder) {
        eprintln!("Error: {}", e);
        process::exit(1);
    }

    println!();
    println!("Asset packing completed successfully!");
    println!("Generated {} .pix files and 1 texture atlas", image_rects.len());
}
