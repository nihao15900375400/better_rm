use crate::archive::*;
use crate::constants::*;
use serde::{Deserialize, Serialize};
use serde_json;
use std::error::Error;
use std::fs::File;
use std::path::PathBuf;

#[derive(Serialize, Deserialize, Debug)]
pub struct Config {
    pub trash: String,
    pub archive_tool: ArchiveTool,
    pub disable_list: Vec<String>,
}

pub fn create_config(path: &PathBuf) -> Result<(), Box<dyn Error>> {
    println!("Creating: {}", path.display());
    std::fs::write(path, CONFIG_JSON_DATA)?;
    println!("done.");
    Ok(())
}
pub fn load_config(path: &PathBuf) -> Result<Config, serde_json::Error> {
    let file = File::open(path).unwrap();
    let cfg: Config = serde_json::from_reader(file)?;
    Ok(cfg)
}
