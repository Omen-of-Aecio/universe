#[derive(Copy,Clone)]
pub enum Color {
    WHITE, BLACK,
}

impl Color {
    pub fn to_rgb(&self) -> [f32; 3] {
        match self {
            &Color::BLACK => [0.0, 0.0, 0.0],
            &Color::WHITE => [1.0, 1.0, 1.0],
        }
    }
}
