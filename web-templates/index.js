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
 *
 * IMPORTANT: Uses wasm_on_asset_loaded() (a free function) instead of sg.on_asset_loaded()
 * (an instance method). This avoids double mutable borrow when fetch callbacks fire
 * during async init_from_cache(). Data is queued and processed in tick().
 */
export const js_load_asset = (url) => {
    fetch(url)
        .then(data => {
            // Convert Response to ArrayBuffer (binary data)
            return data.arrayBuffer();
        })
        .then(res => {
            // Queue asset data via free function (no &mut Game needed)
            wasm_on_asset_loaded(url, new Uint8Array(res));
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
import init, {PixelGame, wasm_init_pixel_assets, wasm_set_app_data, wasm_on_asset_loaded} from "./pkg/pixel.js";
const wasm = await init();

/**
 * Layered Symbol Texture Loading (Texture2DArray)
 *
 * RustPixel uses a Texture2DArray with multiple square layers (e.g., 2048×2048)
 * instead of a single large atlas. The layered_symbol_map.json describes which
 * layer PNGs to load and maps symbol strings to (layer, u, v) coordinates.
 */
const symbolMapResponse = await fetch("assets/pix/layered_symbol_map.json");
const symbolMapText = await symbolMapResponse.text();
const symbolMap = JSON.parse(symbolMapText);
const layerSize = symbolMap.layer_size;
const layerFiles = symbolMap.layer_files;
console.log(`[DEBUG] Loading ${layerFiles.length} layers (${layerSize}x${layerSize} each)`);

// Load all layer PNGs in parallel, extract raw RGBA data, and concatenate
const bytesPerLayer = layerSize * layerSize * 4;
const allLayerData = new Uint8Array(bytesPerLayer * layerFiles.length);

await Promise.all(layerFiles.map(async (file, i) => {
    const resp = await fetch(`assets/pix/${file}`);
    const blob = await resp.blob();
    const bitmap = await createImageBitmap(blob);

    // Extract raw RGBA pixels via OffscreenCanvas
    let cv, cx;
    if (typeof OffscreenCanvas !== 'undefined') {
        cv = new OffscreenCanvas(bitmap.width, bitmap.height);
        cx = cv.getContext("2d");
    } else {
        cv = document.createElement("canvas");
        cv.width = bitmap.width;
        cv.height = bitmap.height;
        cx = cv.getContext("2d");
    }
    cx.drawImage(bitmap, 0, 0);
    const pixels = cx.getImageData(0, 0, bitmap.width, bitmap.height).data;

    // Copy into concatenated buffer at the correct offset
    allLayerData.set(pixels, i * bytesPerLayer);
    console.log(`[DEBUG] Layer ${i} loaded: ${bitmap.width}x${bitmap.height}`);
}));

console.log(`[DEBUG] All layers loaded: ${allLayerData.length} bytes total`);

// Initialize all assets: game config + layers + symbol_map
wasm_init_pixel_assets("pixel_game", layerSize, layerFiles.length, allLayerData, symbolMapText);

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
 * WGPU using the pre-cached layer data (Texture2DArray).
 */
const sg = PixelGame.new();

// Initialize WGPU with cached layers (async - must await!)
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

