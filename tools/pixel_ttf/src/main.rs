use fontdue::Font;
use fontdue::FontSettings;

fn main() {
    // 自定义位图尺寸
    let n = 8; // 例如，16x16 位图

    // 1. 加载字体文件
    let font_data = std::fs::read("assets/pixel8.ttf").expect("无法读取字体文件");
    let font = Font::from_bytes(font_data, FontSettings::default()).expect("无法解析字体");

    // 2. 定义 Unicode 范围（可根据需要调整）
    let start_codepoint = 0x00;   // 从空格开始
    let end_codepoint = 0xFF;     // 到 '~' 结束

    for codepoint in start_codepoint..=end_codepoint {
        if let Some(character) = char::from_u32(codepoint) {
            // 检查字体是否包含该字符
            if font.lookup_glyph_index(character) != 0 {
                // 3. 渲染字符为 nxn 位图
                let (metrics, bitmap) = font.rasterize(character, n as f32);

                // 将位图调整为 nxn（如果渲染结果不是 nxn，需要进行缩放或裁剪）
                let bitmap_nxn = resize_bitmap(&bitmap, metrics.width, metrics.height, n, n);

                // 4. 输出点阵数据
                println!("字符: '{}'", character);
                print_bitmap(&bitmap_nxn);
            }
        }
    }
}

// 辅助函数：调整位图大小为 nxn
fn resize_bitmap(bitmap: &[u8], width: usize, height: usize, new_width: usize, new_height: usize) -> Vec<Vec<u8>> {
    let mut resized = vec![vec![0u8; new_width]; new_height];
    for y in 0..new_height {
        for x in 0..new_width {
            let src_x = x * width / new_width;
            let src_y = y * height / new_height;
            if src_x < width && src_y < height {
                resized[y][x] = bitmap[src_y * width + src_x];
            }
        }
    }
    resized
}

// 辅助函数：打印位图
fn print_bitmap(bitmap: &[Vec<u8>]) {
    for row in bitmap {
        for &pixel in row {
            if pixel > 128 {
                print!("█");
            } else {
                print!(" ");
            }
        }
        println!();
    }
}

