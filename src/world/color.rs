#[derive(Copy, Clone, RustcEncodable, RustcDecodable)]
pub enum Color {
    White, Black,
}

impl Color {
    pub fn to_rgb(&self) -> [f32; 3] {
        match self {
            &Color::Black => [0.0, 0.0, 0.0],
            &Color::White => [1.0, 1.0, 1.0],
        }
    }
    pub fn to_intensity(&self) -> f32 {
     match self {
            &Color::Black => 0.0,
            &Color::White => 1.0,
     }
    }
}
