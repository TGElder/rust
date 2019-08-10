use commons::*;
use downhill_map::DIRECTIONS;
use mesh::Mesh;
use single_downhill_map::SingleDownhillMap;

#[derive(Debug, PartialEq)]
pub struct FlowMap {
    flow: M<f64>,
}

impl FlowMap {
    pub fn new(width: usize) -> FlowMap {
        FlowMap {
            flow: M::zeros(width, width),
        }
    }

    pub fn get_flow(&self, x: i32, y: i32) -> f64 {
        self.flow[(x as usize, y as usize)]
    }

    pub fn get_flow_matrix(&self) -> &M<f64> {
        &self.flow
    }

    pub fn get_max_flow(&self) -> f64 {
        *self.flow.iter().max_by(unsafe_ordering).unwrap()
    }

    pub fn set_flow(&mut self, flow: M<f64>) {
        self.flow = flow;
    }

    pub fn from(mesh: &Mesh, downhill_map: &SingleDownhillMap, rainfall: &M<f64>) -> FlowMap {
        let mut out = FlowMap::new(mesh.get_width() as usize);
        out.rain_on_all(mesh, downhill_map, rainfall);
        out
    }

    fn rain_on(
        &mut self,
        mesh: &Mesh,
        downhill_map: &SingleDownhillMap,
        x: i32,
        y: i32,
        volume: f64,
    ) {
        let mut focus = (x, y);
        while mesh.in_bounds(focus.0, focus.1) {
            self.flow[(focus.0 as usize, focus.1 as usize)] += volume;
            let direction = DIRECTIONS[downhill_map.get_direction(focus.0, focus.1)];
            focus = (focus.0 + direction.0, focus.1 + direction.1);
        }
    }

    fn rain_on_all(&mut self, mesh: &Mesh, downhill_map: &SingleDownhillMap, rainfall: &M<f64>) {
        for x in 0..mesh.get_width() {
            for y in 0..mesh.get_width() {
                self.rain_on(mesh, downhill_map, x, y, rainfall[(x as usize, y as usize)]);
            }
        }
    }
}

#[cfg(test)]
mod tests {

    use super::*;

    use single_downhill_map::MockDownhillMap;

    #[rustfmt::skip]
    #[test]
    pub fn test_rain_on() {
        let mesh = Mesh::new(4, 0.0);

        let directions = vec![
            vec![3, 3, 3, 3],
            vec![3, 3, 3, 3],
            vec![0, 0, 0, 0],
            vec![0, 0, 0, 0],
        ];
        let downhill_map = MockDownhillMap::new(directions);

        let mut flow_map = FlowMap::new(4);
        flow_map.rain_on(&mesh, &downhill_map, 2, 1, 1.0);

        let expected =
            M::from_row_slice(4, 4, &[
                0.0, 0.0, 0.0, 0.0,
                0.0, 1.0, 1.0, 1.0,
                0.0, 1.0, 0.0, 0.0,
                0.0, 0.0, 0.0, 0.0
            ]);
        let expected = FlowMap { flow: expected };

        assert_eq!(flow_map, expected);
    }

    #[rustfmt::skip]
    #[test]
    pub fn test_from() {
        let mesh = Mesh::new(4, 0.0);

        let directions = vec![
            vec![3, 3, 3, 3],
            vec![3, 3, 3, 3],
            vec![0, 0, 0, 0],
            vec![0, 0, 0, 0],
        ];
        let downhill_map = MockDownhillMap::new(directions);

        let flow_map = FlowMap::from(&mesh, &downhill_map, &M::from_element(4, 4, 1.0));

        let expected =
            M::from_row_slice(4, 4, &[
                1.0, 2.0, 3.0, 4.0,
                3.0, 6.0, 9.0, 12.0,
                2.0, 2.0, 2.0, 2.0,
                1.0, 1.0, 1.0, 1.0
            ]);
        let expected = FlowMap { flow: expected };

        assert_eq!(flow_map, expected);
    }

    #[rustfmt::skip]
    #[test]
    pub fn test_max_flow() {
        let flow = M::from_row_slice(
            4,
            4,
            &[
                1.0, 9.0, 4.0, 10.0,
                14.0, 12.0, 5.0, 11.0,
                7.0, 2.0, 16.0, 8.0,
                13.0, 3.0, 15.0, 6.0
            ],
        );
        let flow_map = FlowMap { flow };
        assert!(flow_map.get_max_flow().almost(16.0));
    }

}
