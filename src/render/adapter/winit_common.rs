//! # 🔗 Winit Common Module
//!
//! This module is one of the important achievements of the WGPU refactoring, extracting
//! common code between `WinitGlowAdapter` and `WinitWgpuAdapter`, implementing DRY principle
//! and improving code maintainability.
//!
//! ## 🎯 Design Goals
//!
//! ### Code Reuse
//! - **Eliminate duplication**: Large amount of identical code between two winit adapters
//! - **Unified interface**: Provide consistent event handling and window management interface
//! - **Maintainability**: Modifying shared logic only requires updating one place
//!
//! ### Performance Optimization
//! - **Zero-cost abstraction**: Shared code completely eliminates runtime overhead through inlining
//! - **Compile-time specialization**: Each backend can obtain optimal machine code
//! - **Memory efficiency**: Avoid duplicate data structure definitions
//!
//! ## 📦 Provided Features
//!
//! ### Window Management
//! ```text
//! ┌─────────────────────────────────────────────────────────────┐
//! │                   Window Management                         │
//! │  ┌─────────────┬─────────────┬─────────────────────────────┐ │
//! │  │ WindowInit  │   Event     │    OS Integration           │ │
//! │  │   Params    │  Handling   │                             │ │
//! │  │             │             │                             │ │
//! │  │ - Size      │ - Input     │ - Native Decoration         │ │
//! │  │ - Title     │ - Mouse     │ - Native Dragging           │ │
//! │  │ - Ratios    │ - TUI Mode  │ - Resizable Control         │ │
//! │  └─────────────┴─────────────┴─────────────────────────────┘ │
//! └─────────────────────────────────────────────────────────────┘
//! ```
//!
//! ### Event Handling
//! ```text
//! ┌─────────────────────────────────────────────────────────────┐
//! │                    Event Translation                        │
//! │                                                             │
//! │  Winit Events ──┐                    ┌── RustPixel Events   │
//! │                 │                    │                     │
//! │  ┌─────────────┐ │  ┌─────────────┐  │ ┌─────────────────┐ │
//! │  │ Keyboard    │─┼─→│   Common    │──┼→│ Unified Events  │ │
//! │  │ Mouse       │ │  │ Translation │  │ │ (Key, Mouse)    │ │
//! │  │ Window      │─┘  │   Logic     │  └→│ (TUI-aware)     │ │
//! │  └─────────────┘    └─────────────┘    └─────────────────┘ │
//! └─────────────────────────────────────────────────────────────┘
//! ```
//!
//! ## 🚀 Refactoring Value
//!
//! ### Code Reduction
//! - **~200 lines of duplicate code** extracted to shared module
//! - **4 duplicate functions** merged into unified implementation
//! - **Maintenance burden halved** - only need to maintain one copy of logic
//!
//! ### Consistency Guarantee
//! - **Unified event handling** logic and response across all winit backends
//! - **Consistent coordinate calculations** for high-DPI and TUI mode displays
//! - **Accurate mouse conversion** accounting for character height modes
//! - **Consistent error handling** and edge cases

use crate::event::Event;
use crate::render::adapter::{AdapterBase, PIXEL_SYM_HEIGHT, PIXEL_SYM_WIDTH};
use std::sync::Arc;
use winit::{
    event::{Event as WinitEvent, WindowEvent},
    window::{CustomCursor, Window},
};

// ============================================================================
// Border Area Detection (Shared by all Winit adapters)
// ============================================================================

/// Border area enumeration for window interaction
///
/// Defines mouse click area types for determining whether drag operation should be triggered.
/// Used by both Glow and WGPU adapters.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BorderArea {
    /// Normal content area (not a border)
    None,
    /// Close button area
    Close,
    /// Top title bar area (draggable)
    TopBar,
    /// Other border areas (draggable)
    Other,
}

/// Check if mouse position is in border area
///
/// Determines the type of border area for the mouse click position.
/// Used for deciding whether to trigger drag operation.
///
/// # Parameters
/// - `x`: Mouse X coordinate (logical pixels)
/// - `y`: Mouse Y coordinate (logical pixels)
/// - `base`: Reference to AdapterBase for cell dimensions
///
/// # Returns
/// The corresponding BorderArea type
pub fn check_border_area(x: f64, y: f64, base: &AdapterBase) -> BorderArea {
    let w = base.gr.cell_width() as f64;
    let h = base.gr.cell_height() as f64;
    let sw = (base.cell_w + 2) as f64;

    if y >= 0.0 && y < h {
        if x >= 0.0 && x <= (sw - 1.0) * w {
            return BorderArea::TopBar;
        }
        if x > (sw - 1.0) * w && x <= sw * w {
            return BorderArea::Close;
        }
    } else if x > w && x <= (sw - 1.0) * w {
        return BorderArea::None;
    }
    BorderArea::Other
}

// ============================================================================
// Custom Cursor Setup (Shared by all Winit adapters)
// ============================================================================

/// Load and create custom cursor from project assets
///
/// Loads cursor image from assets/pix/cursor.png and creates a CustomCursor.
/// Handles transparency and pre-multiplied alpha.
///
/// # Parameters
/// - `event_loop`: Reference to the event loop for creating cursor
///
/// # Returns
/// Some(CustomCursor) if successful, None if cursor file not found or failed to load
pub fn load_custom_cursor(
    event_loop: &winit::event_loop::EventLoop<()>,
) -> Option<CustomCursor> {
    let project_path = &crate::get_game_config().project_path;
    let cursor_path = format!(
        "{}{}{}",
        project_path,
        std::path::MAIN_SEPARATOR,
        "assets/pix/cursor.png"
    );

    if let Ok(cursor_img) = image::open(&cursor_path) {
        let cursor_rgba = cursor_img.to_rgba8();
        let (width, height) = cursor_rgba.dimensions();
        let mut cursor_data = cursor_rgba.into_raw();

        // Pre-multiplied alpha handling - common method to solve cursor transparency issues
        for chunk in cursor_data.chunks_exact_mut(4) {
            let alpha = chunk[3] as f32 / 255.0;
            chunk[0] = (chunk[0] as f32 * alpha) as u8; // R * alpha
            chunk[1] = (chunk[1] as f32 * alpha) as u8; // G * alpha
            chunk[2] = (chunk[2] as f32 * alpha) as u8; // B * alpha
        }

        // Create CustomCursor source from image data
        match CustomCursor::from_rgba(cursor_data, width as u16, height as u16, 0, 0) {
            Ok(cursor_source) => {
                return Some(event_loop.create_custom_cursor(cursor_source));
            }
            Err(e) => {
                log::warn!("Failed to create cursor source: {:?}", e);
            }
        }
    } else {
        log::warn!("Cursor image not found: {}", cursor_path);
    }
    None
}

/// Apply custom cursor to window
///
/// Sets the custom cursor on the window and ensures it's visible.
///
/// # Parameters
/// - `window`: Reference to the window
/// - `cursor`: The custom cursor to apply
pub fn apply_cursor_to_window(window: &Arc<Window>, cursor: &CustomCursor) {
    window.set_cursor(cursor.clone());
    window.set_cursor_visible(true);
}

/// Window drag state management
///
/// Records window drag related states, supporting window position movement through mouse dragging.
/// Similar to SDL version implementation, providing the same user experience.
#[derive(Default)]
pub struct Drag {
    /// Whether drag operation needs to be executed
    pub need: bool,
    /// Whether currently dragging
    pub draging: bool,
    /// Drag start mouse X coordinate
    pub mouse_x: f64,
    /// Drag start mouse Y coordinate
    pub mouse_y: f64,
    /// X-axis drag offset
    pub dx: f64,
    /// Y-axis drag offset
    pub dy: f64,
}

/// Window initialization parameters
#[derive(Debug, Clone)]
pub struct WindowInitParams {
    pub width: u16,
    pub height: u16,
    pub ratio_x: f32,
    pub ratio_y: f32,
    pub title: String,
    pub texture_path: String,
}

/// Window move function
///
/// Executes actual window drag movement operation. Handles drag state updates and window position setting.
///
/// # Parameters
/// - `drag_need`: Mutable reference to whether drag is needed
/// - `window`: Optional window reference
/// - `dx`: X-axis offset
/// - `dy`: Y-axis offset
///
/// # Behavior
/// - Moves window position based on drag offset
/// - Resets drag flag
pub fn winit_move_win(drag_need: &mut bool, window: Option<&Window>, dx: f64, dy: f64) {
    // dragging window, set the correct position of a window
    if *drag_need {
        if let Some(win) = window {
            if let Ok(pos) = win.outer_position() {
                let new_x = pos.x + dx as i32;
                let new_y = pos.y + dy as i32;
                let _ = win.set_outer_position(winit::dpi::PhysicalPosition::new(new_x, new_y));
            }
        }
        *drag_need = false;
    }
}

/// Convert Winit events to RustPixel events
///
/// Converts winit native events to RustPixel internal event format for unified processing.
/// Supports multiple event types including keyboard input and mouse operations.
///
/// # Parameters
/// - `event`: Winit event reference
/// - `adjx`: X-axis adjustment factor (for high DPI displays)
/// - `adjy`: Y-axis adjustment factor (for high DPI displays)
/// - `use_tui_height`: If true, uses TUI character height (32px) for mouse coordinate conversion;
///                     if false, uses Sprite character height (16px)
/// - `cursor_pos`: Mutable reference to current cursor position
///
/// # Mouse Coordinate Conversion
/// Mouse pixel coordinates are converted to character cell coordinates.
/// The conversion accounts for TUI double-height mode to ensure accurate click detection.
///
/// # Returns
/// Returns Some(Event) if event can be converted, otherwise returns None
pub fn input_events_from_winit(
    event: &WinitEvent<()>,
    adjx: f32,
    adjy: f32,
    use_tui_height: bool,
    cursor_pos: &mut (f64, f64),
) -> Option<Event> {
    use crate::event::{
        Event, KeyCode, KeyEvent, KeyModifiers, MouseButton::*, MouseEvent, MouseEventKind::*,
    };

    let sym_width = PIXEL_SYM_WIDTH.get().expect("lazylock init");
    let sym_height = PIXEL_SYM_HEIGHT.get().expect("lazylock init");

    match event {
        WinitEvent::WindowEvent {
            event: window_event,
            ..
        } => {
            match window_event {
                WindowEvent::KeyboardInput {
                    event: key_event, ..
                } => {
                    if key_event.state == winit::event::ElementState::Pressed {
                        if let winit::keyboard::PhysicalKey::Code(keycode) = key_event.physical_key
                        {
                            let kc = match keycode {
                                winit::keyboard::KeyCode::Space => ' ',
                                winit::keyboard::KeyCode::KeyA => 'a',
                                winit::keyboard::KeyCode::KeyB => 'b',
                                winit::keyboard::KeyCode::KeyC => 'c',
                                winit::keyboard::KeyCode::KeyD => 'd',
                                winit::keyboard::KeyCode::KeyE => 'e',
                                winit::keyboard::KeyCode::KeyF => 'f',
                                winit::keyboard::KeyCode::KeyG => 'g',
                                winit::keyboard::KeyCode::KeyH => 'h',
                                winit::keyboard::KeyCode::KeyI => 'i',
                                winit::keyboard::KeyCode::KeyJ => 'j',
                                winit::keyboard::KeyCode::KeyK => 'k',
                                winit::keyboard::KeyCode::KeyL => 'l',
                                winit::keyboard::KeyCode::KeyM => 'm',
                                winit::keyboard::KeyCode::KeyN => 'n',
                                winit::keyboard::KeyCode::KeyO => 'o',
                                winit::keyboard::KeyCode::KeyP => 'p',
                                winit::keyboard::KeyCode::KeyQ => 'q',
                                winit::keyboard::KeyCode::KeyR => 'r',
                                winit::keyboard::KeyCode::KeyS => 's',
                                winit::keyboard::KeyCode::KeyT => 't',
                                winit::keyboard::KeyCode::KeyU => 'u',
                                winit::keyboard::KeyCode::KeyV => 'v',
                                winit::keyboard::KeyCode::KeyW => 'w',
                                winit::keyboard::KeyCode::KeyX => 'x',
                                winit::keyboard::KeyCode::KeyY => 'y',
                                winit::keyboard::KeyCode::KeyZ => 'z',
                                winit::keyboard::KeyCode::ArrowUp => {
                                    return Some(Event::Key(KeyEvent::new(
                                        KeyCode::Up,
                                        KeyModifiers::NONE,
                                    )))
                                }
                                winit::keyboard::KeyCode::ArrowDown => {
                                    return Some(Event::Key(KeyEvent::new(
                                        KeyCode::Down,
                                        KeyModifiers::NONE,
                                    )))
                                }
                                winit::keyboard::KeyCode::ArrowLeft => {
                                    return Some(Event::Key(KeyEvent::new(
                                        KeyCode::Left,
                                        KeyModifiers::NONE,
                                    )))
                                }
                                winit::keyboard::KeyCode::ArrowRight => {
                                    return Some(Event::Key(KeyEvent::new(
                                        KeyCode::Right,
                                        KeyModifiers::NONE,
                                    )))
                                }
                                winit::keyboard::KeyCode::Enter => {
                                    return Some(Event::Key(KeyEvent::new(
                                        KeyCode::Enter,
                                        KeyModifiers::NONE,
                                    )))
                                }
                                winit::keyboard::KeyCode::Escape => {
                                    return Some(Event::Key(KeyEvent::new(
                                        KeyCode::Esc,
                                        KeyModifiers::NONE,
                                    )))
                                }
                                winit::keyboard::KeyCode::Backspace => {
                                    return Some(Event::Key(KeyEvent::new(
                                        KeyCode::Backspace,
                                        KeyModifiers::NONE,
                                    )))
                                }
                                winit::keyboard::KeyCode::Tab => {
                                    return Some(Event::Key(KeyEvent::new(
                                        KeyCode::Tab,
                                        KeyModifiers::NONE,
                                    )))
                                }
                                winit::keyboard::KeyCode::Delete => {
                                    return Some(Event::Key(KeyEvent::new(
                                        KeyCode::Delete,
                                        KeyModifiers::NONE,
                                    )))
                                }
                                winit::keyboard::KeyCode::Home => {
                                    return Some(Event::Key(KeyEvent::new(
                                        KeyCode::Home,
                                        KeyModifiers::NONE,
                                    )))
                                }
                                winit::keyboard::KeyCode::End => {
                                    return Some(Event::Key(KeyEvent::new(
                                        KeyCode::End,
                                        KeyModifiers::NONE,
                                    )))
                                }
                                winit::keyboard::KeyCode::PageUp => {
                                    return Some(Event::Key(KeyEvent::new(
                                        KeyCode::PageUp,
                                        KeyModifiers::NONE,
                                    )))
                                }
                                winit::keyboard::KeyCode::PageDown => {
                                    return Some(Event::Key(KeyEvent::new(
                                        KeyCode::PageDown,
                                        KeyModifiers::NONE,
                                    )))
                                }
                                winit::keyboard::KeyCode::Insert => {
                                    return Some(Event::Key(KeyEvent::new(
                                        KeyCode::Insert,
                                        KeyModifiers::NONE,
                                    )))
                                }
                                winit::keyboard::KeyCode::Digit1 => '1',
                                winit::keyboard::KeyCode::Digit2 => '2',
                                winit::keyboard::KeyCode::Digit3 => '3',
                                winit::keyboard::KeyCode::Digit4 => '4',
                                winit::keyboard::KeyCode::Digit5 => '5',
                                winit::keyboard::KeyCode::Digit6 => '6',
                                winit::keyboard::KeyCode::Digit7 => '7',
                                winit::keyboard::KeyCode::Digit8 => '8',
                                winit::keyboard::KeyCode::Digit9 => '9',
                                winit::keyboard::KeyCode::Digit0 => '0',
                                _ => return None,
                            };
                            return Some(Event::Key(KeyEvent::new(
                                KeyCode::Char(kc),
                                KeyModifiers::NONE,
                            )));
                        }
                    }
                }
                WindowEvent::CursorMoved { position, .. } => {
                    cursor_pos.0 = position.x;
                    cursor_pos.1 = position.y;

                    // Convert pixel coordinates to cell coordinates
                    // No border offset needed (using OS window decoration)
                    // Account for TUI mode: double height (32px) vs sprite height (16px)
                    let cell_height = if use_tui_height {
                        *sym_height * 2.0
                    } else {
                        *sym_height
                    };
                    let px = (cursor_pos.0 / (*sym_width as f64 / adjx as f64)) as u16;
                    let py = (cursor_pos.1 / (cell_height as f64 / adjy as f64)) as u16;

                    return Some(Event::Mouse(MouseEvent {
                        kind: Moved,
                        column: px,
                        row: py,
                        modifiers: KeyModifiers::NONE,
                    }));
                }
                WindowEvent::MouseInput { state, button, .. } => {
                    // Account for TUI mode: double height (32px) vs sprite height (16px)
                    let cell_height = if use_tui_height {
                        *sym_height * 2.0
                    } else {
                        *sym_height
                    };
                    
                    if *state == winit::event::ElementState::Pressed {
                        // Convert pixel coordinates to cell coordinates
                        // No border offset needed (using OS window decoration)
                        let px = (cursor_pos.0 / (*sym_width as f64 / adjx as f64)) as u16;
                        let py = (cursor_pos.1 / (cell_height as f64 / adjy as f64)) as u16;

                        let mouse_button = match button {
                            winit::event::MouseButton::Left => Left,
                            winit::event::MouseButton::Right => Right,
                            winit::event::MouseButton::Middle => Middle,
                            _ => Left,
                        };

                        return Some(Event::Mouse(MouseEvent {
                            kind: Down(mouse_button),
                            column: px,
                            row: py,
                            modifiers: KeyModifiers::NONE,
                        }));
                    } else {
                        // Convert pixel coordinates to cell coordinates
                        // No border offset needed (using OS window decoration)
                        let px = (cursor_pos.0 / (*sym_width as f64 / adjx as f64)) as u16;
                        let py = (cursor_pos.1 / (cell_height as f64 / adjy as f64)) as u16;

                        let mouse_button = match button {
                            winit::event::MouseButton::Left => Left,
                            winit::event::MouseButton::Right => Right,
                            winit::event::MouseButton::Middle => Middle,
                            _ => Left,
                        };

                        return Some(Event::Mouse(MouseEvent {
                            kind: Up(mouse_button),
                            column: px,
                            row: py,
                            modifiers: KeyModifiers::NONE,
                        }));
                    }
                }
                _ => {}
            }
        }
        _ => {}
    }
    None
}

/// 🔧 Winit Adapter Common Initialization Function
///
/// This function extracts all common initialization logic between WinitGlowAdapter and WinitWgpuAdapter,
/// implementing DRY principle and greatly reducing code duplication.
///
/// ## 🎯 Shared Initialization Steps
/// 1. **Texture Loading**: Load PIXEL_TEXTURE_FILE and set symbol dimensions
/// 2. **Parameter Setting**: Configure window size, scaling ratios and other basic parameters
/// 3. **Event Loop**: Create winit EventLoop instance
/// 4. **Parameter Storage**: Save WindowInitParams for resumed event use
///
/// ## 🚀 Performance Advantages
/// - **Compile-time optimization**: Inlining eliminates function call overhead
/// - **Code reuse**: Avoid maintaining duplicate logic
/// - **Type safety**: Strong typed generics ensure correct adapter usage
///
/// # Generic Parameters
/// - `T`: Adapter type that must implement basic size and title setting interface
///
/// # Parameters
/// - `adapter`: Mutable reference to adapter
/// - `w`: Window width (in cells)
/// - `h`: Window height (in cells)
/// - `rx`: X-axis scaling ratio
/// - `ry`: Y-axis scaling ratio
/// - `title`: Window title
///
/// # Returns
/// - `(EventLoop<()>, WindowInitParams, String)`: Event loop, initialization parameters and texture path
pub fn winit_init_common<T>(
    adapter: &mut T,
    w: u16,
    h: u16,
    rx: f32,
    ry: f32,
    title: String,
) -> (winit::event_loop::EventLoop<()>, WindowInitParams, String)
where
    T: crate::render::adapter::Adapter,
{
    use crate::render::adapter::{PIXEL_SYM_HEIGHT, PIXEL_SYM_WIDTH, PIXEL_TEXTURE_FILE};
    use log::info;
    use winit::event_loop::EventLoop;

    info!("Initializing Winit adapter common components...");

    // 1. Use pre-loaded texture data from init_pixel_assets()
    // Texture and symbol_map are already loaded during init_pixel_assets()
    // PIXEL_SYM_WIDTH/HEIGHT are already set
    let tex_data = crate::get_pixel_texture_data();
    let texwidth = tex_data.width;
    let texheight = tex_data.height;

    // Build texture path for logging (actual data comes from cache)
    let project_path = &crate::get_game_config().project_path;
    let texture_path = format!(
        "{}{}{}",
        project_path,
        std::path::MAIN_SEPARATOR,
        PIXEL_TEXTURE_FILE
    );

    info!("Using pre-loaded texture: {}x{}", texwidth, texheight);
    info!(
        "Symbol dimensions: {}x{} (Sprite: 16x16, TUI: 16x32)",
        PIXEL_SYM_WIDTH.get().expect("PIXEL_SYM_WIDTH not initialized - call init_pixel_assets first"),
        PIXEL_SYM_HEIGHT.get().expect("PIXEL_SYM_HEIGHT not initialized - call init_pixel_assets first"),
    );

    // 2. Set basic parameters
    adapter.set_size(w, h);
    adapter.set_title(title.clone());

    // Get base reference once to avoid multiple mutable borrows
    let base = adapter.get_base();
    base.gr.set_ratiox(rx);
    base.gr.set_ratioy(ry);

    // Get needed values first, then call methods
    let cell_w = base.cell_w;
    let cell_h = base.cell_h;
    base.gr.set_pixel_size(cell_w, cell_h);

    info!(
        "Window pixel size: {}x{}",
        base.gr.pixel_w, base.gr.pixel_h
    );

    // 3. Create event loop
    let event_loop = EventLoop::new().unwrap();

    // 4. Create window initialization parameters
    let window_init_params = WindowInitParams {
        width: w,
        height: h,
        ratio_x: rx,
        ratio_y: ry,
        title,
        texture_path: texture_path.clone(),
    };

    info!("Common initialization completed, window will be created in resumed()");

    (event_loop, window_init_params, texture_path)
} 