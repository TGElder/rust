use downhill_map::DIRECTIONS;
use mesh::Mesh;
use single_downhill_map::SingleDownhillMap;

#[derive(Debug, PartialEq)]
pub struct FlowMap {
    flow: na::DMatrix<u32>,
}

impl FlowMap {
    pub fn new(width: usize) -> FlowMap {
        FlowMap {
            flow: na::DMatrix::zeros(width, width),
        }
    }

    pub fn get_flow(&self, x: i32, y: i32) -> u32 {
        self.flow[(x as usize, y as usize)]
    }

    pub fn get_flow_matrix(&self) -> &na::DMatrix<u32> {
        &self.flow
    }

    pub fn get_max_flow(&self) -> u32 {
        *self.flow.iter().max().unwrap()
    }

    pub fn set_flow(&mut self, flow: na::DMatrix<u32>) {
        self.flow = flow;
    }

    pub fn from(mesh: &Mesh, downhill_map: &Box<SingleDownhillMap>) -> FlowMap {
        let mut out = FlowMap::new(mesh.get_width() as usize);
        out.rain_on_all(mesh, downhill_map);
        out
    }

    fn rain_on(&mut self, mesh: &Mesh, downhill_map: &Box<SingleDownhillMap>, x: i32, y: i32) {
        let mut focus = (x, y);
        while mesh.in_bounds(focus.0, focus.1) {
            self.flow[(focus.0 as usize, focus.1 as usize)] += 1;
            let direction = DIRECTIONS[downhill_map.get_direction(focus.0, focus.1)];
            focus = (focus.0 + direction.0, focus.1 + direction.1);
        }
    }

    fn rain_on_all(&mut self, mesh: &Mesh, downhill_map: &Box<SingleDownhillMap>) {
        for x in 0..mesh.get_width() {
            for y in 0..mesh.get_width() {
                self.rain_on(mesh, downhill_map, x, y);
            }
        }
    }
}

#[cfg(test)]
mod tests {

    use super::*;
    use single_downhill_map::MockDownhillMap;

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
        let downhill_map: Box<SingleDownhillMap> = Box::new(downhill_map);

        let mut flow_map = FlowMap::new(4);
        flow_map.rain_on(&mesh, &downhill_map, 2, 1);

        let expected =
            na::DMatrix::from_row_slice(4, 4, &[0, 0, 0, 0, 0, 1, 1, 1, 0, 1, 0, 0, 0, 0, 0, 0]);
        let expected = FlowMap { flow: expected };

        assert_eq!(flow_map, expected);
    }

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
        let downhill_map: Box<SingleDownhillMap> = Box::new(downhill_map);

        let flow_map = FlowMap::from(&mesh, &downhill_map);

        let expected =
            na::DMatrix::from_row_slice(4, 4, &[1, 2, 3, 4, 3, 6, 9, 12, 2, 2, 2, 2, 1, 1, 1, 1]);
        let expected = FlowMap { flow: expected };

        assert_eq!(flow_map, expected);
    }

    #[test]
    pub fn test_max_flow() {
        let flow = na::DMatrix::from_row_slice(
            4,
            4,
            &[1, 9, 4, 10, 14, 12, 5, 11, 7, 2, 16, 8, 13, 3, 15, 6],
        );
        let flow_map = FlowMap { flow };
        assert_eq!(flow_map.get_max_flow(), 16);
    }

}
