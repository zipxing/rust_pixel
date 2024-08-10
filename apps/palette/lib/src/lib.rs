//
// implement core algorithm...
//
#![allow(dead_code)]
use lazy_static::lazy_static;
use rust_pixel::render::style::{delta_e_ciede2000, ColorPro, ColorGradient, ColorSpace::*, Fraction};
use rust_pixel::util::Rand;
use std::collections::HashMap;
use log::info;

static COLORS_RGB_WITH_NAME: [(&'static str, u8, u8, u8); 139] = [
    ("aliceblue", 240, 248, 255),
    ("antiquewhite", 250, 235, 215),
    ("aqua", 0, 255, 255),
    ("aquamarine", 127, 255, 212),
    ("azure", 240, 255, 255),
    ("beige", 245, 245, 220),
    ("bisque", 255, 228, 196),
    ("black", 0, 0, 0),
    ("blanchedalmond", 255, 235, 205),
    ("blue", 0, 0, 255),
    ("blueviolet", 138, 43, 226),
    ("brown", 165, 42, 42),
    ("burlywood", 222, 184, 135),
    ("cadetblue", 95, 158, 160),
    ("chartreuse", 127, 255, 0),
    ("chocolate", 210, 105, 30),
    ("coral", 255, 127, 80),
    ("cornflowerblue", 100, 149, 237),
    ("cornsilk", 255, 248, 220),
    ("crimson", 220, 20, 60),
    ("darkblue", 0, 0, 139),
    ("darkcyan", 0, 139, 139),
    ("darkgoldenrod", 184, 134, 11),
    ("darkgray", 169, 169, 169),
    ("darkgreen", 0, 100, 0),
    ("darkkhaki", 189, 183, 107),
    ("darkmagenta", 139, 0, 139),
    ("darkolivegreen", 85, 107, 47),
    ("darkorange", 255, 140, 0),
    ("darkorchid", 153, 50, 204),
    ("darkred", 139, 0, 0),
    ("darksalmon", 233, 150, 122),
    ("darkseagreen", 143, 188, 143),
    ("darkslateblue", 72, 61, 139),
    ("darkslategray", 47, 79, 79),
    ("darkturquoise", 0, 206, 209),
    ("darkviolet", 148, 0, 211),
    ("deeppink", 255, 20, 147),
    ("deepskyblue", 0, 191, 255),
    ("dimgray", 105, 105, 105),
    ("dodgerblue", 30, 144, 255),
    ("firebrick", 178, 34, 34),
    ("floralwhite", 255, 250, 240),
    ("forestgreen", 34, 139, 34),
    ("fuchsia", 255, 0, 255),
    ("gainsboro", 220, 220, 220),
    ("ghostwhite", 248, 248, 255),
    ("gold", 255, 215, 0),
    ("goldenrod", 218, 165, 32),
    ("gray", 128, 128, 128),
    ("green", 0, 128, 0),
    ("greenyellow", 173, 255, 47),
    ("honeydew", 240, 255, 240),
    ("hotpink", 255, 105, 180),
    ("indianred", 205, 92, 92),
    ("indigo", 75, 0, 130),
    ("ivory", 255, 255, 240),
    ("khaki", 240, 230, 140),
    ("lavender", 230, 230, 250),
    ("lavenderblush", 255, 240, 245),
    ("lawngreen", 124, 252, 0),
    ("lemonchiffon", 255, 250, 205),
    ("lightblue", 173, 216, 230),
    ("lightcoral", 240, 128, 128),
    ("lightcyan", 224, 255, 255),
    ("lightgoldenrodyellow", 250, 250, 210),
    ("lightgray", 211, 211, 211),
    ("lightgreen", 144, 238, 144),
    ("lightpink", 255, 182, 193),
    ("lightsalmon", 255, 160, 122),
    ("lightseagreen", 32, 178, 170),
    ("lightskyblue", 135, 206, 250),
    ("lightslategray", 119, 136, 153),
    ("lightsteelblue", 176, 196, 222),
    ("lightyellow", 255, 255, 224),
    ("lime", 0, 255, 0),
    ("limegreen", 50, 205, 50),
    ("linen", 250, 240, 230),
    ("maroon", 128, 0, 0),
    ("mediumaquamarine", 102, 205, 170),
    ("mediumblue", 0, 0, 205),
    ("mediumorchid", 186, 85, 211),
    ("mediumpurple", 147, 112, 219),
    ("mediumseagreen", 60, 179, 113),
    ("mediumslateblue", 123, 104, 238),
    ("mediumspringgreen", 0, 250, 154),
    ("mediumturquoise", 72, 209, 204),
    ("mediumvioletred", 199, 21, 133),
    ("midnightblue", 25, 25, 112),
    ("mintcream", 245, 255, 250),
    ("mistyrose", 255, 228, 225),
    ("moccasin", 255, 228, 181),
    ("navajowhite", 255, 222, 173),
    ("navy", 0, 0, 128),
    ("oldlace", 253, 245, 230),
    ("olive", 128, 128, 0),
    ("olivedrab", 107, 142, 35),
    ("orange", 255, 165, 0),
    ("orangered", 255, 69, 0),
    ("orchid", 218, 112, 214),
    ("palegoldenrod", 238, 232, 170),
    ("palegreen", 152, 251, 152),
    ("paleturquoise", 175, 238, 238),
    ("palevioletred", 219, 112, 147),
    ("papayawhip", 255, 239, 213),
    ("peachpuff", 255, 218, 185),
    ("peru", 205, 133, 63),
    ("pink", 255, 192, 203),
    ("plum", 221, 160, 221),
    ("powderblue", 176, 224, 230),
    ("purple", 128, 0, 128),
    ("rebeccapurple", 102, 51, 153),
    ("red", 255, 0, 0),
    ("rosybrown", 188, 143, 143),
    ("royalblue", 65, 105, 225),
    ("saddlebrown", 139, 69, 19),
    ("salmon", 250, 128, 114),
    ("sandybrown", 244, 164, 96),
    ("seagreen", 46, 139, 87),
    ("seashell", 255, 245, 238),
    ("sienna", 160, 82, 45),
    ("silver", 192, 192, 192),
    ("skyblue", 135, 206, 235),
    ("slateblue", 106, 90, 205),
    ("slategray", 112, 128, 144),
    ("snow", 255, 250, 250),
    ("springgreen", 0, 255, 127),
    ("steelblue", 70, 130, 180),
    ("tan", 210, 180, 140),
    ("teal", 0, 128, 128),
    ("thistle", 216, 191, 216),
    ("tomato", 255, 99, 71),
    ("turquoise", 64, 224, 208),
    ("violet", 238, 130, 238),
    ("wheat", 245, 222, 179),
    ("white", 255, 255, 255),
    ("whitesmoke", 245, 245, 245),
    ("yellow", 255, 255, 0),
    ("yellowgreen", 154, 205, 50),
];

lazy_static! {
    pub static ref COLORS_WITH_NAME: Vec<(&'static str, ColorPro)> = {
        let mut ncolors = vec![];
        for c in COLORS_RGB_WITH_NAME {
            let cr = ColorPro::from_space_u8(SRGBA, c.1, c.2, c.3, 255);
            ncolors.push((c.0, cr));
        }
        ncolors
    };
    pub static ref COLORS_WITH_NAME_RGB_INDEX: HashMap<(u8, u8, u8, u8), usize> = {
        let mut rgb_index = HashMap::new();
        for (i, sc) in COLORS_WITH_NAME.iter().enumerate() {
            let rgb = sc.1.get_srgba_u8();
            rgb_index.insert(rgb, i);
        }
        rgb_index
    }; 
}

pub fn find_similar_colors(color: &ColorPro) -> (usize, usize, usize) {
    let mut deltas: Vec<(usize, f64)> = vec![];
    for idx in 0..COLORS_WITH_NAME.len() {
        let c = COLORS_WITH_NAME[idx];
        let d = delta_e_ciede2000(color[LabA].unwrap(), c.1[LabA].unwrap());
        deltas.push((idx, d));
    }
    deltas.sort_by_key(|nc| (nc.1 * 1000.0) as i32);
    if deltas[0].1 == 0.0 {
        (deltas[1].0, deltas[2].0, deltas[3].0)
    } else {
        (deltas[0].0, deltas[1].0, deltas[2].0)
    }
}

pub fn gradient(colors: &Vec<ColorPro>, gcount: usize, output_colors: &mut Vec<ColorPro>) {
    let color_count = colors.len();
    output_colors.clear();
    if color_count < 2 {
        return;
    }
    let mut color_scale = ColorGradient::empty();

    for (i, color) in colors.into_iter().enumerate() {
        let position = Fraction::from(i as f64 / (color_count as f64 - 1.0));
        color_scale.add_stop(*color, position);
    }
    info!("color_stop.....{:?}", color_scale);
    for i in 0..gcount {
        let position = Fraction::from(i as f64 / (gcount as f64 - 1.0));
        let color = color_scale
            .sample(position, OKLchA)
            .expect("gradient color");
        let cp = ColorPro::from_space(OKLchA, color);
        output_colors.push(cp);
    }
}

pub struct PaletteData {
    pub rand: Rand,
    pub pool: Vec<u8>,
    pub index: usize,
}

impl PaletteData {
    pub fn new() -> Self {
        let mut rd = Rand::new();
        rd.srand_now();
        Self {
            rand: rd,
            pool: vec![],
            index: 0,
        }
    }

    pub fn shuffle(&mut self) {
        self.pool.clear();
        for i in 1..=52u8 {
            self.pool.push(i);
        }
        self.rand.shuffle(&mut self.pool);
        // println!("shuffle ok...");
    }

    pub fn next(&mut self) -> u8 {
        let ret;
        if self.pool.len() > 0 {
            ret = self.pool[self.index];
            self.index = (self.index + 1) % 52;
        } else {
            ret = 0;
        }
        ret
    }
}

#[cfg(test)]
mod tests {
    // use super::*;
    #[test]
    fn it_works() {
        // let result = PaletteData::new();
    }
}
