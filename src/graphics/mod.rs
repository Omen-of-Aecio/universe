use self::ren::{lines, polygons};
use global::Tile;
use specs;
use tilenet::TileNet;
use tilenet_ren;
use tilenet_ren::MinifySamplerFilter;

use cli::cam::Camera;

use glium::{Display, Surface};

pub mod ren;

// Thoughts...
// It makes more sense to keep a reference to Game rather than pass as argument
// because it is erroneous to ever use different games for rendering, than that
// used in construction. - Erlend

// The problem with this is that we have aliasing of references. Game isn't something that
// `Graphics` owns. You won't be able to hold a &mut outside of Graphics. - Kevin

pub struct Graphics {
    display: Display,
    pub tilenet_renderer: tilenet_ren::Ren,
    poly_renderer: polygons::Ren,
    pub line_renderer: lines::Ren, /* TODO: Maybe make line_renderer private and make interface
                                    * for all renderers in Graphics */
}

impl Graphics {
    pub fn new(display: Display, tilenet: &TileNet<Tile>) -> Graphics {
        let mut g = Graphics {
            display: display.clone(),
            tilenet_renderer: tilenet_ren::Ren::new(display.clone(), tilenet),
            poly_renderer: polygons::Ren::new(display.clone()),
            line_renderer: lines::Ren::new(display.clone()),
        };
        g.tilenet_renderer.set_bg_col(0.1, 0.05, 0.05);
        g.tilenet_renderer
            .set_minify_filter(MinifySamplerFilter::Linear);
        g.tilenet_renderer.set_smooth(false);
        // g.tilenet_renderer.set_magnify_filter(MagnifySamplerFilter::Linear);

        g
    }

    pub fn render(&mut self, cam: Camera, world: &specs::World) {
        let mut target = self.display.draw(); // target: glium::Frame
        target.clear_color(0.0, 0.0, 0.0, 1.0);

        self.tilenet_renderer.render(
            &mut target,
            (cam.center.x, cam.center.y),
            cam.zoom,
            cam.width,
            cam.height,
        );
        self.poly_renderer.render(&mut target, cam, world);

        target.finish().unwrap();
    }
}
