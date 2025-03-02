mod cfg;
mod cfg_pest;

pub use cfg::Config as PrinterConfig;

use std::collections::HashMap;

pub struct GantryConfig {
    /// printer instances to boot up
    pub instances: HashMap<String, InstanceConfig>,
}

pub struct InstanceConfig {
    /// uuid
    pub uuid: u128,
    /// path to the printer config for instance
    pub config_path: String,
}

impl GantryConfig {
    pub async fn parse(_file: &str) -> Result<Self, ()> {
        return Ok(GantryConfig {
            instances: HashMap::new(),
        });
    }
}
