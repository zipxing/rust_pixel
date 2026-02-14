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
import init, {PixelGame, wasm_init_pixel_assets, wasm_set_app_data} from "./pkg/pixel.js";
const wasm = await init();

/**
 * Symbol Texture Loading and Processing
 *
 * RustPixel uses a symbol atlas (symbols.png) containing all drawable characters
 * and sprites. This must be loaded and processed before the game can render.
 *
 * We use fetch + createImageBitmap to bypass browser Image size limits.
 * Some browsers limit Image objects to 4096x4096, but createImageBitmap
 * can handle larger images (up to GPU texture limits, typically 8192 or 16384).
 */
const response = await fetch("assets/pix/symbols.png");
const blob = await response.blob();

// createImageBitmap bypasses Image object size limits
const imageBitmap = await createImageBitmap(blob);
console.log(`[DEBUG] ImageBitmap created: ${imageBitmap.width}x${imageBitmap.height}`);

// Create OffscreenCanvas if available (better for large images), fallback to regular canvas
let canvas, ctx;
if (typeof OffscreenCanvas !== 'undefined') {
    canvas = new OffscreenCanvas(imageBitmap.width, imageBitmap.height);
    ctx = canvas.getContext("2d");
    console.log(`[DEBUG] Using OffscreenCanvas: ${canvas.width}x${canvas.height}`);
} else {
    canvas = document.createElement("canvas");
    canvas.width = imageBitmap.width;
    canvas.height = imageBitmap.height;
    ctx = canvas.getContext("2d");
    console.log(`[DEBUG] Using regular canvas: ${canvas.width}x${canvas.height}`);
}

if (!ctx) {
    console.error("[ERROR] Failed to get 2D context!");
}

// Draw ImageBitmap to canvas and extract raw pixel data
ctx.drawImage(imageBitmap, 0, 0);
const imgdata = ctx.getImageData(0, 0, imageBitmap.width, imageBitmap.height).data;
console.log(`[DEBUG] getImageData complete, length: ${imgdata.length}`);

// Use imageBitmap dimensions for Rust
const timg = { width: imageBitmap.width, height: imageBitmap.height };

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
console.log(`[DEBUG] Texture image size: ${timg.width}x${timg.height}`);
console.log(`[DEBUG] Image data length: ${imgdata.length} bytes (expected: ${timg.width * timg.height * 4})`);
wasm_init_pixel_assets("pixel_game", timg.width, timg.height, imgdata, symbolMapText);

/**
 * Optional App Data Loading
 *
 * Apps can receive text data (e.g., markdown files, config) from the browser
 * via the `?data=` URL parameter. This avoids compile-time embedding and allows
 * switching content without recompilation.
 *
 * Example: http://localhost:8080?data=assets/demo.md
 */
const dataUrl = new URLSearchParams(window.location.search).get('data');
if (dataUrl) {
    try {
        const dataResponse = await fetch(dataUrl);
        if (dataResponse.ok) {
            wasm_set_app_data(await dataResponse.text());
        } else {
            console.warn(`Failed to load app data: ${dataUrl} (${dataResponse.status})`);
        }
    } catch (e) {
        console.warn('Failed to load app data:', e);
    }
}

/**
 * Game Instance Creation and Initialization
 *
 * Creates the main game object (compiled from Rust to WASM) and initializes
 * WebGL using the pre-cached texture data.
 */
const sg = PixelGame.new();

// Initialize WGPU (async - must await!)
await sg.init_from_cache();

/**
 * Dynamic Canvas Sizing
 *
 * After WGPU initialization, get the actual rendering dimensions from Rust
 * and resize the HTML canvas to match exactly. This prevents scaling artifacts
 * where the WGPU surface size differs from the canvas size.
 */
const canvasSize = sg.get_canvas_size();
const gameCanvas = document.getElementById("canvas");
gameCanvas.width = canvasSize[0];
gameCanvas.height = canvasSize[1];

// Scale canvas CSS size to fit browser window while maintaining aspect ratio
function fitCanvasToWindow() {
    const scaleX = window.innerWidth / canvasSize[0];
    const scaleY = window.innerHeight / canvasSize[1];
    const scale = Math.min(scaleX, scaleY);
    const displayW = Math.floor(canvasSize[0] * scale);
    const displayH = Math.floor(canvasSize[1] * scale);
    gameCanvas.style.width = displayW + "px";
    gameCanvas.style.height = displayH + "px";
    gameCanvas.style.left = Math.floor((window.innerWidth - displayW) / 2) + "px";
    gameCanvas.style.top = Math.floor((window.innerHeight - displayH) / 2) + "px";
}
fitCanvasToWindow();
window.addEventListener("resize", fitCanvasToWindow);
console.log(`Canvas: ${canvasSize[0]}x${canvasSize[1]} (WGPU), scaled to fit window`)

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

