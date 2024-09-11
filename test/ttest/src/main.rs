use glam::{Vec2, Vec3, Vec4};
use std::f32::consts::PI;

fn linear_ease(begin: f32, change: f32, duration: f32, time: f32) -> f32 {
    change * time / duration + begin
}

fn exponential_ease_in_out(begin: f32, change: f32, duration: f32, time: f32) -> f32 {
    if time == 0.0 {
        return begin;
    } else if time == duration {
        return begin + change;
    }
    let time = time / (duration / 2.0);
    if time < 1.0 {
        return change / 2.0 * (2.0_f32.powf(10.0 * (time - 1.0))) + begin;
    }
    return change / 2.0 * (-2.0_f32.powf(-10.0 * (time - 1.0)) + 2.0) + begin;
}

fn sinusoidal_ease_in_out(begin: f32, change: f32, duration: f32, time: f32) -> f32 {
    -change / 2.0 * ((PI * time / duration).cos() - 1.0) + begin
}

fn rand(co: Vec2) -> f32 {
    (co.dot(Vec2::new(12.9898, 78.233)).sin() * 43758.5453).fract()
}

fn cross_fade(uv: Vec2, dissolve: f32, from_color: Vec3, to_color: Vec3) -> Vec3 {
    from_color.lerp(to_color, dissolve)
}

fn transition(uv: Vec2, progress: f32, strength: f32, from_color_fn: fn(Vec2) -> Vec3, to_color_fn: fn(Vec2) -> Vec3) -> Vec4 {
    let tex_coord = uv / Vec2::ONE;

    // 线性插值图像中心在图像中间移动
    let center = Vec2::new(linear_ease(0.25, 0.5, 1.0, progress), 0.5);
    let dissolve = exponential_ease_in_out(0.0, 1.0, 1.0, progress);

    // 镜像正弦循环 0->strength 然后 strength->0
    let strength = sinusoidal_ease_in_out(0.0, strength, 0.5, progress);

    let mut color = Vec3::ZERO;
    let mut total = 0.0;
    let to_center = center - tex_coord;

    // 随机化查找值以隐藏固定数量的样本
    let offset = rand(uv);

    for t in 0..=40 {
        let percent = (t as f32 + offset) / 40.0;
        let weight = 4.0 * (percent - percent * percent);
        let sample_color = cross_fade(
            tex_coord + to_center * percent * strength,
            dissolve,
            from_color_fn(tex_coord),
            to_color_fn(tex_coord),
        );
        color += sample_color * weight;
        total += weight;
    }
    color /= total;
    Vec4::new(color[0], color[1], color[2], 1.0)
}

// 定义 Vec3 和 Vec4，Vec2 的 from_color_fn 和 to_color_fn 实现略过，可以根据需要实现。
fn main() {
}
