use crate::parameters::Parameters;

pub trait HasParameters {
    fn parameters(&self) -> &Parameters;
}
