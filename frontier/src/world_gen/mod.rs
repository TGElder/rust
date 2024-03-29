mod dredging;
mod groundwater_gen;
mod rainfall_gen;
mod river_water;
mod sea_border;
mod temperature;
mod validation;
mod vegetation_gen;

use crate::world::World;
use crate::world_gen::dredging::dredge;
use commons::equalize::{equalize_with_filter, PositionValue};
use commons::grid::Grid;
use commons::scale::Scale;
use commons::*;
use groundwater_gen::*;
use num::Float;
use pioneer::erosion::Erosion;
use pioneer::mesh::Mesh;
use pioneer::mesh_splitter::MeshSplitter;
use pioneer::river_runner::*;
use rainfall_gen::*;
use rand::prelude::*;
use river_water::*;
use sea_border::with_sea_border;
use serde::{Deserialize, Serialize};
use std::default::Default;
use std::f64::MAX;
use std::fmt::Debug;
use temperature::*;
use validation::*;
use vegetation_gen::*;

#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub struct WorldGenParameters {
    pub river_width_range: (f64, f64),
    pub latitude_range: (f64, f64),
    pub cliff_gradient: f32,
    pub split_range: (f64, f64),
    pub max_height: f64,
    pub sea_level: f64,
    pub beach_level: f32,
    pub erosion_iterations: usize,
    pub erosion_amount: f64,
    pub river_threshold: f64,
    pub river_water: RiverWaterParams,
    pub rainfall: RainfallGenParams,
    pub temperature: TemperatureParams,
    pub vegetation: VegetationParams,
    pub validation: WorldValidationParams,
}

impl Default for WorldGenParameters {
    fn default() -> WorldGenParameters {
        WorldGenParameters {
            river_width_range: (0.01, 0.5),
            latitude_range: (0.0, 50.0),
            cliff_gradient: 0.5,
            split_range: (0.0, 0.9),
            max_height: 16.0,
            sea_level: 1.0,
            beach_level: 1.05,
            erosion_iterations: 16,
            erosion_amount: 0.9,
            river_threshold: 16.0,
            river_water: RiverWaterParams::default(),
            rainfall: RainfallGenParams::default(),
            temperature: TemperatureParams::default(),
            vegetation: VegetationParams::default(),
            validation: WorldValidationParams::default(),
        }
    }
}

pub fn generate_world<T: Rng>(power: usize, rng: &mut T, params: &WorldGenParameters) -> World {
    loop {
        let candidate = try_generate_world(power, rng, params);
        if world_is_valid(&params.validation, &candidate) {
            return candidate;
        }
    }
}

fn try_generate_world<T: Rng>(power: usize, rng: &mut T, params: &WorldGenParameters) -> World {
    let mut mesh = Mesh::new(1, 0.0);
    mesh.set_z(0, 0, MAX);

    println!("Generating world...");
    for i in 0..power {
        mesh = MeshSplitter::split(&mesh, rng, params.split_range);
        let threshold = i * 2;
        mesh = Erosion::erode(
            mesh,
            rng,
            threshold as f64,
            params.erosion_iterations,
            params.erosion_amount,
        );
        println!("{}", power - i);
    }

    let rescaled = mesh.rescale(&Scale::new(
        (mesh.get_min_z(), mesh.get_max_z()),
        (0.0, params.max_height),
    ));
    let terrain = rescaled.get_z_vector().map(|z| z as f32);
    let terrain = with_sea_border(terrain, params.sea_level as f32);
    let mut out = World::new(terrain, params.sea_level as f32);

    dredge(&mut out);

    let temperatures = compute_temperatures(&out, params);
    load_temperatures(&mut out, &temperatures);
    let rainfall = gen_rainfall(&out, params);
    load_rainfall(&mut out, &rainfall);

    let river_end_level = Scale::new(
        (0.0, params.max_height),
        (mesh.get_min_z(), mesh.get_max_z()),
    )
    .scale(params.sea_level - 0.01);
    let river_cells = get_river_cells(
        &mesh,
        params.river_threshold,
        river_end_level,
        params.river_width_range,
        &rainfall,
        rng,
    );
    for cell in river_cells {
        out.add_river(cell);
    }

    let river_water = compute_river_water(&out, params);
    let river_water = river_water.map(|v| v.sqrt());
    load_river_water(&mut out, &river_water);

    let groundwater = compute_groundwater(&out);
    load_groundwater(&mut out, &groundwater);

    let mut vegetation_gen = VegetationGen::new(power, &mut out, params, rng);
    let vegetation = vegetation_gen.compute_vegetation();
    vegetation_gen.load_vegetation(&vegetation);

    out
}

fn rescale_ignoring_sea<T>(output: M<T>, world: &World) -> M<T>
where
    T: 'static + Debug + Float,
{
    let (min, max) = min_max_ignoring_sea(&output, world);
    let scale = Scale::new((min, max), (T::zero(), T::one()));
    output.map(|v| scale.scale(v))
}

fn equalize_ignoring_sea<T>(output: M<T>, world: &World) -> M<T>
where
    T: 'static + Debug + Float,
{
    equalize_with_filter(output, &|PositionValue { position, .. }| {
        !world.is_sea(position)
    })
}

fn min_max_ignoring_sea<T>(output: &M<T>, world: &World) -> (T, T)
where
    T: 'static + Debug + Float,
{
    let mut min: Option<T> = None;
    let mut max: Option<T> = None;
    for x in 0..world.width() {
        for y in 0..world.height() {
            if !world.is_sea(&v2(x, y)) {
                let value = output[(x, y)];
                min = Some(min.map_or(value, |min| min.min(value)));
                max = Some(max.map_or(value, |max| max.max(value)));
            }
        }
    }
    (min.unwrap_or_else(T::zero), max.unwrap_or_else(T::one))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[rustfmt::skip]
    fn test_min_max_ignoring_sea() {
        let output = M::from_vec(3, 3, vec![
            9.0, 8.0, 7.0,
            6.0, 5.0, 4.0,
            3.0, 2.0, 1.0,
        ]);
        let world = World::new(
            M::from_vec(3, 3, vec![
                0.0, 1.0, 1.0,
                1.0, 2.0, 1.0,
                1.0, 0.0, 0.0,
            ]),
            0.5,
        );
        assert_eq!(min_max_ignoring_sea(&output, &world), (3.0, 8.0));
    }
}
