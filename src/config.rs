use cosmic::cosmic_config::{self, cosmic_config_derive::CosmicConfigEntry, CosmicConfigEntry};

use serde::{Deserialize, Serialize};
pub const CONFIG_VERSION: u64 = 1;

#[derive(Clone, CosmicConfigEntry, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Config {
    // #[serde(default)]
    // pub show_tooltip: bool,
    // #[serde(default)]
    // pub last_used_limit: usize,
    // #[serde(default)]
    // pub last_used: Vec<String>,
    // #[serde(default)]
    // pub font_family: String,
    // #[serde(default)]
    // pub show_unicode: bool,
}

impl Default for Config {
    fn default() -> Self {
        Self {}
    }
}
