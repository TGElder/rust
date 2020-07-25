use super::*;

pub trait Builder {
    fn can_build(&self, build: &Build) -> bool;
    fn build(&mut self, build: Build);
}
