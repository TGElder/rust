use downhill_map::DownhillMap;
use rand::prelude::*;

pub trait SingleDownhillMap {
    fn get_direction(&self, x: i32, y: i32) -> usize;
}

pub struct MockDownhillMap {
    directions: Vec<Vec<usize>>,
}

impl MockDownhillMap {
    pub fn new(directions: Vec<Vec<usize>>) -> MockDownhillMap {
        MockDownhillMap { directions }
    }
}

impl SingleDownhillMap for MockDownhillMap {
    fn get_direction(&self, x: i32, y: i32) -> usize {
        self.directions[x as usize][y as usize]
    }
}

pub struct RandomDownhillMap {
    width: i32,
    directions: na::DMatrix<u8>,
}

impl RandomDownhillMap {
    pub fn new<R: Rng>(downhill_map: &DownhillMap, rng: &mut Box<R>) -> RandomDownhillMap {
        if !downhill_map.all_cells_have_downhill() {
            panic!("Not all cells have downhill");
        }
        let width = downhill_map.get_width();
        let mut directions = na::DMatrix::zeros(width as usize, width as usize);
        for x in 0..width {
            for y in 0..width {
                let candidates: Vec<u8> = downhill_map
                    .get_directions(x, y)
                    .iter()
                    .enumerate()
                    .filter(|(_, downhill)| **downhill)
                    .map(|(index, _)| index as u8)
                    .collect();

                directions[(x as usize, y as usize)] = *candidates.choose(&mut *rng).unwrap();
            }
        }
        RandomDownhillMap { width, directions }
    }

    pub fn get_width(&self) -> i32 {
        self.width
    }
}

impl SingleDownhillMap for RandomDownhillMap {
    fn get_direction(&self, x: i32, y: i32) -> usize {
        self.directions[(x as usize, y as usize)] as usize
    }
}

#[cfg(test)]
mod tests {

    use super::*;
    use mesh::Mesh;

    #[test]
    fn random_downhill_map_should_contain_downhill_directions() {
        let mut mesh = Mesh::new(4, 0.0);
        let z = na::DMatrix::from_row_slice(
            4,
            4,
            &[
                0.3, 0.8, 0.7, 0.6, 0.4, 0.9, 0.4, 0.5, 0.5, 0.8, 0.3, 0.2, 0.6, 0.7, 0.6, 0.1,
            ],
        );
        mesh.set_z_vector(z);

        let downhill_map = DownhillMap::new(&mesh);

        let mut rng = Box::new(rand::thread_rng());
        let random_downhill_map = RandomDownhillMap::new(&downhill_map, &mut rng);

        for x in 0..random_downhill_map.get_width() {
            for y in 0..random_downhill_map.get_width() {
                let direction = random_downhill_map.get_direction(x, y);
                let downhill = &downhill_map.get_directions(x, y)[direction];
                assert_eq!(*downhill, true);
            }
        }
    }

}
