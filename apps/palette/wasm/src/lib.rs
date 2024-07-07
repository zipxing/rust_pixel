use palette_lib::PaletteData;
use wasm_bindgen::prelude::*;
use web_sys::js_sys;

#[cfg_attr(target_arch = "wasm32", wasm_bindgen)]
pub struct WasmPalette {
    gcs: PaletteData,
    webbuf: Vec<u8>,
}

#[cfg(target_arch = "wasm32")]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen)]
impl WasmPalette {
    // js调用创建game结构
    pub fn new() -> Self {
       let gcs = PaletteData::new();
       Self {
           gcs,
           webbuf: vec![],
       }
    }
    
    pub fn shuffle(&mut self) {
        self.gcs.shuffle(); 
    }

    pub fn next(&mut self) {
        self.webbuf.clear();
        let cs = self.gcs.next();
        self.webbuf.push(cs);
    }

    pub fn web_buffer_len(&self) -> usize {
        self.webbuf.len()
    }

    pub fn web_buffer(&self) -> *const u8 {
        self.webbuf.as_slice().as_ptr()
    }
}

