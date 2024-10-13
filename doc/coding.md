[Readme]: ../README.md
[Principle]: principle.md

Before coding, please read the [Readme] first, then read the [Principle] to understand the basic concepts

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
- `Model` is an encapsulation of game data and status, and also implements most of the core logic other than rendering.
- `Render` renders based on game state data. 

The traits of Model and Render are defined as follows:
```rust
/// The Model interface, main entrance for data and core logic
pub trait Model {
    fn init(&mut self, ctx: &mut Context);
    fn update(&mut self, ctx: &mut Context, dt: f32) {
        // render logo movie...
        if ctx.stage <= LOGO_FRAME {
            return;
        }
        // Update timer and trigger related events 
        timer_update();
        // handle event
        self.handle_event(ctx, dt);
        // handle timer event
        self.handle_timer(ctx, dt);
        // handle keyboard & mouse input
        self.handle_input(ctx, dt);
        // handle games auto logic
        self.handle_auto(ctx, dt);
    }
    // user handle interface...
    fn handle_timer(&mut self, ctx: &mut Context, dt: f32);
    fn handle_event(&mut self, ctx: &mut Context, dt: f32);
    fn handle_input(&mut self, ctx: &mut Context, dt: f32);
    fn handle_auto(&mut self, ctx: &mut Context, dt: f32);
}

/// The Render interface, takes context and model as input params. It renders every single frame
pub trait Render {
    type Model: Model;

    fn init(&mut self, ctx: &mut Context, m: &mut Self::Model);
    fn update(&mut self, ctx: &mut Context, m: &mut Self::Model, dt: f32) {
        self.handle_event(ctx, m, dt);
        self.handle_timer(ctx, m, dt);
        self.draw(ctx, m, dt);
    }
    // user handle interface...
    fn handle_event(&mut self, ctx: &mut Context, model: &mut Self::Model, dt: f32);
    fn handle_timer(&mut self, ctx: &mut Context, model: &mut Self::Model, dt: f32);
    fn draw(&mut self, ctx: &mut Context, model: &mut Self::Model, dt: f32);
}
```

The actual Model code reference is as follows:
```rust
#[repr(u8)]
enum BlockState {
    Normal,
}

pub struct BlockModel {
    pub data: BlockData,
    pub bezier: AnimationSequence<PointF32>,
    pub card: u8,
}

impl BlockModel {
    pub fn new() -> Self {
        Self {
            data: BlockData::new(),
            bezier: AnimationSequence::new(),
            card: 0,
        }
    }
}

impl Model for BlockModel {
    fn init(&mut self, _context: &mut Context) {
        let in_points = [
            PointF32 { x: 10.0, y: 30.0 },
            PointF32 { x: 210.0, y: 450.0 },
            PointF32 { x: 110.0, y: 150.0 },
            PointF32 {
                x: 1200.0,
                y: 150.0,
            },
            PointF32 {
                x: BLOCKW as f32 * 16.0,
                y: BLOCKH as f32 * 16.0,
            },
        ];
        let num = 100;
        let mut pts = vec![PointF32 { x: 0.0, y: 0.0 }; num];
        draw_bezier_curves(&in_points, &mut pts);

        let mut ks = Vec::new();

        for i in 0..num {
            ks.push((pts[i], i as f64 / num as f64, EaseIn).into());
        }

        self.bezier = AnimationSequence::from(ks);
        self.data.shuffle();
        self.card = self.data.next();
        event_emit("Block.RedrawTile");
    }

    fn handle_input(&mut self, context: &mut Context, _dt: f32) {
        let es = context.input_events.clone();
        for e in &es {
            match e {
                Event::Key(key) => match key.code {
                    KeyCode::Char('s') => {
                        self.data.shuffle();
                        self.card = self.data.next();
                        event_emit("Block.RedrawTile");
                    }
                    KeyCode::Char('n') => {
                        self.card = self.data.next();
                        event_emit("Block.RedrawTile");
                    }
                    _ => {
                        context.state = BlockState::Normal as u8;
                    }
                },
                _ => {}
            }
        }
        context.input_events.clear();
    }

    fn handle_auto(&mut self, _context: &mut Context, _dt: f32) {}
    fn handle_event(&mut self, _context: &mut Context, _dt: f32) {}
    fn handle_timer(&mut self, _context: &mut Context, _dt: f32) {}
}
```

The actual Render code reference is as follows:
```rust
pub struct BlockRender {
    pub panel: Panel,
}

impl BlockRender {
    pub fn new() -> Self {
        let mut panel = Panel::new();

        // use cfg attribute to differentiate between graphics and text modes
        #[cfg(any(feature = "sdl", target_arch = "wasm32"))]
        {
            // create pixel sprites in graphic mode...
            for i in 0..15 {
                let mut pl = Sprite::new(4, 6, 1, 1);
                // Use set_graph_sym set char content in graphics mode
                pl.set_graph_sym(0, 0, 1, 83, Color::Indexed(14));
                // Alpha only support in graphics mode
                pl.set_alpha(255 - 15 * (15 - i));
                panel.add_pixel_sprite(pl, &format!("PL{}", i + 1));
            }
        }

        // background...
        let mut gb = Sprite::new(0, 0, BLOCKW, BLOCKH);
        // Alpha only support in graphics mode
        gb.set_alpha(30);
        panel.add_sprite(gb, "back");
        panel.add_sprite(Sprite::new(0, 0, CARDW as u16, CARDH as u16), "t0");

        // msg, work on both text and graphics mode...
        let adj = 2u16;
        let mut msg1 = Sprite::new(0 + adj, 14, 40, 1);
        msg1.set_default_str("press N for next card");
        panel.add_sprite(msg1, "msg1");
        let mut msg2 = Sprite::new(40 + adj, 14, 40, 1);
        msg2.set_default_str("press S shuffle cards");
        panel.add_sprite(msg2, "msg2");

        // register Block.RedrawTile event, associated draw_tile method
        event_register("Block.RedrawTile", "draw_tile");

        Self { panel }
    }

    pub fn draw_tile(&mut self, ctx: &mut Context, d: &mut BlockModel) {
        let l = self.panel.get_sprite("t0");

        // make asset identifier...
        // in graphics mode, poker card asset file named n.pix
        // in text mode, poker card asset file named n.txt
        // 
        #[cfg(any(feature = "sdl", target_arch = "wasm32"))]
        let ext = "pix";

        #[cfg(not(any(feature = "sdl", target_arch = "wasm32")))]
        let ext = "txt";

        let cn = if d.card == 0 {
            format!("poker/back.{}", ext)
        } else {
            format!("poker/{}.{}", d.card, ext)
        };

        // set sprite content by asset identifier...
        asset2sprite!(l, ctx, &cn);

        // set sprite position...
        l.set_pos(1, 7);
    }
}

impl Render for BlockRender {
    type Model = BlockModel;

    fn init(&mut self, context: &mut Context, _data: &mut Self::Model) {
        context
            .adapter
            .init(BLOCKW + 2, BLOCKH, 1.0, 1.0, "block".to_string());
        self.panel.init(context);

        // set a static back img for text mode...
        #[cfg(not(any(feature = "sdl", target_arch = "wasm32")))]
        {
            let gb = self.panel.get_sprite("back");
            asset2sprite!(gb, context, "back.txt");
        }
    }

    fn handle_event(&mut self, context: &mut Context, data: &mut Self::Model, _dt: f32) {
        // if a Block.RedrawTile event checked, call draw_tile function...
        if event_check("Block.RedrawTile", "draw_tile") {
            self.draw_tile(context, data);
        }
    }

    fn handle_timer(&mut self, _context: &mut Context, d: &mut Self::Model, _dt: f32) {}

    fn draw(&mut self, ctx: &mut Context, d: &mut Self::Model, dt: f32) {
        #[cfg(any(feature = "sdl", target_arch = "wasm32"))]
        {
            // set a animate background for graphic mode...
            // asset file is 1.ssf
            let ss = &mut self.panel.get_sprite("back");
            asset2sprite!(ss, ctx, "1.ssf", (ctx.stage / 3) as usize, 40, 1);

            // set a bezier animation for graphic mode...
            for i in 0..15 {
                let pl = &mut self.panel.get_pixel_sprite(&format!("PL{}", i + 1));
                d.bezier
                    .advance_and_maybe_reverse(dt as f64 * 0.1 + 0.01 * i as f64);
                let kf_now = d.bezier.now_strict().unwrap();
                pl.set_pos(kf_now.x as u16, kf_now.y as u16);
                let c = ((ctx.stage / 10) % 255) as u8;
                pl.set_graph_sym(0, 0, 1, 83, Color::Indexed(c));
                d.bezier.advance_and_maybe_reverse(-0.01 * i as f64);
            }
        }

        // draw all compents in panel...
        self.panel.draw(ctx).unwrap();
    }
}
```
You should pay attention not to have too deep coupling between Render and Model.
