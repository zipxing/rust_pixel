#[derive(Clone, Copy)]
pub struct GlColor {
    pub r: f32,
    pub g: f32,
    pub b: f32,
    pub a: f32,
}

impl GlColor {
    pub fn new(r: f32, g: f32, b: f32, a: f32) -> Self {
        Self { r, g, b, a }
    }

    pub fn copy(&self) -> Self {
        *self
    }

    pub fn add(&mut self, color: &GlColor) {
        self.r = (self.r + color.r).min(1.0);
        self.g = (self.g + color.g).min(1.0);
        self.b = (self.b + color.b).min(1.0);
    }

    pub fn multiply(&mut self, color: &GlColor) {
        self.r *= color.r;
        self.g *= color.g;
        self.b *= color.b;
    }

    pub fn equals(&self, color: &GlColor) -> bool {
        self.r == color.r && self.g == color.g && self.b == color.b && self.a == color.a
    }
}

