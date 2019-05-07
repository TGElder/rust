use super::super::engine::DrawingType;
use super::super::vertex_objects::VBO;
use super::utils::*;
use super::Drawing;
use color::Color;
use coords::*;

pub struct HouseDrawing {
    vbo: VBO,
}

impl Drawing for HouseDrawing {
    fn draw(&self) {
        self.vbo.draw();
    }

    fn get_z_mod(&self) -> f32 {
        0.0
    }

    fn drawing_type(&self) -> &DrawingType {
        self.vbo.drawing_type()
    }

    fn get_visibility_check_coord(&self) -> Option<&WorldCoord> {
        None
    }
}

impl HouseDrawing {
    pub fn new(
        world_coordinate: WorldCoord,
        width: f32,
        height: f32,
        roof_height: f32,
        base_color: Color,
        light_direction: na::Vector3<f32>,
    ) -> HouseDrawing {
        let triangle_coloring: Box<TriangleColoring> =
            Box::new(AngleTriangleColoring::new(base_color, light_direction));
        let square_coloring: Box<SquareColoring> =
            Box::new(AngleSquareColoring::new(base_color, light_direction));

        let x = world_coordinate.x as f32;
        let y = world_coordinate.y as f32;
        let z = world_coordinate.z as f32;

        let a = na::Vector3::new(x - width, y - width, 0.0);
        let b = na::Vector3::new(x + width, y - width, 0.0);
        let c = na::Vector3::new(x + width, y + width, 0.0);
        let d = na::Vector3::new(x - width, y + width, 0.0);
        let e = na::Vector3::new(x - width, y - width, z + height);
        let f = na::Vector3::new(x + width, y - width, z + height);
        let g = na::Vector3::new(x + width, y + width, z + height);
        let h = na::Vector3::new(x - width, y + width, z + height);

        let s = na::Vector3::new(x, y, z + height + roof_height);

        let mut vbo = VBO::new(DrawingType::Plain);

        let mut vertices = vec![];
        vertices.append(&mut get_colored_vertices_from_square(
            &[e, h, d, a],
            &square_coloring,
        ));
        vertices.append(&mut get_colored_vertices_from_square(
            &[h, g, c, d],
            &square_coloring,
        ));
        vertices.append(&mut get_colored_vertices_from_square(
            &[g, f, b, c],
            &square_coloring,
        ));
        vertices.append(&mut get_colored_vertices_from_square(
            &[f, e, a, b],
            &square_coloring,
        ));
        vertices.append(&mut get_colored_vertices_from_triangle(
            &[h, e, s],
            &triangle_coloring,
        ));
        vertices.append(&mut get_colored_vertices_from_triangle(
            &[g, h, s],
            &triangle_coloring,
        ));
        vertices.append(&mut get_colored_vertices_from_triangle(
            &[f, g, s],
            &triangle_coloring,
        ));
        vertices.append(&mut get_colored_vertices_from_triangle(
            &[e, f, s],
            &triangle_coloring,
        ));

        vbo.load(vertices);

        HouseDrawing { vbo }
    }
}
