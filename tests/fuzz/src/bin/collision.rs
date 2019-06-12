use geometry::{grid2d::Grid, vec::Vec2};
use honggfuzz::fuzz;
use universe::mediators::does_line_collide_with_grid::collision_test;

fn bytes_to_f32(x: &[u8]) -> f32 {
    let mut data: [u8; 4] = [0; 4];
    data[0] = x[0];
    data[1] = x[1];
    data[2] = x[2];
    data[3] = x[3];
    unsafe { std::mem::transmute::<[u8; 4], f32>(data) }
}

fn main() {
    loop {
        fuzz!(|data: &[u8]| {
            if data.len() < (2 + 2 * 4 + 2 * 4) {
                return;
            }
            let x_size = data[0];
            let y_size = data[1];
            let mut grid: Grid<bool> = Grid::new();
            grid.resize(x_size as usize, y_size as usize);

            let velocity = Vec2::new(bytes_to_f32(&data[2..6]), bytes_to_f32(&data[6..10]));
            let mut vertices = Vec::new();
            vertices.push(Vec2::new(
                bytes_to_f32(&data[10..14]),
                bytes_to_f32(&data[14..18]),
            ));
            vertices.push(Vec2::new(
                bytes_to_f32(&data[18..22]),
                bytes_to_f32(&data[22..26]),
            ));
            vertices.push(Vec2::new(
                bytes_to_f32(&data[26..30]),
                bytes_to_f32(&data[30..34]),
            ));
            vertices.push(Vec2::new(
                bytes_to_f32(&data[34..38]),
                bytes_to_f32(&data[38..42]),
            ));

            collision_test(&vertices, None, velocity, &grid, |x| *x);
        });
    }
}
