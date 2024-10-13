[Readme]: ../README.md
[Principle]: principle.md

Before coding, please read the [Readme] first and install `rust_pixel` then read the [Principle] to understand the basic concepts

### Create new project
- Create game or terminal-app use cargo-pixel tool:
```
cargo pixel c games block 
cargo pixel c apps block 
```
The above command will create a new project in the `games` or `apps` subdirectory of the rust_pixel directory.


More commonly, you can create a **standalone** project that depends on rust_pixel, using the following command:
```
cargo pixel c .. block --standalone
cd ../block
cargo pixel r block t -r    # run standalone project in term mode...
cargo pixel r block s -r    # run standalone project in sdl mode...
cargo pixel r block w -r    # run standalone project in web mode...
```
In this way, an independent project named `block` will be created in the upper directory of rust_pixel.

### Project main entry
- src/main.rs is the binary main entry
```rust
fn main() {
    // block::run is defined in lib.rs
    block::run()
}
```

If the project only runs in graphics mode, you can use macro:
```rust
fn main() {
    // if not graphics mode, exit() will be call
    rust_pixel::only_graphics_mode!();

    // if not terminal mode, exit() will be call
    // rust_pixel::only_terminal_mode!();

    // use cfg avoid "unreachable" compile warning
    #[cfg(any(feature = "sdl", target_arch = "wasm32"))]
    block::run()
}
```

- src/lib.rs is the main code logic entry

To reduce duplication of code, procedural macros are used:

```rust
mod model;
mod render;

use pixel_macro::pixel_game;

// refer to rust_pixel/pixel_macro for macro details
pixel_game!(Block, "app", ".");  
```

`pixel_game!(Block, "app", ".")` will expand into the following code:

```rust
use crate::{model::BlockModel, render::BlockRender};
use rust_pixel::game::Game;

#[cfg(target_arch = "wasm32")]
use rust_pixel::render::adapter::web::{input_events_from_web, WebAdapter};
use wasm_bindgen::prelude::*;
#[cfg(target_arch = "wasm32")]
use wasm_bindgen_futures::js_sys;
#[cfg(target_arch = "wasm32")]
use log::info;

#[cfg_attr(target_arch = "wasm32", wasm_bindgen)]
pub struct BlockGame {
    g: Game<BlockModel, BlockRender>,
}

pub fn init_game() -> BlockGame {
    let m = BlockModel::new();
    let r = BlockRender::new();
    let mut g = Game::new_with_project_path(m, r, "app/block", Some("."));
    g.init();
    BlockGame { g }
}

#[cfg(target_arch = "wasm32")]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen)]
impl BlockGame {
    pub fn new() -> Self {
        init_game()
    }

    pub fn tick(&mut self, dt: f32) {
        self.g.on_tick(dt);
    }

    pub fn key_event(&mut self, t: u8, e: web_sys::Event) {
        let abase = &self
            .g
            .context
            .adapter
            .as_any()
            .downcast_ref::<WebAdapter>()
            .unwrap()
            .base;
        if let Some(pe) = input_events_from_web(t, e, abase.ratio_x, abase.ratio_y) {
            self.g.context.input_events.push(pe);
        }
    }

    pub fn upload_imgdata(&mut self, w: i32, h: i32, d: &js_sys::Uint8ClampedArray) {
        let length = d.length() as usize;
        let mut pixels = vec![0u8; length];
        d.copy_to(&mut pixels);
        info!("RUST...pixels.len={}", pixels.len());

        let wa = &mut self
            .g
            .context
            .adapter
            .as_any()
            .downcast_mut::<WebAdapter>()
            .unwrap();

        wa.init_glpix(w, h, &pixels);
    }

    pub fn on_asset_loaded(&mut self, url: &str, data: &[u8]) {
        // info!("asset({:?}): {:?}!!!", url, data);
        self.g.context.asset_manager.set_data(url, data);
    }

    pub fn get_ratiox(&mut self) -> f32 {
        self.g.context.adapter.get_base().ratio_x
    }

    pub fn get_ratioy(&mut self) -> f32 {
        self.g.context.adapter.get_base().ratio_y
    }
}

// call by main.rs
pub fn run() {
    let mut g = init_game().g;
    g.run().unwrap();
    g.render.panel.reset(&mut g.context);
}
``` 

### Model and Render

