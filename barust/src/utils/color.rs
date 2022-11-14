use cairo::Context;

#[derive(Debug, Default, Clone, Copy)]
pub struct Color {
    pub r: f64,
    pub g: f64,
    pub b: f64,
    pub a: f64,
}

impl Color {
    pub const fn new(r: f64, g: f64, b: f64, a: f64) -> Self {
        Self { r, g, b, a }
    }
}

pub fn set_source_rgba(context: &Context, color: Color) {
    context.set_source_rgba(color.r, color.g, color.b, color.a);
}

