use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::avatar::Avatar;

#[derive(Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Avatars {
    pub all: HashMap<String, Avatar>,
    pub selected: Option<String>,
}

impl Avatars {
    pub fn selected(&self) -> Option<&Avatar> {
        self.selected
            .as_ref()
            .and_then(|avatar| self.all.get(avatar))
    }
}
