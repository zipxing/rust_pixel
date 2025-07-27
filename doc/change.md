# 1.0.2
- cargo upgrade: upgrade depend crate version
- toml 0.9 is bad, downgrade to toml 0.8 for cargo.toml parse

# 1.0.1
- Merge tools to cargo-pixel
```
cargo install --path . --force
cargo pixel 
```

# 1.0.0
- Added wgpu render backend
```
cargo pixel r petview wg -r   # winit + wgpu
cargo pixel r petview g -r    # winit + glow
cargo pixel r petview s -r    # sdl + glow
cargo pixel r petview w -r    # wasm with glow
```
- Remove pixel_macro crate,move to lib.rs

# 0.6.0 / 0.6.1
- Bug fix
- Rename c64.png to symbols.png 
- 0.6.1 fix symbols.png bug

# 0.5.9
- Added pixel_symbol tool, which can dig symbols from a pixel art picture.
```
cargo pixel r pixel_symbol t -r assets/pixel.png 16
```
- Fix petview app web mode bug

# 0.5.8
- Fix a runtime error in linux / wsl

# 0.5.7
- Split the app's render.rs into two files, render_terminal.rs and render_graphics.rs, to make the code logic clearer.
- Added pixel_symbol tool for extract the symbol set used in pixel art picture.

# 0.5.6
- Added pixel asset tool, which can package scattered png images into c64.png and generate pix files corresponding to each image
```
cargo pixel r pixel_asset t ./png_sources ./out
cp out/*.pix apps/city/assets
cp out/texture_atlas.png apps/city/assets/pix
```
- Please refer to apps/city games, now this game supports a better graphics mode
```
cargo pixel r city s -r
```

# 0.5.5
- Update cargo-pixel, added self-update feature. 
- If ~/rust_pixel_work/Cargo.toml version not equal cargo-pixel version, then auto exec `cargo install --path . --force`

# 0.5.4
- Update docs 

# 0.5.3
- Refactored the entire project structure by moving the contents of the rust-pixel directory to the root directory.
- Refactored the way project directories are managed: in development mode, using env::var("CARGO_MANIFEST_DIR") as the starting directory; after release, you can flexibly pass in the path via command line arguments.
- Refactored the cargo-pixel tool and set it as the binary file of rust_pixel, so you can directly install cargo-pixel via cargo install rust_pixel.

# 0.5.2
- Update coding.md and principle.md documents
- Fix cargo-pixel bug, please update cargo-pixel:
```
cargo install --path tools/cargo-pixel --root ~/.cargo
```
- Fix gl transition size

# 0.5.1
- Update petview game, added a online demo
- Update README.md and added a petview demo video

# 0.5.0
- Fixed numerous cargo clippy warnings

# 0.4.9
- Refactored gl/pixel.rs, Use the GlRender trait to wrap several opengl shaders and facilitate further extension of the shader
- Updated the cargo-pixel tool so that when it runs, it first compares the version number in pixel.toml. If it is inconsistent, it prompts to update cargo-pixel.

# 0.4.8
- Refactored the underlying rendering module, abandoned the canvas API of SDL, and replaced it with OpenGL shader,
- Opengl rendering improved engine performance (CPU dropped from 38% to about 15%) 
- Added the ability to use shader to achieve various special effects(coding petview transition)
- Refer to rust-pixel/src/render/adapter/sdl.rs
- Abstracted the repeated code in lib.rs of each application into a procedural macro:
```
pixel_game!(Snake)
```
- Refer to pixel_macro/src/lib.rs

# 0.4.7
- add petview game for petscii arts
- update tpetii tool for convert petscii art to pix files
- added graph mode cell background color

# 0.4.6
- Added linux & windows install guide
- Update readme

# 0.4.5
- Update cargo.toml fix rustdoc bug...

# 0.4.4
- Palette tool work
```
cargo pixel r palette t -r
```

# 0.4.3
- Fix cargo pixel r foobar w bug

# 0.4.2
- Refactor Panel, added layers for rendering
- Refer to apps/palette render.rs
```
impl PaletteRender {
    pub fn new() -> Self {
        let mut panel = Panel::new();
        // creat main layer
        panel.add_layer("main");

        // background
        let gb = Sprite::new(0, 0, PALETTEW, PALETTEH);
        panel.add_layer_sprite(gb, "main", "back");

        // top menu
        let mb = Sprite::new(MENUX, MENUY, MENUW, 1);
        panel.add_layer_sprite(mb, "main", "menu");

        ...

        // creat 6 state layers
        for i in 0..6 {
            panel.add_layer(&format!("{}", i));
            if i != 0 {
                panel.deactive_layer(&format!("{}", i));
            }
        }
        ...
    }

    pub fn draw_menu<G: Model>(&mut self, ctx: &mut Context, model: &mut G) {
        let d = model.as_any().downcast_mut::<PaletteModel>().unwrap();

        // get layer sprite
        let mb = self.panel.get_layer_sprite("main", "menu");
        ...
    }
```
- Continue writing palette application
```
$ cargo pixel r palette t 
```

# 0.4.1
- Refactor Game and Context, added project_path variable
- Better support for standalone crates using rust_pixel
- Usually, you can use the following commands to creat games or apps that use pixel:
```
$ cargo pixel c games mygame
$ cargo pixel c apps myapp
```
However, these crates are embedded into rust_pixel, 
```
    let mut g = Game::new(m, r, "games/mygame");
    g.init();
```
the project_path is "games/mygame" or "apps/myapp" by default, the assets file path should be "games/mygame/asset/"
if you creat a standalone project with:
```
$ cargo pixel c .. mygame --standalone
$ cargo pixel c .. mygame -s
```
and use "cargo run" in mygame directory, you should use this codes for load assets in "./asset/" path:
```
    let mut g = Game::new_with_project_path(m, r, "games/mygame", ".");
    g.init();
```
The cargo pixel creat <path> <project_name> --standlone command will automatically complete the above work for you.
```
$ cargo pixel c .. mygame --standalone  #Create a standalone crate in ../mygame 
$ cd ../mygame 
$ cargo pixel r mygame t
$ cargo pixel r mygame s

```

# 0.4.0
- Add ColorPro for professional color process
- Support HSL, CMYK, Lab, Lch, OkLab, OkLch ColorSpaces
- Add a terminal application: palette
```
$ cargo pixel r palette t

pub enum ColorSpace {
    SRGBA,
    LinearRGBA,
    CMYK,
    HSLA,
    HSVA,
    HWBA,
    LabA,
    LchA,
    OKLabA,
    OKLchA,
    XYZA,
}
```

# 0.3.3
- Fix cargo-pixel create bug
```
$ cargo pixel c games mygame
$ cargo pixel c apps myapp
```

# 0.3.2
- Rewrite cargo-pixel with rust
- Removed python3 dependence
- Updated readme.md

# 0.3.1
1. Add bezier algorithm, refer to algorithm/bezier.rs
2. Add a demo about keyframe animation and bezier path,refer to games/template/model.rs & render.rs
- model
```
        let in_points = [
            PointF32 { x: 0.0, y: 0.0 },
            PointF32 { x: 1200.0, y: 100.0 },
            PointF32 {
                x: TEMPLATEW as f32 * 16.0,
                y: TEMPLATEH as f32 * 16.0,
            },
        ];
        let num = 100;
        let mut pts = vec![PointF32 { x: 0.0, y: 0.0 }; num];
        draw_bezier_curves(&in_points, &mut pts);
        let mut ks = Vec::new();
        for i in 0..num {
            ks.push((pts[i], i as f64 / num as f64).into());
        }
        self.bezier = AnimationSequence::from(ks);
```
- render
```
        for i in 0..15 {
            let mut pl = Sprite::new(4, 6, 1, 1);
            pl.set_graph_sym(0, 0, 1, 83, Color::Indexed(203));
            pl.set_alpha(255 - 15*(15 - i));
            panel.add_pixel_sprite(pl, &format!("PL{}", i+1));
        }
        ...
        for i in 0..15 {
            let pl = &mut self.panel.get_pixel_sprite(&format!("PL{}", i+1));
            d.bezier.advance_and_maybe_reverse(dt as f64 * 0.1 + 0.01 * i as f64);
            let kf_now = d.bezier.now_strict().unwrap();
            pl.set_pos(kf_now.x as u16, kf_now.y as u16);
            d.bezier.advance_and_maybe_reverse(-0.01 * i as f64);
        }
```
3. Fix some bugs...

# 0.3.0
1. Add particle system
```
    //refer to games/snake/model.rs
    self.pats.fire_at(10.0, 10.0);
    ...
    self.pats.update(dt as f64);
    
    //refer to games/snake/render.rs
    pub fn draw_movie<G: Model>(&mut self, _ctx: &mut Context, model: &mut G) {
        let d = model.as_any().downcast_mut::<SnakeModel>().unwrap();

        self.panel.draw_objpool(&mut d.pats.particles, |pl, m| {
            pl.set_pos(m.obj.loc[0] as u16, m.obj.loc[1] as u16);
        });
    }
```
2. Optimize some render APIs for sprite
```
    /// set string content at (x,y) with fg/bg color...
    pub fn set_color_str<S>(&mut self, x: u16, y: u16, string: S, f: Color, b: Color);

    /// set string content at (0,0) with default style...
    pub fn set_default_str<S>(&mut self, string: S);

    /// set graphic model symbol(texture:texture_id, index:sym) at (x,y) with fgcolor...
    pub fn set_graph_sym(&mut self, x: u16, y: u16, texture_id: u8, sym: u8, f: Color);
```

# 0.2.0
1. Add a good template with bin / lib / ffi / wasm for create your own game, app or library
- refer to games/template
- create your own game, run:
```
cargo pixel c <my_game_name> 
```
2. Add global alpha for sprite
```
let sp: Sprite ... ...
sp.set_alpha(100);
```

