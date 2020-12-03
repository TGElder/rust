use crate::names::{ListNamer, Namer};
use isometric::Color;
use serde::{Deserialize, Serialize};
use std::fmt::Debug;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct NationDescription {
    pub name: String,
    pub color: Color,
    pub skin_color: Color,
    pub town_name_file: String,
}

#[derive(Serialize, Deserialize)]
pub struct Nation {
    description: NationDescription,
    #[serde(skip)]
    town_namer: Option<ListNamer>,
}

impl Nation {
    pub fn from_description(description: &NationDescription) -> Nation {
        Nation {
            description: description.clone(),
            town_namer: None,
        }
    }

    pub fn description(&self) -> &NationDescription {
        &self.description
    }

    pub fn color(&self) -> &Color {
        &self.description.color
    }

    pub fn skin_color(&self) -> &Color {
        &self.description.skin_color
    }

    fn lazy_town_namer(&mut self) -> &mut ListNamer {
        let town_name_file = &self.description.town_name_file;
        self.town_namer
            .get_or_insert_with(|| ListNamer::from_file(town_name_file))
    }

    pub fn get_town_name(&mut self) -> String {
        self.lazy_town_namer().next_name()
    }
}

impl Debug for Nation {
    fn fmt(
        &self,
        formatter: &mut std::fmt::Formatter<'_>,
    ) -> std::result::Result<(), std::fmt::Error> {
        self.description.fmt(formatter)
    }
}

impl PartialEq for Nation {
    fn eq(&self, other: &Nation) -> bool {
        self.description.eq(&other.description)
    }
}

pub fn nation_descriptions() -> Vec<NationDescription> {
    vec![
        NationDescription {
            name: "China".to_string(),
            color: Color::new(1.0, 0.7, 0.0, 1.0),
            skin_color: medium_light_skin(),
            town_name_file: "resources/names/towns/china".to_string(),
        },
        NationDescription {
            name: "France".to_string(),
            color: Color::new(0.0, 0.0, 0.5, 1.0),
            skin_color: light_skin(),
            town_name_file: "resources/names/towns/france".to_string(),
        },
        NationDescription {
            name: "Germany".to_string(),
            color: Color::new(0.0, 0.0, 0.0, 1.0),
            skin_color: light_skin(),
            town_name_file: "resources/names/towns/germany".to_string(),
        },
        NationDescription {
            name: "India".to_string(),
            color: Color::new(1.0, 0.4, 0.0, 1.0),
            skin_color: medium_dark_skin(),
            town_name_file: "resources/names/towns/india".to_string(),
        },
        NationDescription {
            name: "Indonesia".to_string(),
            color: Color::new(1.0, 0.0, 0.0, 1.0),
            skin_color: medium_skin(),
            town_name_file: "resources/names/towns/indonesia".to_string(),
        },
        NationDescription {
            name: "Iran".to_string(),
            color: Color::new(0.0, 1.0, 0.0, 1.0),
            skin_color: medium_light_skin(),
            town_name_file: "resources/names/towns/iran".to_string(),
        },
        NationDescription {
            name: "Italy".to_string(),
            color: Color::new(1.0, 1.0, 1.0, 1.0),
            skin_color: medium_light_skin(),
            town_name_file: "resources/names/towns/italy".to_string(),
        },
        NationDescription {
            name: "Japan".to_string(),
            color: Color::new(0.5, 0.0, 0.0, 1.0),
            skin_color: medium_light_skin(),
            town_name_file: "resources/names/towns/japan".to_string(),
        },
        NationDescription {
            name: "Nigeria".to_string(),
            color: Color::new(0.5, 1.0, 0.5, 1.0),
            skin_color: dark_skin(),
            town_name_file: "resources/names/towns/nigeria".to_string(),
        },
        NationDescription {
            name: "Russia".to_string(),
            color: Color::new(0.0, 0.0, 1.0, 1.0),
            skin_color: light_skin(),
            town_name_file: "resources/names/towns/russia".to_string(),
        },
        NationDescription {
            name: "Spain".to_string(),
            color: Color::new(1.0, 1.0, 0.0, 1.0),
            skin_color: medium_light_skin(),
            town_name_file: "resources/names/towns/spain".to_string(),
        },
        NationDescription {
            name: "Thailand".to_string(),
            color: Color::new(0.5, 0.0, 0.5, 1.0),
            skin_color: medium_skin(),
            town_name_file: "resources/names/towns/thailand".to_string(),
        },
        NationDescription {
            name: "Turkey".to_string(),
            color: Color::new(0.0, 1.0, 1.0, 1.0),
            skin_color: medium_light_skin(),
            town_name_file: "resources/names/towns/turkey".to_string(),
        },
        NationDescription {
            name: "United Kingdom".to_string(),
            color: Color::new(1.0, 0.0, 1.0, 1.0),
            skin_color: light_skin(),
            town_name_file: "resources/names/towns/united_kingdom".to_string(),
        },
    ]
}

fn light_skin() -> Color {
    Color::new(0.96, 0.87, 0.80, 1.0)
}

fn medium_light_skin() -> Color {
    Color::new(0.95, 0.82, 0.63, 1.0)
}

fn medium_skin() -> Color {
    Color::new(0.83, 0.67, 0.53, 1.0)
}

fn medium_dark_skin() -> Color {
    Color::new(0.68, 0.49, 0.34, 1.0)
}

fn dark_skin() -> Color {
    Color::new(0.49, 0.33, 0.24, 1.0)
}

pub fn skin_colors() -> [Color; 5] {
    [
        light_skin(),
        medium_light_skin(),
        medium_skin(),
        medium_dark_skin(),
        dark_skin(),
    ]
}
