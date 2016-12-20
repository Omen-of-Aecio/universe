use geometry::polygon::Polygon;
use world::World;

enum Stage {
    Player,
    // add for example Polygons(usize) for also iterating over other polygons.
    Done,
}

pub struct PolygonIter<'a> {
    world: &'a World,
    stage: Stage,
}

impl<'a> PolygonIter<'a> {
    pub fn new(world: &'a World) -> PolygonIter<'a> {
        PolygonIter {
            world: world,
            stage: Stage::Player,
        }
    }
}

impl<'a> Iterator for PolygonIter<'a> {
    type Item = &'a Polygon;
    fn next(&mut self) -> Option<&'a Polygon> {
        match self.stage {
            Stage::Player => {
                self.stage = Stage::Done;
                Some(&self.world.player.shape)
            },
            Stage::Done => {
                None
            }
        }
    }
}
