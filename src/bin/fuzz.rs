use honggfuzz::fuzz;
use universe::{
    libs::geometry::{grid2d::Grid, vec::Vec2},
    mediators::does_line_collide_with_grid::does_line_collide_with_grid,
};

fn bytes_to_f32(x: [u8; 4]) -> f32 {
    unsafe { std::mem::transmute::<[u8; 4], f32>(x) }
}
fn main() {
    // Here you can parse `std::env::args and
    // setup / initialize your project

    // You have full control over the loop but
    // you're supposed to call `fuzz` ad vitam aeternam
    loop {
        // The fuzz macro gives an arbitrary object (see `arbitrary crate`)
        // to a closure-like block of code.
        // For performance reasons, it is recommended that you use the native type
        // `&[u8]` when possible.
        // Here, this slice will contain a "random" quantity of "random" data.
        fuzz!(|data: &[u8]| {
            if data.len() < (2 + 2 * 4 + 2 * 4) {
                return;
            }
            let x_size = data[0];
            let y_size = data[1];
            let mut grid: Grid<bool> = Grid::new();
            // if x_size > 0 {
            //     panic!["BOOM2"];
            // }
            grid.resize(x_size as usize, y_size as usize);

            let f1d: [u8; 4] = [data[2], data[3], data[4], data[5]];
            let f1 = bytes_to_f32(f1d);
            let f2d: [u8; 4] = [data[6], data[7], data[8], data[9]];
            let f2 = bytes_to_f32(f2d);
            let f3d: [u8; 4] = [data[10], data[11], data[12], data[13]];
            let f3 = bytes_to_f32(f3d);
            let f4d: [u8; 4] = [data[14], data[15], data[16], data[17]];
            let f4 = bytes_to_f32(f4d);

            let from = Vec2 { x: f1, y: f2 };
            let to = Vec2 { x: f3, y: f4 };

            does_line_collide_with_grid(&grid, from, to, |x| *x);
        });
    }
}
