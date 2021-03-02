use super::*;
use crate::avatar::*;
use crate::resource::Resource;
use commons::{na, V3};
use isometric::coords::*;
use isometric::drawing::{
    create_billboard, create_boat, draw_boat, update_billboard_texture, update_billboard_vertices,
    DrawBoatParams,
};
use isometric::Color;
use isometric::Command;
use std::collections::HashMap;
use std::iter::once;

pub struct AvatarArtist {
    params: AvatarArtistParams,
    previous_draw_actions: HashMap<String, AvatarDrawAction>,
}

pub struct AvatarArtistParams {
    boat_params: DrawBoatParams,
    load_size: f32,
    load_height: f32,
}

impl AvatarArtistParams {
    pub fn new(light_direction: &V3<f32>) -> AvatarArtistParams {
        AvatarArtistParams {
            boat_params: DrawBoatParams {
                width: 0.13,
                side_height: 0.04,
                bow_length: 0.06,
                mast_height: 0.4,
                base_color: Color::new(0.46875, 0.257_812_5, 0.070_312_5, 0.8),
                sail_color: Color::new(1.0, 1.0, 1.0, 1.0),
                light_direction: *light_direction,
            },
            load_size: 0.15,
            load_height: 0.3,
        }
    }
}

pub struct AvatarDrawCommand<'a> {
    pub avatar: &'a Avatar,
    pub draw_when_done: bool,
}

impl AvatarArtist {
    pub fn new(params: AvatarArtistParams) -> AvatarArtist {
        AvatarArtist {
            params,
            previous_draw_actions: HashMap::new(),
        }
    }

    pub fn init(&self, name: &str) -> Vec<Command> {
        once(create_boat(boat_drawing_name(&name)))
            .chain(once(create_billboard(load_drawing_name(&name))))
            .collect()
    }

    pub fn update_avatars(
        &mut self,
        commands: &[AvatarDrawCommand],
        instant: &u128,
    ) -> Vec<Command> {
        let mut out = vec![];
        out.append(&mut self.draw_avatars(commands, instant));
        out
    }

    fn draw_avatars(&mut self, commands: &[AvatarDrawCommand], instant: &u128) -> Vec<Command> {
        commands
            .iter()
            .flat_map(|command| self.draw_command(command, instant))
            .collect()
    }

    fn draw_command(&mut self, command: &AvatarDrawCommand, instant: &u128) -> Vec<Command> {
        let mut out = vec![];
        let avatar = command.avatar;
        let name = &avatar.name;
        let new_draw_action = avatar_draw_action(&command, &instant);
        let previous_draw_action = self.previous_draw_actions.get(name);
        if let Some(previous_draw_action) = previous_draw_action {
            if !Self::should_redraw_avatar(&previous_draw_action, &new_draw_action) {
                return vec![];
            }
        } else {
            out.append(&mut self.init(name));
        }
        self.previous_draw_actions
            .insert(name.to_string(), new_draw_action);

        match new_draw_action {
            AvatarDrawAction::Draw => out.append(&mut self.draw_avatar(avatar, instant)),
            AvatarDrawAction::Hide => out.append(&mut self.hide(name)),
        }
        out
    }

    fn draw_avatar(&self, avatar: &Avatar, instant: &u128) -> Vec<Command> {
        let journey = avatar.journey.as_ref().unwrap();
        let world_coord = journey.world_coord_at(instant);
        let mut out = vec![];
        out.append(&mut self.draw_boat_if_required(&avatar.name, &journey, world_coord, instant));
        out.append(&mut self.draw_load(
            &avatar.name,
            &journey.progress_at(instant).load(),
            world_coord,
        ));
        out
    }

    #[rustfmt::skip]
    fn get_rotation_matrix(journey: &Journey, instant: &u128) -> na::Matrix3<f32> {
        let rotation = journey.progress_at(instant).rotation();
        let cos = rotation.angle().cos();
        let sin = rotation.angle().sin();
        na::Matrix3::from_vec(vec![
            cos, sin, 0.0,
            -sin, cos, 0.0,
            0.0, 0.0, 1.0,
        ])
    }

    fn should_redraw_avatar(
        previous_draw_action: &AvatarDrawAction,
        new_draw_action: &AvatarDrawAction,
    ) -> bool {
        if let AvatarDrawAction::Draw = new_draw_action {
            true
        } else {
            previous_draw_action != new_draw_action
        }
    }

    fn draw_boat_if_required(
        &self,
        name: &str,
        journey: &Journey,
        world_coord: WorldCoord,
        instant: &u128,
    ) -> Vec<Command> {
        if self.should_draw_boat(journey, instant) {
            self.draw_boat(name, journey, world_coord, instant)
        } else {
            vec![self.hide_boat(name)]
        }
    }

    fn should_draw_boat(&self, journey: &Journey, instant: &u128) -> bool {
        journey.progress_at(instant).vehicle() == Vehicle::Boat
    }

    fn draw_boat(
        &self,
        name: &str,
        journey: &Journey,
        world_coord: WorldCoord,
        instant: &u128,
    ) -> Vec<Command> {
        draw_boat(
            &boat_drawing_name(name),
            world_coord,
            AvatarArtist::get_rotation_matrix(journey, instant),
            &self.params.boat_params,
        )
    }

    fn draw_load(
        &self,
        name: &str,
        load: &AvatarLoad,
        mut world_coord: WorldCoord,
    ) -> Vec<Command> {
        if let AvatarLoad::Resource(resource) = load {
            let texture = unwrap_or!(
                resource_texture(*resource),
                return vec![self.hide_load(name)]
            );
            let mut out = vec![];
            let name = load_drawing_name(name);
            world_coord.z += self.params.load_height;
            out.append(&mut update_billboard_vertices(
                name.clone(),
                world_coord,
                self.params.load_size,
                self.params.load_size,
            ));
            out.push(update_billboard_texture(name, texture));
            out
        } else {
            vec![self.hide_load(name)]
        }
    }

    fn hide(&self, name: &str) -> Vec<Command> {
        once(self.hide_boat(name))
            .chain(once(self.hide_load(name)))
            .collect()
    }

    fn hide_boat(&self, name: &str) -> Command {
        Command::SetDrawingVisibility {
            name: boat_drawing_name(name),
            visible: false,
        }
    }

    fn hide_load(&self, name: &str) -> Command {
        Command::SetDrawingVisibility {
            name: load_drawing_name(name),
            visible: false,
        }
    }
}

fn drawing_name(name: &str, part: &str) -> String {
    format!("avatar-{}-{}", name.to_string(), part)
}

fn boat_drawing_name(name: &str) -> String {
    drawing_name(name, "boat")
}

fn load_drawing_name(name: &str) -> String {
    drawing_name(name, "load")
}

fn resource_texture(resource: Resource) -> Option<&'static str> {
    match resource {
        Resource::Bananas => Some("resources/textures/twemoji/bananas.png"),
        Resource::Bison => Some("resources/textures/twemoji/bison.png"),
        Resource::Coal => Some("resources/textures/twemoji/derivative/coal.png"),
        Resource::Crabs => Some("resources/textures/twemoji/crabs.png"),
        Resource::Crops => Some("resources/textures/twemoji/wheat.png"),
        Resource::Deer => Some("resources/textures/twemoji/deer.png"),
        Resource::Fur => Some("resources/textures/twemoji/fur.png"),
        Resource::Gems => Some("resources/textures/twemoji/gems.png"),
        Resource::Gold => Some("resources/textures/twemoji/gold.png"),
        Resource::Iron => Some("resources/textures/twemoji/derivative/iron.png"),
        Resource::Ivory => Some("resources/textures/twemoji/ivory.png"),
        Resource::Pasture => Some("resources/textures/twemoji/cow.png"),
        Resource::Spice => Some("resources/textures/twemoji/spice.png"),
        Resource::Stone => Some("resources/textures/twemoji/stone.png"),
        Resource::Truffles => Some("resources/textures/twemoji/truffles.png"),
        Resource::Whales => Some("resources/textures/twemoji/whales.png"),
        Resource::Wood => Some("resources/textures/twemoji/wood.png"),
        _ => None,
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum AvatarDrawAction {
    Draw,
    Hide,
}

fn avatar_draw_action(command: &AvatarDrawCommand, instant: &u128) -> AvatarDrawAction {
    match &command.avatar.journey {
        Some(journey) => match command.draw_when_done || !journey.done(instant) {
            true => AvatarDrawAction::Draw,
            false => AvatarDrawAction::Hide,
        },
        None => AvatarDrawAction::Hide,
    }
}
