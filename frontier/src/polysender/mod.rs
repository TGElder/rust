#[allow(clippy::module_inception)]
mod polysender;

pub use polysender::Polysender;

use commons::async_trait::async_trait;
