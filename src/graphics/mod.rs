// use tilenet_render::renderer::Renderer as TileNetRenderer;
use tilenet_ren;
use self::ren::polygons;

use glium::{ Display, Surface };

use world::World;

pub mod ren;

// Thoughts...
// It makes more sense to keep a reference to World rather than pass as argument
// because it is erroneous to ever use different worlds for rendering, than that
// used in construction.


pub struct Graphics {
    display: Display,
    tilenet_renderer: tilenet_ren::Ren,
    poly_renderer: polygons::Ren,
}

impl Graphics {

    pub fn new(display: Display, world: &World) -> Graphics
    {
        Graphics {
            display: display.clone(),
            tilenet_renderer: tilenet_ren::Ren::new(display.clone(), &world.tilenet),
            poly_renderer: polygons::Ren::new(display.clone(), &world.polygons),
        }
    }


    pub fn render(&mut self, center_x: f32, center_y: f32, zoom: f32, width: u32, height: u32, world: &World) {
        let mut target = self.display.draw();        // target: glium::Frame
        target.clear_color(0.0, 0.0, 0.0, 1.0);
        self.tilenet_renderer.render(&mut target, center_x, center_y, zoom, width, height);
        self.poly_renderer.render(&mut target, center_x, center_y, zoom, width, height, world);

        target.finish().unwrap();
    }
}
