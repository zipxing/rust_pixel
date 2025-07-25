// RustPixel
// copyright zipxing@hotmail.com 2022～2025

use crate::render::style::color_pro::*;

pub fn delta_e_cie76(lab1: ColorData, lab2: ColorData) -> f64 {
    ((lab1.v[0] - lab2.v[0]).powi(2)
        + (lab1.v[1] - lab2.v[1]).powi(2)
        + (lab1.v[2] - lab2.v[2]).powi(2))
    .sqrt()
}

fn deg_to_rad(deg: f64) -> f64 {
    deg * PI / 180.0
}

fn rad_to_deg(rad: f64) -> f64 {
    rad * 180.0 / PI
}

pub fn delta_e_ciede2000(lab1: ColorData, lab2: ColorData) -> f64 {
    let k_l = 1.0;
    let k_c = 1.0;
    let k_h = 1.0;

    let delta_l_prime = lab2.v[0] - lab1.v[0];
    let l_bar = (lab1.v[0] + lab2.v[0]) / 2.0;
    let c1 = (lab1.v[1].powi(2) + lab1.v[2].powi(2)).sqrt();
    let c2 = (lab2.v[1].powi(2) + lab2.v[2].powi(2)).sqrt();
    let c_bar = (c1 + c2) / 2.0;
    let g = 0.5 * (1.0 - (c_bar.powi(7) / (c_bar.powi(7) + 25.0_f64.powi(7))).sqrt());

    let a1_prime = lab1.v[1] * (1.0 + g);
    let a2_prime = lab2.v[1] * (1.0 + g);
    let c1_prime = (a1_prime.powi(2) + lab1.v[2].powi(2)).sqrt();
    let c2_prime = (a2_prime.powi(2) + lab2.v[2].powi(2)).sqrt();
    let c_bar_prime = (c1_prime + c2_prime) / 2.0;
    let delta_c_prime = c2_prime - c1_prime;

    let h1_prime = rad_to_deg(lab1.v[2].atan2(a1_prime)).rem_euclid(360.0);
    let h2_prime = rad_to_deg(lab2.v[2].atan2(a2_prime)).rem_euclid(360.0);
    let delta_h_prime = if (c1_prime * c2_prime).abs() < 1e-4 {
        0.0
    } else if (h2_prime - h1_prime).abs() <= 180.0 {
        h2_prime - h1_prime
    } else if h2_prime <= h1_prime {
        h2_prime - h1_prime + 360.0
    } else {
        h2_prime - h1_prime - 360.0
    };
    let delta_h_prime_radians = deg_to_rad(delta_h_prime);
    let delta_h_prime_2 = 2.0 * (c1_prime * c2_prime).sqrt() * (delta_h_prime_radians / 2.0).sin();

    let h_bar_prime = if (c1_prime * c2_prime).abs() < 1e-4 {
        h1_prime + h2_prime
    } else if (h1_prime - h2_prime).abs() <= 180.0 {
        (h1_prime + h2_prime) / 2.0
    } else if h1_prime + h2_prime < 360.0 {
        (h1_prime + h2_prime + 360.0) / 2.0
    } else {
        (h1_prime + h2_prime - 360.0) / 2.0
    };
    let t = 1.0 - 0.17 * (deg_to_rad(h_bar_prime - 30.0)).cos()
        + 0.24 * (deg_to_rad(2.0 * h_bar_prime)).cos()
        + 0.32 * (deg_to_rad(3.0 * h_bar_prime + 6.0)).cos()
        - 0.20 * (deg_to_rad(4.0 * h_bar_prime - 63.0)).cos();
    let s_l = 1.0 + (0.015 * (l_bar - 50.0).powi(2) / (20.0 + (l_bar - 50.0).powi(2)).sqrt());
    let s_c = 1.0 + 0.045 * c_bar_prime;
    let s_h = 1.0 + 0.015 * c_bar_prime * t;
    let r_t = -2.0 * (deg_to_rad(60.0 * (-((h_bar_prime - 275.0) / 25.0).powi(2)).exp())).sin();

    ((delta_l_prime / (k_l * s_l)).powi(2)
        + (delta_c_prime / (k_c * s_c)).powi(2)
        + (delta_h_prime_2 / (k_h * s_h)).powi(2)
        + r_t * (delta_c_prime / (k_c * s_c)) * (delta_h_prime_2 / (k_h * s_h)))
        .sqrt()
}


