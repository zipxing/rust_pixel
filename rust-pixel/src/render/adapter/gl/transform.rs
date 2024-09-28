// RustPixel
// copyright zipxing@hotmail.com 2022~2024

#[derive(Clone, Copy)]
pub struct GlTransform {
    pub m00: f32,
    pub m10: f32,
    pub m20: f32,
    pub m01: f32,
    pub m11: f32,
    pub m21: f32,
}

impl GlTransform {
    pub fn new() -> Self {
        Self {
            m00: 1.0,
            m10: 0.0,
            m20: 0.0,
            m01: 0.0,
            m11: 1.0,
            m21: 0.0,
        }
    }

    pub fn new_with_values(a: f32, b: f32, c: f32, d: f32, e: f32, f: f32) -> Self {
        Self {
            m00: a,
            m10: b,
            m20: c,
            m01: d,
            m11: e,
            m21: f,
        }
    }

    pub fn identity(&mut self) {
        self.m00 = 1.0;
        self.m10 = 0.0;
        self.m20 = 0.0;
        self.m01 = 0.0;
        self.m11 = 1.0;
        self.m21 = 0.0;
    }

    pub fn set(&mut self, other: &GlTransform) {
        *self = *other;
    }

    pub fn copy(&self) -> Self {
        *self
    }

    pub fn multiply(&mut self, other: &GlTransform) {
        let m00 = self.m00 * other.m00 + self.m10 * other.m01;
        let m10 = self.m00 * other.m10 + self.m10 * other.m11;
        let m20 = self.m20 + self.m00 * other.m20 + self.m10 * other.m21;
        let m01 = self.m01 * other.m00 + self.m11 * other.m01;
        let m11 = self.m01 * other.m10 + self.m11 * other.m11;
        let m21 = self.m21 + self.m01 * other.m20 + self.m11 * other.m21;

        self.m00 = m00;
        self.m10 = m10;
        self.m20 = m20;
        self.m01 = m01;
        self.m11 = m11;
        self.m21 = m21;
    }

    pub fn translate(&mut self, x: f32, y: f32) {
        self.m20 += self.m00 * x + self.m10 * y;
        self.m21 += self.m01 * x + self.m11 * y;
    }

    pub fn rotate(&mut self, angle: f32) {
        let cos = angle.cos();
        let sin = angle.sin();

        let m00 = self.m00;
        let m01 = self.m01;

        self.m00 = m00 * cos - self.m10 * sin;
        self.m10 = m00 * sin + self.m10 * cos;
        self.m01 = m01 * cos - self.m11 * sin;
        self.m11 = m01 * sin + self.m11 * cos;
    }

    pub fn shear(&mut self, x: f32, y: f32) {
        let m00 = self.m00;
        let m01 = self.m01;

        self.m00 += self.m10 * y;
        self.m10 += m00 * x;
        self.m01 += self.m11 * y;
        self.m11 += m01 * x;
    }

    pub fn scale(&mut self, x: f32, y: f32) {
        self.m00 *= x;
        self.m10 *= y;
        self.m01 *= x;
        self.m11 *= y;
    }

    pub fn invert(&mut self) {
        let s00 = self.m00;
        let s10 = self.m10;
        let s20 = self.m20;
        let s01 = self.m01;
        let s11 = self.m11;
        let s21 = self.m21;

        let det = s00 * s11 - s10 * s01;
        if det == 0.0 {
            return;
        }
        let inv_det = 1.0 / det;

        self.m00 = s11 * inv_det;
        self.m10 = -s10 * inv_det;
        self.m20 = (s10 * s21 - s11 * s20) * inv_det;
        self.m01 = -s01 * inv_det;
        self.m11 = s00 * inv_det;
        self.m21 = (s01 * s20 - s00 * s21) * inv_det;
    }
}

