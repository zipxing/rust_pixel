use rust_pixel::pixel_game;

// pixel_game! macro will automatically include model.rs, render_terminal.rs, render_graphics.rs
// and create the appropriate type aliases and modules
pixel_game!(BasicSnake);
