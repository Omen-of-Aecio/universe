use geometry::polygon::Polygon;
use world::World;

/// PolygonIter will consequtively go through these 'stages'
#[derive(Copy, Clone)]
enum Stage {
    Players (i32),
    // add for example Polygons(usize) for also iterating over other polygons.
}

pub struct PolygonIter<'a> {
    world: &'a World,
    stage: Stage,
}

impl<'a> PolygonIter<'a> {
    pub fn new(world: &'a World) -> PolygonIter<'a> {
        PolygonIter {
            world: world,
            stage: Stage::Players (-1),
        }
    }
}

impl<'a> Iterator for PolygonIter<'a> {
    type Item = &'a Polygon;
    fn next(&mut self) -> Option<&'a Polygon> {
        match self.stage {
            Stage::Players (ref mut index) => {
                *index += 1;
                if *index as usize >= self.world.players.len() {
                    None
                } else {
                    Some(&self.world.players[*index as usize].shape)
                }
            },
        }
    }
}
