use wasm_bindgen::prelude::*;
#[cfg(any(feature = "sdl", target_arch = "wasm32"))]
use tower::run;

fn main() -> Result<(), JsValue> {
    #[cfg(not(any(feature = "sdl", target_arch = "wasm32")))]
    {
        println!("Run in graphics only...");
        Ok(())
    }
    #[cfg(any(feature = "sdl", target_arch = "wasm32"))]
    run()
}
