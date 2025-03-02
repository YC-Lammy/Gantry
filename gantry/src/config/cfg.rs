use std::collections::HashMap;

#[derive(Debug)]
pub struct Config {
    pub sections: Vec<Section>,
}

impl Config {
    pub fn parse(file: &str) -> Result<Self, pest::error::Error<super::cfg_pest::Rule>> {
        return super::cfg_pest::parse_cfg(file);
    }
}

#[derive(Debug)]
pub struct Section {
    pub prefix_name: String,
    pub suffix_name: Option<String>,
    pub values: HashMap<String, Value>,
}

#[derive(Debug, PartialEq)]
pub enum Value {
    Number(f64),
    NumberArray(Vec<f64>),
    /// calculated ratio, for example 80:8 would become 10
    Ratio(f64),
    String(String),
    StringArray(Vec<String>),
}
