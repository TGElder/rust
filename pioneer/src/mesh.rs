use scale::Scale;
use std::f64;
use utils::float_ordering;

#[derive(Clone, Debug, PartialEq)]
pub struct Mesh {
    width: i32,
    z: na::DMatrix<f64>,
    out_of_bounds_z: f64,
}

impl Mesh {
    pub fn new(width: i32, out_of_bounds_z: f64) -> Mesh {
        Mesh {
            width,
            z: na::DMatrix::zeros(width as usize, width as usize),
            out_of_bounds_z,
        }
    }

    pub fn get_width(&self) -> i32 {
        self.width
    }

    pub fn get_z_in_bounds(&self, x: i32, y: i32) -> f64 {
        self.z[(x as usize, y as usize)]
    }

    pub fn get_z_vector(&self) -> &na::DMatrix<f64> {
        &self.z
    }

    pub fn in_bounds(&self, x: i32, y: i32) -> bool {
        x >= 0 && y >= 0 && x < self.width && y < self.width
    }

    pub fn get_z(&self, x: i32, y: i32) -> f64 {
        if self.in_bounds(x, y) {
            self.get_z_in_bounds(x, y)
        } else {
            self.out_of_bounds_z
        }
    }

    pub fn set_z(&mut self, x: i32, y: i32, z: f64) {
        self.z[(x as usize, y as usize)] = z;
    }

    pub fn set_z_vector(&mut self, z: na::DMatrix<f64>) {
        self.z = z;
    }

    pub fn get_min_z(&self) -> f64 {
        *self.z.iter().min_by(float_ordering).unwrap()
    }

    pub fn get_max_z(&self) -> f64 {
        *self.z.iter().max_by(float_ordering).unwrap()
    }

    pub fn get_out_of_bounds_z(&self) -> f64 {
        self.out_of_bounds_z
    }

    pub fn rescale(&self, scale: &Scale) -> Mesh {
        let mut out = Mesh::new(self.width, self.out_of_bounds_z);
        for x in 0..self.width {
            for y in 0..self.width {
                out.set_z(x, y, scale.scale(self.get_z(x, y)));
            }
        }
        out
    }
}

#[cfg(test)]
mod tests {

    use super::*;

    #[test]
    fn test_get_min_z() {
        let mut mesh = Mesh::new(3, 0.0);

        let z = na::DMatrix::from_row_slice(3, 3, &[0.8, 0.1, 0.3, 0.9, 0.7, 0.4, 0.2, 0.5, 0.6]);

        mesh.set_z_vector(z);

        assert_eq!(mesh.get_min_z(), 0.1);
    }

    #[test]
    fn test_get_max_z() {
        let mut mesh = Mesh::new(3, 0.0);

        let z = na::DMatrix::from_row_slice(3, 3, &[0.8, 0.1, 0.3, 0.9, 0.7, 0.4, 0.2, 0.5, 0.6]);

        mesh.set_z_vector(z);

        assert_eq!(mesh.get_max_z(), 0.9);
    }

    #[test]
    fn test_rescale() {
        let mut mesh = Mesh::new(2, 0.0);
        let z = na::DMatrix::from_row_slice(2, 2, &[2.0, 4.0, 3.0, 2.0]);
        mesh.set_z_vector(z);

        let scale = Scale::new((2.0, 4.0), (0.0, 128.0));
        let actual = mesh.rescale(&scale);

        let mut expected = Mesh::new(2, 0.0);
        let z = na::DMatrix::from_row_slice(2, 2, &[0.0, 128.0, 64.0, 0.0]);
        expected.set_z_vector(z);

        assert_eq!(actual, expected);
    }

}
