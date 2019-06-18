use commons::unsafe_ordering;
use mesh::Mesh;

use rand::prelude::*;
use scale::Scale;

#[derive(Debug, PartialEq)]
struct Split {
    x: i32,
    y: i32,
    z: f64,
}

#[derive(Debug, PartialEq)]
struct SplitRule {
    x: i32,
    y: i32,
    range: (f64, f64),
}

#[derive(Debug, PartialEq)]
struct SplitProcess {
    split_rules: Vec<SplitRule>,
    splits: Vec<Split>,
}

impl SplitRule {
    fn generate_split<R: Rng>(&self, rng: &mut R, random_range: (f64, f64)) -> Split {
        let r: f64 = rng.gen_range(random_range.0, random_range.1);
        let scale: Scale<f64> = Scale::new((0.0, 1.0), self.range);
        Split {
            x: self.x,
            y: self.y,
            z: scale.scale(r),
        }
    }
}

impl SplitProcess {
    fn new(mesh: &Mesh, x: i32, y: i32) -> SplitProcess {
        const OFFSETS: [(i32, i32); 4] = [(0, 0), (0, 1), (1, 0), (1, 1)];

        let mut split_rules: Vec<SplitRule> = OFFSETS
            .iter()
            .map(|o| {
                let dx: i32 = (o.0 as i32 * 2) - 1;
                let dy: i32 = (o.1 as i32 * 2) - 1;
                let z = mesh.get_z(x, y);
                let zs = [mesh.get_z(x + dx, y), mesh.get_z(x, y + dy), z];
                let min_z = zs.iter().min_by(unsafe_ordering).unwrap();

                SplitRule {
                    x: x * 2 + o.0,
                    y: y * 2 + o.1,
                    range: (*min_z, z),
                }
            })
            .collect();

        split_rules.sort_by(|a, b| a.range.0.partial_cmp(&b.range.0).unwrap());

        SplitProcess {
            split_rules,
            splits: Vec::with_capacity(4),
        }
    }

    fn next<R: Rng>(mut self, rng: &mut R, random_range: (f64, f64)) -> SplitProcess {
        fn update_rule(rule: SplitRule, split: &Split) -> SplitRule {
            if rule.x == split.x || rule.y == split.y {
                SplitRule {
                    x: rule.x,
                    y: rule.y,
                    range: (split.z.min(rule.range.0), rule.range.1),
                }
            } else {
                rule
            }
        }

        let split = self.split_rules[0].generate_split(rng, random_range);
        self.split_rules.remove(0);
        self.split_rules = self
            .split_rules
            .into_iter()
            .map(|rule| update_rule(rule, &split))
            .collect();
        self.splits.push(split);
        self
    }

    fn complete<R: Rng>(mut self, rng: &mut R, random_range: (f64, f64)) -> Vec<Split> {
        while !self.split_rules.is_empty() {
            self = self.next(rng, random_range);
        }
        self.splits
    }
}

pub struct MeshSplitter {}

impl MeshSplitter {
    fn get_all_splits<R: Rng>(mesh: &Mesh, rng: &mut R, random_range: (f64, f64)) -> Vec<Split> {
        let mut out = Vec::with_capacity((mesh.get_width() * mesh.get_width() * 4) as usize);
        for x in 0..mesh.get_width() {
            for y in 0..mesh.get_width() {
                out.append(&mut SplitProcess::new(mesh, x, y).complete(rng, random_range));
            }
        }
        out
    }

    pub fn split<R: Rng>(mesh: &Mesh, rng: &mut R, random_range: (f64, f64)) -> Mesh {
        let mut out = Mesh::new(mesh.get_width() * 2, mesh.get_out_of_bounds_z());
        for split in MeshSplitter::get_all_splits(mesh, rng, random_range) {
            out.set_z(split.x, split.y, split.z);
        }
        out
    }

    pub fn split_n_times<R: Rng>(
        mesh: &Mesh,
        rng: &mut R,
        random_range: (f64, f64),
        times: u32,
    ) -> Mesh {
        let mut out = MeshSplitter::split(mesh, rng, random_range);
        for _ in 1..times {
            out = MeshSplitter::split(&out, rng, random_range);
        }
        out
    }
}

#[cfg(test)]
mod tests {

    use super::*;
    use rand::rngs::mock::StepRng;
    use std::u64;

    fn get_rng() -> StepRng {
        StepRng::new(u64::MAX / 2 + 1, 0)
    }

    #[test]
    fn test_split_rule_generate_split() {
        let rule = SplitRule {
            x: 11,
            y: 12,
            range: (0.12, 0.1986),
        };
        let expected = Split {
            x: 11,
            y: 12,
            z: 0.15537,
        };
        assert_eq!(rule.generate_split(&mut get_rng(), (0.1, 0.8)), expected);
    }

    #[test]
    fn test_split_process_new() {
        let mut mesh = Mesh::new(3, 0.0);

        let z = na::DMatrix::from_row_slice(3, 3, &[0.8, 0.3, 0.2, 0.9, 0.7, 0.4, 0.1, 0.5, 0.6]);

        mesh.set_z_vector(z);

        let expected_1 = SplitProcess {
            split_rules: vec![
                SplitRule {
                    x: 2,
                    y: 2,
                    range: (0.3, 0.7),
                },
                SplitRule {
                    x: 2,
                    y: 3,
                    range: (0.3, 0.7),
                },
                SplitRule {
                    x: 3,
                    y: 3,
                    range: (0.4, 0.7),
                },
                SplitRule {
                    x: 3,
                    y: 2,
                    range: (0.5, 0.7),
                },
            ],
            splits: vec![],
        };

        let expected_2 = SplitProcess {
            split_rules: vec![
                SplitRule {
                    x: 2,
                    y: 3,
                    range: (0.3, 0.7),
                },
                SplitRule {
                    x: 2,
                    y: 2,
                    range: (0.3, 0.7),
                },
                SplitRule {
                    x: 3,
                    y: 3,
                    range: (0.4, 0.7),
                },
                SplitRule {
                    x: 3,
                    y: 2,
                    range: (0.5, 0.7),
                },
            ],
            splits: vec![],
        };

        let actual = SplitProcess::new(&mesh, 1, 1);

        assert_eq!(actual == expected_1 || actual == expected_2, true);
    }

    #[test]
    fn test_split_process_next() {
        let random_range = (0.0, 1.0);

        let process = SplitProcess {
            split_rules: vec![
                SplitRule {
                    x: 0,
                    y: 0,
                    range: (0.1, 0.7),
                },
                SplitRule {
                    x: 1,
                    y: 0,
                    range: (0.2, 0.7),
                },
                SplitRule {
                    x: 0,
                    y: 1,
                    range: (0.5, 0.7),
                },
                SplitRule {
                    x: 1,
                    y: 1,
                    range: (0.5, 0.7),
                },
            ],
            splits: vec![],
        };

        let actual = process.next(&mut get_rng(), random_range);

        let expected = SplitProcess {
            split_rules: vec![
                SplitRule {
                    x: 1,
                    y: 0,
                    range: (0.2, 0.7),
                },
                SplitRule {
                    x: 0,
                    y: 1,
                    range: (0.4, 0.7),
                },
                SplitRule {
                    x: 1,
                    y: 1,
                    range: (0.5, 0.7),
                },
            ],
            splits: vec![Split { x: 0, y: 0, z: 0.4 }],
        };

        assert_eq!(actual, expected);
    }

    #[test]
    fn test_split_process_complete() {
        let mut mesh = Mesh::new(3, 0.0);

        let z = na::DMatrix::from_row_slice(3, 3, &[0.8, 0.3, 0.2, 0.9, 0.7, 0.4, 0.1, 0.5, 0.6]);

        mesh.set_z_vector(z);

        let mut rng = get_rng();
        let random_range = (0.1, 0.5);

        let expected = SplitProcess::new(&mesh, 1, 1)
            .next(&mut rng, random_range)
            .next(&mut rng, random_range)
            .next(&mut rng, random_range)
            .next(&mut rng, random_range)
            .splits;

        assert_eq!(
            SplitProcess::new(&mesh, 1, 1).complete(&mut rng, random_range),
            expected
        );
    }

    #[test]
    fn test_mesh_splitter_split() {
        let mut mesh = Mesh::new(2, 0.0);

        let z = na::DMatrix::from_row_slice(2, 2, &[0.1, 0.2, 0.3, 0.4]);

        mesh.set_z_vector(z);

        let mut rng = get_rng();
        let random_range = (0.1, 0.5);

        let next = MeshSplitter::split(&mesh, &mut rng, random_range);

        fn check_splits(mesh: &Mesh, splits: Vec<Split>) {
            for split in splits {
                assert_eq!(mesh.get_z(split.x, split.y), split.z);
            }
        }

        check_splits(
            &next,
            SplitProcess::new(&mesh, 0, 0).complete(&mut rng, random_range),
        );
        check_splits(
            &next,
            SplitProcess::new(&mesh, 0, 1).complete(&mut rng, random_range),
        );
        check_splits(
            &next,
            SplitProcess::new(&mesh, 1, 0).complete(&mut rng, random_range),
        );
        check_splits(
            &next,
            SplitProcess::new(&mesh, 1, 1).complete(&mut rng, random_range),
        );
    }

    #[test]
    fn mesh_splitter_should_retain_downhill_property() {
        use downhill_map::DownhillMap;

        let mut mesh = Mesh::new(1, 0.0);
        mesh.set_z(0, 0, 1.0);
        let mut rng = get_rng();
        let random_range = (0.1, 0.5);

        mesh = MeshSplitter::split_n_times(&mesh, &mut rng, random_range, 10);
        assert_eq!(mesh.get_width(), 1024);
        let downhill = DownhillMap::new(&mesh);
        assert_eq!(downhill.all_cells_have_downhill(), true);
    }

}
