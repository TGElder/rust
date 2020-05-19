use crate::M;
use noise::{utils::*, Perlin, Seedable};

pub fn stacked_perlin_noise(
    width: usize,
    height: usize,
    seed: u32,
    frequency_weights: Vec<f64>,
) -> M<f64> {
    let perlin = Perlin::new();
    frequency_weights
        .into_iter()
        .enumerate()
        .filter(|(_, weight)| *weight != 0.0)
        .map(|(i, weight)| {
            perlin_noise(&perlin, 2f64.powf(i as f64), seed + i as u32, width, height) * weight
        })
        .sum()
}

fn perlin_noise(
    perlin: &Perlin,
    frequency: f64,
    seed: u32,
    output_width: usize,
    output_height: usize,
) -> M<f64> {
    let perlin = perlin.set_seed(seed);
    let noise = PlaneMapBuilder::new(&perlin)
        .set_x_bounds(-frequency / 2.0, frequency / 2.0)
        .set_y_bounds(-frequency / 2.0, frequency / 2.0)
        .set_size(output_width, output_height)
        .build();
    M::from_fn(output_width, output_height, |x, y| noise.get_value(x, y))
}
