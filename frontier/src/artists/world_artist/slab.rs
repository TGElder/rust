use commons::{v2, V2};

#[derive(Clone, Copy, Hash, PartialEq, Eq, Debug)]
pub struct Slab {
    pub from: V2<usize>,
    pub slab_size: usize,
}

impl Slab {
    pub fn at(point: V2<usize>, slab_size: usize) -> Slab {
        let from = (point / slab_size) * slab_size;
        Slab { from, slab_size }
    }

    pub fn inside<'a>(
        width: &'a usize,
        height: &'a usize,
        slab_size: &'a usize,
    ) -> impl Iterator<Item = Slab> + 'a {
        (0..width / slab_size).flat_map(move |x| {
            (0..height / slab_size)
                .map(move |y| Slab::at(v2(x * slab_size, y * slab_size), *slab_size))
        })
    }

    pub fn to(&self) -> V2<usize> {
        v2(self.from.x + self.slab_size, self.from.y + self.slab_size)
    }
}

#[cfg(test)]
mod tests {

    use super::*;

    #[test]
    fn at() {
        assert_eq!(
            Slab::at(v2(11, 33), 32),
            Slab {
                from: v2(0, 32),
                slab_size: 32,
            }
        );
    }

    #[test]
    fn inside() {
        let actual: Vec<Slab> = Slab::inside(&100, &50, &25).collect();
        assert_eq!(
            actual,
            vec![
                Slab {
                    from: v2(0, 0),
                    slab_size: 25
                },
                Slab {
                    from: v2(0, 25),
                    slab_size: 25
                },
                Slab {
                    from: v2(25, 0),
                    slab_size: 25
                },
                Slab {
                    from: v2(25, 25),
                    slab_size: 25
                },
                Slab {
                    from: v2(50, 0),
                    slab_size: 25
                },
                Slab {
                    from: v2(50, 25),
                    slab_size: 25
                },
                Slab {
                    from: v2(75, 0),
                    slab_size: 25
                },
                Slab {
                    from: v2(75, 25),
                    slab_size: 25
                },
            ]
        );
    }

    #[test]
    fn to() {
        assert_eq!(Slab::at(v2(11, 33), 32).to(), v2(32, 64));
    }
}
