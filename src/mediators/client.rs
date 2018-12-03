use glocals::Client;
use libs::geometry::grid2d::Grid;
use mediators::{logger::log, random_map_generator};

fn initialize_grid(s: &mut Grid<u8>) {
    s.resize(1000, 1000);
}

pub fn entry_point_client(s: &mut Client) {
    log(&mut s.main.threads, 128, "MAIN", "Creating grid", &[]);
    initialize_grid(&mut s.game.grid);
    random_map_generator::proc1(&mut s.game.grid);
    let size = s.game.grid.get_size();
    for j in 0 .. size.1 {
        for i in 0 .. size.0 {
            print!["{}", if *s.game.grid.get(i, j).unwrap() > 0 { 0 } else { 1 }];
        }
        println![""];
	}
}
