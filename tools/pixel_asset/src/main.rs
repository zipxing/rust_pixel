use std::fs;
use std::path::Path;
use image::{DynamicImage, GenericImageView, ImageBuffer, RgbaImage};
use image::GenericImage;
use image::imageops::FilterType;

#[derive(Clone, Copy, Debug)]
struct Rectangle {
    x: u32,
    y: u32,
    width: u32,
    height: u32,
}

struct MaxRectsBin {
    width: u32,
    height: u32,
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
            width,
            height,
            free_rects: vec![initial_rect],
            used_rects: Vec::new(),
        }
    }

    fn insert(&mut self, width: u32, height: u32) -> Option<Rectangle> {
        // 选择最佳的空闲矩形
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
            // 无法放置矩形
            None
        }
    }

    fn find_position_for_new_node_best_area_fit(&self, width: u32, height: u32) -> Option<Rectangle> {
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
        !(a.x + a.width <= b.x || a.x >= b.x + b.width || a.y + a.height <= b.y || a.y >= b.y + b.height)
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
        a.x >= b.x && a.y >= b.y && a.x + a.width <= b.x + b.width && a.y + a.height <= b.y + b.height
    }
}

fn adjust_size_to_multiple_of_eight(width: u32, height: u32) -> (u32, u32) {
    let adjusted_width = ((width + 7) / 8) * 8;
    let adjusted_height = ((height + 7) / 8) * 8;
    (adjusted_width, adjusted_height)
}

struct ImageRect {
    image: DynamicImage,
    rect: Rectangle,
}

fn main() {
    let folder_path = "./cc";
    let atlas_width = 1024;
    let atlas_height = 1024;

    // 1. 加载小图片
    let mut images = Vec::new();
    let paths = fs::read_dir(folder_path).unwrap();

    for path in paths {
        let file_path = path.unwrap().path();
        if file_path.is_file() {
            println!("{}", file_path.display());
            if let Ok(img) = image::open(&file_path) {
                images.push(img);
            }
        }
    }

    // 2. 创建 MaxRectsBin
    let mut bin = MaxRectsBin::new(atlas_width, atlas_height);

    let mut image_rects = Vec::new();

    for img in images {
        let (orig_width, orig_height) = img.dimensions();
        // 调整尺寸为 8 的倍数
        let (adjusted_width, adjusted_height) = adjust_size_to_multiple_of_eight(orig_width, orig_height);

        // 如果调整后的尺寸与原始尺寸不同，需要填充图片
        let padded_image = if adjusted_width != orig_width || adjusted_height != orig_height {
            let mut padded_image = DynamicImage::new_rgba8(adjusted_width, adjusted_height);
            padded_image.copy_from(&img, 0, 0).unwrap();
            padded_image
        } else {
            img
        };

        let padded_image = padded_image.resize_exact(adjusted_width / 2, adjusted_height / 2, FilterType::Lanczos3);

        // 插入到 bin 中
        if let Some(rect) = bin.insert(adjusted_width / 2, adjusted_height / 2) {
            image_rects.push(ImageRect {
                image: padded_image,
                rect,
            });
        } else {
            println!("无法放置图片，纹理空间不足。");
            // 您可以在此处处理未能放置的图片，例如创建新的纹理
        }
    }

    // 3. 创建大纹理
    let mut atlas = RgbaImage::new(atlas_width, atlas_height);

    for image_rect in &image_rects {
        atlas.copy_from(
            &image_rect.image,
            image_rect.rect.x,
            image_rect.rect.y,
        ).unwrap();
    }

    // 4. 保存大纹理
    atlas.save("texture_atlas.png").unwrap();

    // 5. 记录每个图片的位置和尺寸
    for (i, image_rect) in image_rects.iter().enumerate() {
        println!(
            "图片 {}: 位置=({}, {}), 尺寸={}x{}",
            i,
            image_rect.rect.x,
            image_rect.rect.y,
            image_rect.rect.width,
            image_rect.rect.height
        );
    }
}

