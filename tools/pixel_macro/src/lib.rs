extern crate proc_macro;

use proc_macro::TokenStream;
use quote::quote;
use syn::parse::{Parse, ParseStream};
use syn::punctuated::Punctuated;
use syn::{parse_macro_input, Expr, Ident, LitStr, Result};

struct VariadicInput {
    exprs: Punctuated<Expr, syn::Token![,]>,
}

impl Parse for VariadicInput {
    fn parse(input: ParseStream) -> Result<Self> {
        let exprs = Punctuated::parse_terminated(input)?;
        Ok(VariadicInput { exprs })
    }
}

#[proc_macro]
pub fn pixel_game(input: TokenStream) -> TokenStream {
    let VariadicInput { exprs } = parse_macro_input!(input as VariadicInput);
    let name = exprs.into_iter().next().unwrap();

    let name_ident = if let Expr::Path(expr_path) = &name {
        expr_path
            .path
            .get_ident()
            .cloned()
            .expect("Expected an identifier")
    } else {
        panic!("Expected an identifier as the first argument");
    };

    let game_name = Ident::new(&format!("{}Game", name_ident), name_ident.span());
    let model_name = Ident::new(&format!("{}Model", name_ident), name_ident.span());
    let render_name = Ident::new(&format!("{}Render", name_ident), name_ident.span());

    let game_name_str = {
        let ns = format!("{}", name_ident);
        ns.to_lowercase().to_string()
    };
    let game_name_lit = LitStr::new(&game_name_str, name_ident.span());

    let expanded = quote! {
            mod model;
            #[cfg(not(any(feature = "sdl", target_arch = "wasm32")))]
            mod render_terminal;
            #[cfg(any(feature = "sdl", target_arch = "wasm32"))]
            mod render_graphics;

            #[cfg(not(any(feature = "sdl", target_arch = "wasm32")))]
            use crate::{model::#model_name, render_terminal::#render_name};
            #[cfg(any(feature = "sdl", target_arch = "wasm32"))]
            use crate::{model::#model_name, render_graphics::#render_name};
            use rust_pixel::game::Game;
            use rust_pixel::util::get_project_path;

            #[cfg(target_arch = "wasm32")]
            use rust_pixel::render::adapter::web::{input_events_from_web, WebAdapter};
            use wasm_bindgen::prelude::*;
            #[cfg(target_arch = "wasm32")]
            use wasm_bindgen_futures::js_sys;
            #[cfg(target_arch = "wasm32")]
            use log::info;

            #[cfg_attr(target_arch = "wasm32", wasm_bindgen)]
            pub struct #game_name {
                g: Game<#model_name, #render_name>,
            }

            pub fn init_game() -> #game_name {
                let m = #model_name::new();
                let r = #render_name::new();
                let pp = get_project_path();
                println!("asset path : {:?}", pp);
                let mut g = Game::new(m, r, #game_name_lit, &pp);
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
                    if let Some(pe) = input_events_from_web(t, e, abase.pixel_h, abase.ratio_x, abase.ratio_y) {
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

            pub fn run() {
                let mut g = init_game().g;
                g.run().unwrap();
                g.render.panel.reset(&mut g.context);
            }
    };

    TokenStream::from(expanded)
}
