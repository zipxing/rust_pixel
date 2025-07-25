![logo](./screen-shot/logo.png)

![License] [![Latest Version]][crates.io] ![Downloads] [![API Documentation]][docs.rs] ![MSRV]

[License]: https://img.shields.io/badge/license-Apache2.0-blue.svg
[Latest Version]: https://img.shields.io/crates/v/rust_pixel.svg
[crates.io]: https://crates.io/crates/rust_pixel
[Downloads]: https://img.shields.io/crates/d/rust_pixel.svg
[API Documentation]: https://docs.rs/rust_pixel/badge.svg
[docs.rs]: https://docs.rs/rust_pixel
[MSRV]: https://img.shields.io/badge/rust-1.71+-brightgreen.svg?&logo=rust

[Change Log]&nbsp; | &nbsp;[Principle]&nbsp; | &nbsp;[Coding]&nbsp; | &nbsp;[FAQ]&nbsp; | &nbsp;[TODO]

[Change Log]: doc/change.md
[Principle]: doc/principle.md
[Coding]: doc/coding.md
[FAQ]: doc/faq.md
[TODO]: doc/todo.md

RustPixel is a **2D game engine** & **rapid prototyping tools**, supporting both **text** and **graphics** rendering modes.<br>
It is suitable for creating **2D pixel-style games** and developing **terminal applications**.<br>
It can be compiled into **FFI** for front-end and back-end use, and into **WASM** for web projects.

- Text Mode: Built with **crossterm**, runs in the terminal, and uses **ASCII & Unicode Emoji** for drawing.
- Graphical Mode: Built with **wgpu** & **glow** & **sdl2**, using **PETSCII & custom graphics symbols** for rendering.

[online demo]: https://zipxing.github.io/rust_pixel

Here is a petscii art painting browser made with **rust_pixel**. Special thanks to x.com/PETSCIIWORLD for the character painting and the transition shader provided by **gltransition**. Click here for [online demo]。

https://github.com/user-attachments/assets/4758f2b6-66c2-47ed-997d-a9066be449aa

### Features

- Game loops & Model/Render design pattern (game.rs)
- Event/Timer messaging mechanism (event.rs)
- Support text render mode (crossterm) (adapter.rs, cross.rs)
- Unified OpenGL drawing mode supports sdl and wasm (glow & sdl2) (adapter.rs, sdl_adapter.rs, web_adapter.rs)
- Unified Wgpu drawing mode supports wgpu and wasm (winit & wgpu) (adapter.rs, winit_wgpu.rs)
- 3 core OpenGl shaders for sdl2 & web graphics mode: (gl/) 
- 3 core Wgpu shaders for wgpu graphics mode: (wgpu/) 
    - instance rendering shader for draw mainbuffer (render_symbols.rs) 
    - transition shader for transition effect (render_transition.rs)
    - general 2d shader for draw render texture (render_general2d.rs)
- Some common game algorithms (algorithm.rs, algorithm/, util.rs, util/)
- audio & log support (audio.rs, log.rs)
- Demo games: tetris, tower, poker... (apps/)
- Demo terminal ui app: palette... (apps/)
- Examples of wrapping core algorithms into FFI and WASM (apps/poker/ffi, apps/poker/wasm)

### Installation Guide

The main steps of the installation process are as follows:
- Install [DroidSansMono Nerd Font] & setup terminal
- Install dependent libraries and softwares
- Install **Rust** and **Wasm-pack**

The detailed steps for each operating system: &nbsp;&nbsp;[MacOS]&nbsp;&nbsp; | &nbsp;&nbsp;[Linux]&nbsp;&nbsp; | &nbsp;&nbsp;[Windows]

[MacOS]: doc/mac.md
[Linux]: doc/linux.md
[Windows]: doc/win.md
[DroidSansMono Nerd Font]: https://github.com/ryanoasis/nerd-fonts

Starting from version 0.5.3, you can deploy **cargo-pixel** directly from crates.io:
```
cargo install rust_pixel         # use crates.io rust_pixel crate deploy cargo-pixel
cargo pixel                      # first run cargo-pixel will clone rust_pixel to <homedir>/rust_pixel_work automatic 
cd ~/rust_pixel_work             # cd to workspace
cargo pixel r petview s          # run demo game...
```

To use the newest code, you should clone **RustPixel** and deploy **cargo-pixel** tool:
``` 
git clone https://github.com/zipxing/rust_pixel
cd rust_pixel
cargo install --path . --force
``` 

If you have installed an old version of cargo-pixel, you may get an error when running it. git update is required and the new version of cargo-pixel is deployed manually:
```
cd rust_pixel
git pull
cargo install --path . --force
```

### Usage Instructions
``` 
cd rust_pixel
cargo pixel run snake term            #Run the snake game in terminal mode
cargo pixel r snake t                 #Run the snake game in terminal mode - shorthand
cargo pixel r tetris s                #Run the Tetris game in SDL window mode
cargo pixel r tower w                 #Run tower in web,visit http://localhost:8080/ in your browser
cargo pixel r tower w --webport 8081  #Change web server port
cargo pixel r tower w -r              #Run with release mode
cargo pixel r petview g -r            #Run the petview game in winit+glow mode
cargo pixel r petview wg -r           #Run the petview game in winit+wgpu mode
``` 

You can also use cargo pixel to create your own game or app:
```
cargo pixel c mygame           #Create mygame in ./apps using apps/template as a template
```
Creat a standalone app in some directory:
```
cargo pixel c myapp ..  #Create a standalone crate in ../myapp 
cd ../myapp 
cargo pixel r myapp t
cargo pixel r myapp s

```

RustPixel also includes several tools:
1. **palette**: A terminal-ui tool to generate, analyze, convert and manipulate colors.
```
cargo pixel r palette t -r
```
 ![palette](./screen-shot/palette.gif)

2. **tedit**: Used to edit character art assets, example:
``` 
#term mode
cargo pixel r pixel_edit term . assets/logo.txt

#graphics mode
cargo pixel r pixel_edit sdl . assets/logo.pix
```
 ![tedit_t](./screen-shot/tedit_term.png)
 ![tedit_s](./screen-shot/tedit_sdl.png)

3. **tpetii**: Used to convert regular images into PETSCII character art, example:
```
cargo pixel r pixel_petii t assets/a.png -r > assets/a.pix
cargo pixel r pixel_edit s . assets/a.pix
```
```
cargo pixel r pixel_petii t assets/lion.png 40 40 -r > assets/lion.pix
cargo pixel r pixel_edit s . assets/lion.pix
```
 ![tpetii_1](./screen-shot/a.png)
 ![tpetii_2](./screen-shot/lion.png)

4. Script to automatically **convert gif images into PETSCII animations (.ssf)**
```
cargo pixel cg assets/sdq/fire.gif assets/sdq/fire.ssf 40 25 
cargo pixel r pixel_ssf wg -r . assets/sdq/fire.ssf       # preview ssf 
```

### Demo games
1. snake: A snake game with a cool PETSCII animations
```
#graphics mode
cargo pixel r snake s -r
```

![graphics mode](./screen-shot/snake_sdl.gif)

``` 
#term mode
cargo pixel r snake t -r
```

```
#web mode
cargo pixel r snake w -r
#and visit http://localhost:8080/ in your browser
```

2. tetris: A Tetris game where you can play against AI
``` 
#term mode
cargo pixel r tetris t -r
```

 ![term mode](./screen-shot/tetris_term.gif)

```
#graphics mode
cargo pixel r tetris s -r
```

![graphics mode](./screen-shot/tetris_sdl.gif)

```
#web mode
cargo pixel r tetris w -r
#and visit http://localhost:8080/ in your browser
```

![web mode](./screen-shot/tetris_web.gif)

3. poker: Includes the core algorithms for Texas Hold'em and Gin Rummy
``` 
cargo pixel r poker t -r
cargo pixel r gin_rummy t -r
```
 ![gin_rummy](./screen-shot/ginrummy.png)
 ![red_black](./screen-shot/redblack.png)

The poker/ffi directory demo how to wrap Rust algorithms into CFFI for use with other languages, showcasing C++ and Python calling poker_ffi
```
cd apps/poker/ffi
make run
```
The poker/wasm directory demo how to wrap Rust algorithms into wasm for JS calling
```
cd apps/poker/wasm
make run
```

4. tower: A tower defense game prototype demonstrating the use of objpool and pixel_sprite for pixel-perfect sprite movement
``` 
#graphics mode
cargo pixel r tower s -r

#web mode
cargo pixel r tower w -r
#and visit http://localhost:8080/ in your browser
```
 ![tower](./screen-shot/tower_sdl.gif)

and so on ... ...



