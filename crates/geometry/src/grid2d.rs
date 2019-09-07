#[derive(Clone, Default, Debug)]
pub struct Grid<T> {
    grid_data: Vec<T>,
    height: usize,
    width: usize,
}

impl<T> Grid<T>
where
    T: Clone + Default,
{
    pub fn new() -> Grid<T>
    where
        T: Default,
    {
        Grid::default()
    }

    pub fn set(&mut self, x: usize, y: usize, value: T) {
        if x < self.width && y < self.height {
            self.grid_data[x + y * self.width] = value;
        }
    }

    pub fn get(&self, x: usize, y: usize) -> Option<&T> {
        if x < self.width && y < self.height {
            Some(&self.grid_data[x + y * self.width])
        } else {
            None
        }
    }

    pub fn get_mut(&mut self, x: usize, y: usize) -> Option<&mut T> {
        if x < self.width && y < self.height {
            Some(&mut self.grid_data[x + y * self.width])
        } else {
            None
        }
    }

    pub fn resize(&mut self, x: usize, y: usize) {
        self.grid_data.resize(x * y, T::default());
        self.height = y;
        self.width = x;
    }

    pub fn get_size(&self) -> (usize, usize) {
        (self.width, self.height)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new() {
        let grid: Grid<bool> = Grid::new();
        assert![None == grid.get(0, 0)];
    }

    #[test]
    fn resize() {
        let mut grid: Grid<bool> = Grid::new();
        grid.resize(10, 10);
        assert![Some(&false) == grid.get(0, 0)];
    }

    #[test]
    fn resize_zero() {
        let mut grid: Grid<bool> = Grid::new();
        grid.resize(0, 0);
        assert![None == grid.get(0, 0)];
    }

    #[test]
    fn resize_nonsensical() {
        let mut grid: Grid<bool> = Grid::new();
        grid.resize(10, 0);
        assert![None == grid.get(0, 0)];
    }

    #[test]
    fn resize_nonsensical_2() {
        let mut grid: Grid<bool> = Grid::new();
        grid.resize(0, 3);
        assert![None == grid.get(0, 0)];
    }

    #[test]
    fn resize_boundaries() {
        let mut grid: Grid<bool> = Grid::new();
        grid.resize(10, 10);
        assert![Some(&false) == grid.get(9, 9)];
        assert![None == grid.get(10, 9)];
        assert![None == grid.get(9, 10)];
        assert![None == grid.get(10, 10)];
    }

    #[test]
    fn resize_rectangle() {
        let mut grid: Grid<bool> = Grid::new();
        grid.resize(10, 20);
        assert![Some(&false) == grid.get(9, 19)];
        assert![None == grid.get(19, 9)];
    }

    #[test]
    fn get_mut() {
        let mut grid: Grid<bool> = Grid::new();
        grid.resize(10, 20);
        *grid.get_mut(5, 10).unwrap() = true;
        assert![Some(&false) == grid.get(4, 9)];
        assert![Some(&false) == grid.get(5, 9)];
        assert![Some(&false) == grid.get(6, 9)];

        assert![Some(&false) == grid.get(4, 10)];
        assert![Some(&true) == grid.get(5, 10)];
        assert![Some(&false) == grid.get(6, 10)];

        assert![Some(&false) == grid.get(4, 11)];
        assert![Some(&false) == grid.get(5, 11)];
        assert![Some(&false) == grid.get(6, 11)];
    }
}
