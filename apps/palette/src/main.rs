use wasm_bindgen::prelude::*;
#[cfg(not(any(feature = "sdl", target_arch = "wasm32")))]
use palette::run;

fn main() -> Result<(), JsValue> {
    #[cfg(any(feature = "sdl", target_arch = "wasm32"))]
    {
        println!("Run in terminal only...");
        return Ok(());
    }
    #[cfg(not(any(feature = "sdl", target_arch = "wasm32")))]
    run()
}
