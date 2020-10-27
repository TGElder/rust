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

    pub fn to(&self) -> V2<usize> {
        v2(self.from.x + self.slab_size, self.from.y + self.slab_size)
    }
}

#[cfg(test)]
mod tests {

    use super::*;

    #[test]
    fn slab_at() {
        assert_eq!(
            Slab::at(v2(11, 33), 32),
            Slab {
                from: v2(0, 32),
                slab_size: 32,
            }
        );
    }

    #[test]
    fn slab_to() {
        assert_eq!(Slab::at(v2(11, 33), 32).to(), v2(32, 64));
    }
}
