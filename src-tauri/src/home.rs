//! Persistent decoration state. Coordinates use the room's 320×240 canvas.
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

pub const ROOMS: [&str; 3] = ["hearth", "studio", "loft"];
pub const SETTING_HOME_LAYOUT: &str = "home_layout_v1";
pub const GARDEN_LEVEL: i64 = 3;
pub const FLOWER_GROW_SECONDS: i64 = 1800;

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct HomeLayout {
    pub placements: BTreeMap<i64, Placement>,
    pub wallpapers: BTreeMap<String, String>,
    pub garden: BTreeMap<u8, Flower>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Flower {
    pub species: String,
    pub planted_at: i64,
    pub watered_at: Option<i64>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Placement {
    pub room: String,
    pub x: f64,
    pub y: f64,
}

impl Placement {
    pub fn validate(&self) -> crate::Result<()> {
        if !ROOMS.contains(&self.room.as_str()) {
            return Err(crate::Error::Invalid("Choose a room in your cottage".into()));
        }
        if !self.x.is_finite() || !self.y.is_finite() || !(20.0..=300.0).contains(&self.x) || !(30.0..=215.0).contains(&self.y) {
            return Err(crate::Error::Invalid("Keep this treasure inside the room".into()));
        }
        Ok(())
    }
}
