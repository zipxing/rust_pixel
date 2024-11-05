use image::imageops::FilterType;
use image::GenericImage;
use image::{DynamicImage, GenericImageView, RgbaImage};
use std::fs;
use std::env;
use std::io::Write;
use std::fs::File;
use std::path::Path;

#[derive(Clone, Copy, Debug)]
struct Rectangle {
    x: u32,
    y: u32,
    width: u32,
    height: u32,
}

struct MaxRectsBin {
    free_rects: Vec<Rectangle>,
    used_rects: Vec<Rectangle>,
}

impl MaxRectsBin {
    fn new(width: u32, height: u32) -> Self {
        let initial_rect = Rectangle {
            x: 0,
            y: 0,
            width,
            height,
        };
        MaxRectsBin {
            free_rects: vec![initial_rect],
            used_rects: Vec::new(),
        }
    }

    fn insert(&mut self, width: u32, height: u32) -> Option<Rectangle> {
        if let Some(best_rect) = self.find_position_for_new_node_best_area_fit(width, height) {
            let new_node = Rectangle {
                x: best_rect.x,
                y: best_rect.y,
                width,
                height,
            };
            self.place_rectangle(new_node);
            Some(new_node)
        } else {
            None
        }
    }

    fn find_position_for_new_node_best_area_fit(
        &self,
        width: u32,
        height: u32,
    ) -> Option<Rectangle> {
        let mut best_area_fit = u32::MAX;
        let mut best_rect = None;

        for rect in &self.free_rects {
            if width <= rect.width && height <= rect.height {
                let area_fit = rect.width * rect.height - width * height;
                if area_fit < best_area_fit {
                    best_area_fit = area_fit;
                    best_rect = Some(Rectangle {
                        x: rect.x,
                        y: rect.y,
                        width,
                        height,
                    });
                }
            }
        }

        best_rect
    }

    fn place_rectangle(&mut self, rect: Rectangle) {
        self.used_rects.push(rect);

        let mut i = 0;
        while i < self.free_rects.len() {
            if self.split_free_node(self.free_rects[i], rect) {
                self.free_rects.remove(i);
            } else {
                i += 1;
            }
        }

        self.prune_free_list();
    }

    fn split_free_node(&mut self, free_rect: Rectangle, used_rect: Rectangle) -> bool {
        // 如果两矩形不重叠，返回 false
        if !self.is_overlapping(free_rect, used_rect) {
            return false;
        }

        let mut new_rects = Vec::new();

        // 上方
        if used_rect.y > free_rect.y && used_rect.y < free_rect.y + free_rect.height {
            new_rects.push(Rectangle {
                x: free_rect.x,
                y: free_rect.y,
                width: free_rect.width,
                height: used_rect.y - free_rect.y,
            });
        }

        // 下方
        if used_rect.y + used_rect.height < free_rect.y + free_rect.height {
            new_rects.push(Rectangle {
                x: free_rect.x,
                y: used_rect.y + used_rect.height,
                width: free_rect.width,
                height: free_rect.y + free_rect.height - (used_rect.y + used_rect.height),
            });
        }

        // 左侧
        if used_rect.x > free_rect.x && used_rect.x < free_rect.x + free_rect.width {
            new_rects.push(Rectangle {
                x: free_rect.x,
                y: free_rect.y,
                width: used_rect.x - free_rect.x,
                height: free_rect.height,
            });
        }

        // 右侧
        if used_rect.x + used_rect.width < free_rect.x + free_rect.width {
            new_rects.push(Rectangle {
                x: used_rect.x + used_rect.width,
                y: free_rect.y,
                width: free_rect.x + free_rect.width - (used_rect.x + used_rect.width),
                height: free_rect.height,
            });
        }

        for new_rect in new_rects {
            self.free_rects.push(new_rect);
        }

        true
    }

    fn is_overlapping(&self, a: Rectangle, b: Rectangle) -> bool {
        !(a.x + a.width <= b.x
            || a.x >= b.x + b.width
            || a.y + a.height <= b.y
            || a.y >= b.y + b.height)
    }

    fn prune_free_list(&mut self) {
        let mut i = 0;
        while i < self.free_rects.len() {
            let mut j = i + 1;
            while j < self.free_rects.len() {
                if self.is_contained_in(self.free_rects[i], self.free_rects[j]) {
                    self.free_rects.remove(i);
                    i -= 1;
                    break;
                } else if self.is_contained_in(self.free_rects[j], self.free_rects[i]) {
                    self.free_rects.remove(j);
                } else {
                    j += 1;
                }
            }
            i += 1;
        }
    }

    fn is_contained_in(&self, a: Rectangle, b: Rectangle) -> bool {
        a.x >= b.x
            && a.y >= b.y
            && a.x + a.width <= b.x + b.width
            && a.y + a.height <= b.y + b.height
    }
}

fn adjust_size_to_multiple_of_eight(width: u32, height: u32) -> (u32, u32) {
    let adjusted_width = ((width + 7) / 8) * 8;
    let adjusted_height = ((height + 7) / 8) * 8;
    (adjusted_width, adjusted_height)
}

struct ImageRect {
    path: String,
    image: DynamicImage,
    rect: Rectangle,
}

fn main() {
    let args: Vec<String> = env::args().collect();

    let folder_path: &str;
    let dst_dir: &str;

    match args.len() {
        3 => {
            folder_path = &args[1];
            dst_dir = &args[2];
        }
        _ => {
            return;
        }
    }

    let rawimage = image::open("assets/pix/c64.png").unwrap();
    let atlas_width = 1024;
    let atlas_height = 1024 - 128;

    let mut images = Vec::new();
    let paths = fs::read_dir(folder_path).unwrap();

    for path in paths {
        let file_path = path.unwrap().path();
        if file_path.is_file() {
            println!("{}", file_path.display());
            if let Ok(img) = image::open(&file_path) {
                images.push((file_path.file_name().unwrap().to_str().unwrap().to_string(), img));
            }
        }
    }

    let mut bin = MaxRectsBin::new(atlas_width, atlas_height);
    let mut image_rects = Vec::new();
    for img in images {
        let (orig_width, orig_height) = img.1.dimensions();
        let (adjusted_width, adjusted_height) =
            adjust_size_to_multiple_of_eight(orig_width, orig_height);

        let padded_image = if adjusted_width != orig_width || adjusted_height != orig_height {
            let mut padded_image = DynamicImage::new_rgba8(adjusted_width, adjusted_height);
            padded_image.copy_from(&img.1, 0, 0).unwrap();
            (img.0, padded_image)
        } else {
            img
        };

        let padded_image = (
            padded_image.0,
            padded_image.1.resize_exact(
                adjusted_width / 2,
                adjusted_height / 2,
                FilterType::Lanczos3,
            ),
        );

        if let Some(rect) = bin.insert(adjusted_width / 2, adjusted_height / 2) {
            image_rects.push(ImageRect {
                path: padded_image.0.to_string(),
                image: padded_image.1,
                rect,
            });
        } else {
            println!("No Space");
        }
    }

    let mut atlas = RgbaImage::new(atlas_width, atlas_height + 128);
    atlas.copy_from(&rawimage, 0, 0).unwrap();

    for image_rect in &image_rects {
        atlas
            .copy_from(&image_rect.image, image_rect.rect.x, image_rect.rect.y + 128)
            .unwrap();
    }
    atlas.save(&format!("{}/texture_atlas.png", dst_dir)).unwrap();

    for (_i, image_rect) in image_rects.iter().enumerate() {
        let x0 = image_rect.rect.x / 8;
        let y0 = image_rect.rect.y / 8;
        let w = image_rect.rect.width / 8;
        let h = image_rect.rect.height / 8;
        let pathp = Path::new(&format!("{}/{}", dst_dir, image_rect.path)).with_extension("pix");
        let mut file = File::create(pathp).unwrap();
        let line = &format!("width={},height={},texture=255\n", w, h);
        file.write_all(line.as_bytes()).unwrap();

        for a in 0..h {
            for b in 0..w {
                let x = x0 + b;
                let y = y0 + a;
                let s = y % 16 * 16 + x % 16;
                let t = y / 16 * 8 + x / 16;
                let line = &format!("{},{},{},{} ", s, 15, t + 8, 0);
                file.write_all(line.as_bytes()).unwrap();
            }
            file.write_all("\n".as_bytes()).unwrap();
        }
    }
}
