extern crate proc_macro;

use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, Ident, LitStr};

#[proc_macro]
pub fn pixel_game(input: TokenStream) -> TokenStream {
    let name = parse_macro_input!(input as Ident);

    let game_name = Ident::new(&format!("{}Game", name), name.span());
    let model_name = Ident::new(&format!("{}Model", name), name.span());
    let render_name = Ident::new(&format!("{}Render", name), name.span());

    let game_name_str = format!("{}", name);
    let game_name_lit = LitStr::new(&game_name_str, name.span());

    let expanded = quote! {
            #[cfg_attr(target_arch = "wasm32", wasm_bindgen)]
            pub struct #game_name {
                g: Game<#model_name, #render_name>,
            }

            pub fn init_game() -> #game_name {
                let m = #model_name::new();
                let r = #render_name::new();
                let mut g = Game::new(m, r, #game_name_lit);
                g.init();
                #game_name { g }
            }

            #[cfg(target_arch = "wasm32")]
            #[cfg_attr(target_arch = "wasm32", wasm_bindgen)]
            impl #game_name {
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

                pub fn on_asset_loaded(&mut self, url: &str, data: &[u8]) {
                    // info!("asset({:?}): {:?}!!!", url, data);
                    self.g.context.asset_manager.set_data(url, data);
                }

                fn get_wb(&mut self) -> &Vec<RenderCell> {
                    &self
                        .g
                        .context
                        .adapter
                        .get_base()
                        .rbuf
                }

                pub fn web_buffer_len(&mut self) -> usize {
                    self.get_wb().len()
                }

                pub fn web_cell_len(&self) -> usize {
                    std::mem::size_of::<RenderCell>() / 4
                }

                pub fn get_ratiox(&mut self) -> f32 {
                    self.g.context.adapter.get_base().ratio_x
                }

                pub fn get_ratioy(&mut self) -> f32 {
                    self.g.context.adapter.get_base().ratio_y
                }

                // web renders buffer, can be accessed in js using the following
                // const wbuflen = sg.web_buffer_len();
                // const wbufptr = sg.web_buffer();
                // let webbuf = new Uint32Array(wasm.memory.buffer, wbufptr, wbuflen);
                pub fn web_buffer(&mut self) -> *const RenderCell {
                    self.get_wb().as_slice().as_ptr()
                }
            }

            pub fn run() -> Result<(), JsValue> {
                let mut g = init_game().g;
                g.run().unwrap();
                g.render.panel.reset(&mut g.context);
                Ok(())
            }
    };

    TokenStream::from(expanded)
}
