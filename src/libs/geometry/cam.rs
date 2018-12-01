use glium::Display;
use super::Vec2;

#[derive(Copy, Clone)]
pub struct Camera {
    pub zoom: f32,
    pub center: Vec2,
    pub width: u32,
    pub height: u32,
}

impl Default for Camera {
    fn default() -> Camera {
        let (width, height) = (100, 100);
        Camera {
            zoom: 1.0,
            center: Vec2::new(width as f32 / 2.0, height as f32 / 2.0),
            width,
            height,
        }
    }
}

impl Camera {
    pub fn update_win_size(&mut self, size: (u32, u32)) {
        self.width = size.0;
        self.height = size.1;
    }
    pub fn screen_to_world(&self, screen_pos: Vec2) -> Vec2 {
        let screen_size = Vec2::new(self.width as f32, self.height as f32);
        let center = Vec2::new(self.center.x, -self.center.y);

        // Translate by -screen_size/2
        // Scale by 1/zoom
        // Translate by center
        ((screen_pos - screen_size.scale_uni(0.5)).scale_uni(1.0 / self.zoom) + center)
            .scale(1.0, -1.0)
    }
}
