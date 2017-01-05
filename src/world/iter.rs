use geometry::polygon::Polygon;
use world::World;

/// PolygonIter will consequtively go through these 'stages'
enum Stage {
    Players (usize),
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
            stage: Stage::Players (0),
        }
    }
}

impl<'a> Iterator for PolygonIter<'a> {
    type Item = &'a Polygon;
    fn next(&mut self) -> Option<&'a Polygon> {
        match self.stage {
            Stage::Players (index) => {
                if self.world.players.len() == 0 {
                    self.stage = Stage::Done;
                    self.next()
                } else if index == self.world.players.len() - 1 {
                    self.stage = Stage::Done;
                    Some(&self.world.players[index].shape)
                } else {
                    Some(&self.world.players[index].shape)
                }
            },
            Stage::Done => {
                None
            }
        }
    }
}
