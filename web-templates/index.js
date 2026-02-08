/**
 * RustPixel WASM Wrapper Script
 * 
 * This JavaScript file serves as the bridge between the browser environment
 * and the RustPixel game engine compiled to WebAssembly (WASM). It handles:
 * - WASM module initialization and asset loading
 * - Game loop management with precise timing
 * - Event forwarding from browser to Rust
 * - Image data processing for WebGL rendering
 */

/**
 * Asset Loading Bridge Function
 * 
 * This function is called FROM Rust code to load assets asynchronously.
 * The Rust WASM code cannot directly use fetch(), so it delegates to this JS function.
 * 
 * Flow: Rust calls js_load_asset(url) → fetch → arrayBuffer → Uint8Array → back to Rust
 */
export const js_load_asset = (url) => {
    fetch(url)
        .then(data => {
            // Convert Response to ArrayBuffer (binary data)
            return data.arrayBuffer();
        })
        .then(res => {
            // Convert ArrayBuffer to Uint8Array and send back to Rust
            // sg.on_asset_loaded() is the WASM-exported function from Rust
            sg.on_asset_loaded(url, new Uint8Array(res));
        })
        .catch(error => {
            console.error(`Failed to load asset: ${url}`, error);
        });
};

/**
 * Utility functions for game management
 */
const utils = {};

/**
 * High-Precision Game Loop Implementation
 * 
 * Implements a fixed-timestep game loop that maintains 60 FPS regardless of
 * display refresh rate. This ensures consistent game logic timing across devices.
 * 
 * @param {Function} update - The update function to call each frame
 */
utils.loop = update => {
    let lastTime = performance.now();
    const frameDuration = 1000 / 60; // 16.67ms per frame for 60 FPS

    const loopFunction = function(currentTime) {
        const deltaTime = currentTime - lastTime;

        // Only update if enough time has passed (frame rate limiting)
        if (deltaTime >= frameDuration) {
            // Handle cases where multiple frames should have occurred
            // (e.g., tab was in background, system lag)
            const times = Math.floor(deltaTime / frameDuration);
            update((times * frameDuration) * 0.001); // Convert milliseconds to seconds for Rust
            lastTime += times * frameDuration;
        }

        // Schedule next frame using browser's optimal timing
        requestAnimationFrame(loopFunction);
    };

    // Start the loop
    requestAnimationFrame(loopFunction);
};

// ============================================================================
// WASM Initialization and Game Setup
// ============================================================================

/**
 * WASM Module Initialization
 * 
 * The 'await' here is CRITICAL for proper initialization sequence:
 * 1. Downloads the .wasm file from ./pkg/pixel.wasm
 * 2. Instantiates the WebAssembly module
 * 3. Sets up the JavaScript ↔ WASM interface
 * 4. Returns the initialized module with exported functions
 * 
 * WHY AWAIT: WASM initialization is asynchronous because:
 * - Network download of .wasm file takes time
 * - WebAssembly.instantiate() is inherently async
 * - Memory allocation and linking must complete before use
 */
import init, {PixelGame, wasm_init_pixel_assets} from "./pkg/pixel.js";
const wasm = await init();

/**
 * Symbol Texture Loading and Processing
 * 
 * RustPixel uses a symbol atlas (symbols.png) containing all drawable characters
 * and sprites. This must be loaded and processed before the game can render.
 */
const timg = new Image();
timg.src = "assets/pix/symbols.png";

/**
 * Image Decode Await Explanation:
 * 
 * The 'await timg.decode()' is essential because:
 * 1. Image loading (setting src) is asynchronous
 * 2. Browser may not have finished decoding the image data yet
 * 3. decode() returns a Promise that resolves when image is ready for use
 * 4. Without await, getImageData() might fail or return incomplete data
 * 
 * This ensures the image is FULLY loaded and decoded before processing.
 */
await timg.decode();

// Create a canvas to extract pixel data from the loaded image
const canvas = document.createElement("canvas");
canvas.width = timg.width;
canvas.height = timg.height;
const ctx = canvas.getContext("2d");

// Draw the image to canvas and extract raw pixel data
ctx.drawImage(timg, 0, 0);
const imgdata = ctx.getImageData(0, 0, timg.width, timg.height).data;

/**
 * Unified Asset Loading (New Approach)
 *
 * Load the symbol_map.json configuration file and initialize all assets
 * at once using wasm_init_pixel_assets(). This provides:
 * - Unified loading for texture + symbol_map
 * - Consistent with native mode initialization
 * - All assets cached before game creation
 */
const symbolMapResponse = await fetch("assets/pix/symbol_map.json");
const symbolMapText = await symbolMapResponse.text();

// Initialize all assets at once: game config + texture + symbol_map
wasm_init_pixel_assets("pixel_game", timg.width, timg.height, imgdata, symbolMapText);

/**
 * Game Instance Creation and Initialization
 *
 * Creates the main game object (compiled from Rust to WASM) and initializes
 * WebGL using the pre-cached texture data.
 */
const sg = PixelGame.new();
sg.init_from_cache();  // Initialize WebGL using cached texture data

// ============================================================================
// Event System: Browser → Rust
// ============================================================================

/**
 * Input Event Forwarding
 * 
 * Browser events are captured and forwarded to Rust with type identifiers:
 * - 0: Key press events
 * - 1: Mouse button release
 * - 2: Mouse button press  
 * - 3: Mouse movement
 * 
 * The Rust code (in pixel_game! macro) handles these through key_event() method.
 */
window.onkeydown = (e) => {
    sg.key_event(0, e);
    // Prevent browser default for game-relevant keys
    if (["Space","ArrowLeft","ArrowRight","ArrowUp","ArrowDown","PageUp","PageDown","Home","End"].includes(e.code)) {
        e.preventDefault();
    }
};
window.onmouseup = (e) => { sg.key_event(1, e); };
window.onmousedown = (e) => { sg.key_event(2, e); };
window.onmousemove = (e) => { sg.key_event(3, e); };

// ============================================================================
// Game Loop Startup
// ============================================================================

/**
 * Start the main game loop
 * 
 * This calls the Rust game's tick() method at 60 FPS, passing the time delta.
 * The Rust code handles:
 * - Game logic updates
 * - Rendering to WebGL context
 * - State management
 */
utils.loop(function(timeStep) {
    sg.tick(timeStep);
    return true;
});

