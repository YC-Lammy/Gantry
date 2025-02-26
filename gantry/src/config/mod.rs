mod cfg;
mod cfg_pest;

use std::collections::HashMap;

use tokio::fs::File;

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
    pub async fn parse(file: File) -> Result<Self, ()> {
        return Ok(GantryConfig {
            instances: HashMap::new(),
        });
    }
}

pub struct PrinterConfig {}

impl PrinterConfig {
    pub async fn parse(file: File) -> Result<Self, String> {
        return Ok(PrinterConfig {});
    }
}
