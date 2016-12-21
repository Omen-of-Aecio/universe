use tilenet_ren;
use tilenet_ren::{MinifySamplerFilter, MagnifySamplerFilter};
use self::ren::{polygons, lines};

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
    pub tilenet_renderer: tilenet_ren::Ren,
    poly_renderer: polygons::Ren,
    pub line_renderer: lines::Ren, /* TODO: Maybe make line_renderer private and make interface
                                    * for all renderers in Graphics */
}

impl Graphics {
    pub fn new(display: Display, world: &World) -> Graphics {
        let mut g = Graphics {
            display: display.clone(),
            tilenet_renderer: tilenet_ren::Ren::new(display.clone(), &world.tilenet),
            poly_renderer: polygons::Ren::new(display.clone(), world.polygons_iter()),
            line_renderer: lines::Ren::new(display.clone()),
        };
        g.tilenet_renderer.set_bg_col(0.1, 0.05, 0.05);
        g.tilenet_renderer.set_minify_filter(MinifySamplerFilter::Linear);
        g.tilenet_renderer.set_smooth(false);

        // g.tilenet_renderer.set_magnify_filter(MagnifySamplerFilter::Linear);
        g
    }


    pub fn render(&mut self,
                  center: Vec2,
                  zoom: f32,
                  width: u32,
                  height: u32,
                  world: &World) {
        prof!["Add vectors of world", self.add_vectors_from_world(world)];
        let mut target = self.display.draw();        // target: glium::Frame

        prof![
            "Just render",
            target.clear_color(0.0, 0.0, 0.0, 1.0);
            self.tilenet_renderer.render(&mut target, (center.x, center.y), zoom, width, height);
            self.poly_renderer.render(&mut target, center, zoom, width, height, world);
            self.line_renderer.render(&mut target, center, zoom, width, height, world);
        ];

        prof!["Finish", target.finish().unwrap()];

        prof!["Clear", self.line_renderer.clear()];
    }
    fn add_vectors_from_world(&mut self, world: &World) {
        for &(start, dir) in &world.vectors {
            self.line_renderer.add_vector(start, dir);
        }
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
