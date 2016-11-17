pub mod vec;
pub mod polygon;

use std::cmp::{min, max};
use geometry::vec::Vec2;


/// ...
///
/// Returns alpha where the ray first collides with the box.
pub fn ray_vs_unit_box(point: Vec2, dir: Vec2, box_x: usize, box_y: usize) -> Option<f32> {
    let box_x = box_x as f32;
    let box_y = box_y as f32;

    let dir_x_inv = 1.0/dir.x;
    let dir_y_inv = 1.0/dir.y;

    let tx1 = (box_x - point.x)*dir_x_inv;
    let tx2 = (box_x+1.0 - point.x)*dir_x_inv;

    let mut tmin = tx1.min(tx2);
    let mut tmax = tx1.max(tx2);

    let ty1 = (box_y - point.y)*dir_y_inv;
    let ty2 = (box_y+1.0 - point.y)*dir_y_inv;

    tmin = tmin.max(ty1.min(ty2));
    tmax = tmax.min(ty1.max(ty2));

    if tmax >= tmin {
        Some(tmin)
    } else {
        None
    }
}
