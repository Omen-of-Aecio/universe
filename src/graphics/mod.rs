use tilenet_ren;
use self::ren::polygons;

use glium::{Display, Surface};

use world::World;
use geometry::vec::Vec2;

pub mod ren;

// Thoughts...
// It makes more sense to keep a reference to World rather than pass as argument
// because it is erroneous to ever use different worlds for rendering, than that
// used in construction. - Erlend

// The problem with this is that we have aliasing of references. World isn't something that
// `Graphics` owns. You won't be able to hold a &mut outside of Graphics. - Kevin


pub struct Graphics {
    display: Display,
    tilenet_renderer: tilenet_ren::Ren,
    poly_renderer: polygons::Ren,
}

impl Graphics {
    pub fn new(display: Display, world: &World) -> Graphics {
        Graphics {
            display: display.clone(),
            tilenet_renderer: tilenet_ren::Ren::new(display.clone(), &world.tilenet),
            poly_renderer: polygons::Ren::new(display.clone(), &world.polygons),
        }
    }


    pub fn render(&mut self,
                  center: (f32, f32),
                  zoom: f32,
                  width: u32,
                  height: u32,
                  world: &World) {
        let mut target = self.display.draw();        // target: glium::Frame
        target.clear_color(0.0, 0.0, 0.0, 1.0);
        self.tilenet_renderer.render(&mut target, center, zoom, width, height);
        self.poly_renderer.render(&mut target, center, zoom, width, height, world);

        target.finish().unwrap();
    }
}

pub fn screen_to_world(screen_pos: Vec2, center: Vec2, zoom: f32, width: u32, height: u32) -> Vec2 {
    let screen_size = Vec2::new(width as f32, height as f32);
    let center = Vec2::new(center.x, -center.y);

    // Translate by -screen_size/2
    // Scale by 1/zoom
    // Translate by center
    (screen_pos - screen_size.scale(0.5)).scale(1.0 / zoom) + center
}
