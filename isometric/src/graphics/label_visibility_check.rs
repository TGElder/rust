use super::{GLZFinder, GraphicsEngine};
use commons::na::Matrix2;
use commons::rectangle::Rectangle;
use commons::{v2, V2};
use coords::WorldCoord;
use coords::{GLCoord2D, GLCoord3D, ZFinder};
use glutin::dpi::PhysicalSize;
use transform::Transform;

const VISIBILITY_TOLERANCE: f32 = 0.01;

#[derive(Debug)]
pub struct LabelVisibilityCheck {
    pub world_coord: WorldCoord,
    pub ui_offsets: Rectangle<f32>,
}

pub struct LabelVisibilityChecker<'a> {
    padding: f32,
    transform: &'a Transform,
    physical_size: &'a PhysicalSize,
    z_finder: &'a dyn ZFinder,
    pixel_to_screen: Matrix2<f32>,
    ui_elements: Vec<Rectangle<f32>>,
}

impl<'a> LabelVisibilityChecker<'a> {
    pub fn new(graphics_engine: &'a GraphicsEngine) -> LabelVisibilityChecker<'a> {
        LabelVisibilityChecker {
            padding: graphics_engine.label_padding,
            transform: &graphics_engine.transform,
            physical_size: &graphics_engine.viewport_size,
            z_finder: &GLZFinder {},
            pixel_to_screen: graphics_engine.get_pixel_to_screen(),
            ui_elements: vec![],
        }
    }

    pub fn is_visible(&mut self, check: &LabelVisibilityCheck) -> bool {
        if let VisibleBeforeUI::Yes(visible_coord) = self.visible_before_ui(check) {
            let ui_element = self.get_ui_element(&visible_coord, &check.ui_offsets);
            if self.ui_element_is_visible(&ui_element) {
                self.ui_elements.push(ui_element);
                return true;
            }
        }
        false
    }

    fn visible_before_ui(&self, check: &'a LabelVisibilityCheck) -> VisibleBeforeUI {
        let expected_gl_coord_4 = check.world_coord.to_gl_coord_4d(self.transform);
        let gl_coord_2 = GLCoord2D::new(expected_gl_coord_4.x, expected_gl_coord_4.y);
        let visibile_screen_coord = gl_coord_2.to_gl_coord_3d(self.physical_size, self.z_finder);

        if expected_gl_coord_4.z - visibile_screen_coord.z < VISIBILITY_TOLERANCE {
            VisibleBeforeUI::Yes(visibile_screen_coord)
        } else {
            VisibleBeforeUI::No
        }
    }

    fn get_ui_element(&self, screen_coord: &GLCoord3D, offsets: &Rectangle<f32>) -> Rectangle<f32> {
        Rectangle {
            from: self.to_screen_coord(screen_coord, offsets.from),
            to: self.to_screen_coord(screen_coord, offsets.to),
        }
    }

    fn to_screen_coord(&self, screen_coord: &GLCoord3D, offset: V2<f32>) -> V2<f32> {
        self.pixel_to_screen * (offset * self.padding) + v2(screen_coord.x, screen_coord.y)
    }

    fn ui_element_is_visible(&self, ui_element: &Rectangle<f32>) -> bool {
        !self
            .ui_elements
            .iter()
            .any(|other| other.overlaps(ui_element))
    }
}

enum VisibleBeforeUI {
    Yes(GLCoord3D),
    No,
}

#[cfg(test)]
mod tests {

    use super::*;
    use coords::{BufferCoordinate, GLCoord2D};
    use transform::Identity;

    struct MockZFinder {}

    impl ZFinder for MockZFinder {
        fn get_z_at(&self, _: BufferCoordinate) -> f32 {
            1.0
        }
    }

    fn test(test_fn: &dyn Fn(&mut LabelVisibilityCheck, &mut LabelVisibilityChecker)) {
        let mut check = LabelVisibilityCheck {
            world_coord: WorldCoord::new(1.0, 2.0, 3.0),
            ui_offsets: Rectangle {
                from: v2(-1.0, -1.0),
                to: v2(1.0, 1.0),
            },
        };

        let transform = Transform::new(
            GLCoord3D::new(1.0, 1.0, 1.0),
            GLCoord2D::new(0.0, 0.0),
            Box::new(Identity {}),
        );
        let physical_size = PhysicalSize::new(100.0, 100.0);
        let z_finder = MockZFinder {};

        let mut checker = LabelVisibilityChecker {
            padding: 1.0,
            transform: &transform,
            physical_size: &physical_size,
            z_finder: &z_finder,
            pixel_to_screen: Matrix2::identity(),
            ui_elements: vec![],
        };

        test_fn(&mut check, &mut checker)
    }

    #[test]
    fn should_be_invisible_if_world_coord_invisible() {
        test(&move |check, checker| assert!(!checker.is_visible(&check)))
    }

    #[test]
    fn should_be_visible_if_world_coord_visible_and_no_overlapping_ui_elements() {
        test(&move |check, checker| {
            check.world_coord.z = 1.0;
            checker.ui_elements.push(Rectangle {
                from: v2(3.0, 3.0),
                to: v2(4.0, 4.0),
            });
            assert!(checker.is_visible(&check))
        });
    }

    #[test]
    fn should_be_invisible_if_world_coord_visible_and_overlapping_ui_elements() {
        test(&move |check, checker| {
            check.world_coord.z = 1.0;
            checker.ui_elements.push(Rectangle {
                from: v2(1.0, 2.0),
                to: v2(3.0, 4.0),
            });
            assert!(!checker.is_visible(&check))
        });
    }

    #[test]
    fn should_be_invisible_if_world_coord_visible_and_overlapping_ui_elements_due_to_padding() {
        test(&move |check, checker| {
            check.world_coord.z = 1.0;
            checker.padding = 3.0;
            checker.ui_elements.push(Rectangle {
                from: v2(3.0, 3.0),
                to: v2(4.0, 4.0),
            });
            assert!(!checker.is_visible(&check))
        });
    }
}
