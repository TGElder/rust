use commons::*;
use mesh::Mesh;

pub const DIRECTIONS: [(i32, i32); 4] = [(-1, 0), (0, -1), (1, 0), (0, 1)];

#[derive(Debug, PartialEq)]
pub struct DownhillMap {
    width: i32,
    directions: M<[bool; 4]>,
}

impl DownhillMap {
    pub fn new(mesh: &Mesh) -> DownhillMap {
        let mut out = DownhillMap {
            width: mesh.get_width(),
            directions: M::repeat(
                mesh.get_width() as usize,
                mesh.get_width() as usize,
                [false; 4],
            ),
        };
        out.compute_all_directions(mesh);
        out
    }

    pub fn get_width(&self) -> i32 {
        self.width
    }

    pub fn get_directions(&self, x: i32, y: i32) -> [bool; 4] {
        self.directions[(x as usize, y as usize)]
    }

    fn set_directions(&mut self, x: i32, y: i32, directions: [bool; 4]) {
        self.directions[(x as usize, y as usize)] = directions;
    }

    fn compute_directions(mesh: &Mesh, x: i32, y: i32) -> [bool; 4] {
        let z = mesh.get_z(x, y);
        let mut out = [false; 4];
        for d in 0..DIRECTIONS.len() {
            let dx = DIRECTIONS[d].0;
            let dy = DIRECTIONS[d].1;
            out[d] = mesh.get_z(x + dx, y + dy) < z;
        }
        out
    }

    fn compute_all_directions(&mut self, mesh: &Mesh) {
        for x in 0..mesh.get_width() {
            for y in 0..mesh.get_width() {
                let directions = DownhillMap::compute_directions(mesh, x, y);
                self.set_directions(x, y, directions);
            }
        }
    }

    fn cell_has_downhill(&self, x: i32, y: i32) -> bool {
        for downhill in self.get_directions(x, y).iter() {
            if *downhill {
                return true;
            }
        }
        false
    }

    pub fn all_cells_have_downhill(&self) -> bool {
        for x in 0..self.width {
            for y in 0..self.width {
                if !self.cell_has_downhill(x, y) {
                    return false;
                }
            }
        }
        true
    }
}

#[cfg(test)]
mod tests {

    use super::*;

    #[test]
    fn test_compute_directions() {
        let mut mesh = Mesh::new(3, 0.0);
        mesh.set_z_vector(M::from_row_slice(
            3,
            3,
            &[0.1, 0.8, 0.2, 0.3, 0.5, 0.9, 0.6, 0.4, 0.7],
        ));

        let expected = [false, true, true, false];
        let actual = DownhillMap::compute_directions(&mesh, 1, 1);

        assert_eq!(actual, expected);
    }

    #[test]
    fn test_compute_all_directions() {
        let mut mesh = Mesh::new(2, 0.0);
        mesh.set_z_vector(M::from_row_slice(2, 2, &[0.1, 0.2, 0.3, 0.4]));

        let expected = DownhillMap {
            width: 2,
            directions: M::from_row_slice(
                2,
                2,
                &[
                    [true, true, false, false],
                    [true, true, false, true],
                    [true, true, true, false],
                    [true, true, true, true],
                ],
            ),
        };

        let actual = DownhillMap::new(&mesh);

        assert_eq!(actual, expected);
    }

    #[test]
    fn test_all_cells_have_downhill() {
        let mut mesh = Mesh::new(3, 0.0);
        mesh.set_z_vector(M::from_row_slice(
            3,
            3,
            &[0.1, 0.8, 0.2, 0.3, 0.5, 0.9, 0.6, 0.4, 0.7],
        ));
        let downhill = DownhillMap::new(&mesh);

        assert!(downhill.all_cells_have_downhill());
    }

    #[test]
    fn test_not_all_cells_have_downhill() {
        let mut mesh = Mesh::new(3, 0.0);
        mesh.set_z_vector(M::from_row_slice(
            3,
            3,
            &[0.5, 0.8, 0.2, 0.3, 0.1, 0.9, 0.6, 0.4, 0.7],
        ));
        let downhill = DownhillMap::new(&mesh);

        assert!(!downhill.all_cells_have_downhill());
    }
}
