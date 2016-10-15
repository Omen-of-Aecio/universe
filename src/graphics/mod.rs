use tilenet_render::renderer::Renderer as TileNetRenderer;
use glium;
use glium::{ Display };

use world::World;
use world::Tile;


pub struct Graphics<'a> {
    display: &'a Display,
    world: &'a World,
    tilenet_renderer: TileNetRenderer<'a, Tile>,

}

impl<'a> Graphics<'a> {

    pub fn new(display: &'a Display, world: &'a World) -> Graphics<'a>
    {
        Graphics {
            display: display,
            world: world,
            tilenet_renderer: TileNetRenderer::new(display, &world.tiles),
        }
    }

    pub fn render(&mut self, left: f32, top: f32, width: u32, height: u32) {
        self.tilenet_renderer.render(left, top, width, height);
    }
}
