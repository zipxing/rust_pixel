[Readme]: ../README.md
[Principle]: principle.md

Please read the [Readme] first and install `rust_pixel` then read the [Principle] to understand the basic concepts

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
```
fn main() {
    // block::run is defined in lib.rs
    block::run()
}
```

If the project only runs in graphics mode, you can use macro:
```
fn main() {
    // if not graphics mode, exit() will be call
    rust_pixel::only_graphical_mode!();

    // if not terminal mode, exit() will be call
    // rust_pixel::only_terminal_mode!();

    // use cfg avoid "unreachable" compile warning
    #[cfg(any(feature = "sdl", target_arch = "wasm32"))]
    block::run()
}
```

- src/lib.rs is the main code logic entry
```
mod model;
mod render;

use pixel_macro::pixel_game;
pixel_game!(Block, "app", ".");
```
