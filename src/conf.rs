use std::path::{Path, PathBuf};
use std::io::{self, Read, Write};
use crate::constants::*;
use crate::utils::*;
use crate::archive_and_hash::*;
use std::error::Error;
use sqlx::sqlite::SqlitePool;
use sqlx::Row;
use std::fs::File;
use serde_json;
use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize, Debug)]
pub struct Config {
    pub trash: String,
    pub archive_tool: ArchiveTool,
    pub hash_tool: HashTool,
    pub disable_list: Vec<String>
}

pub fn create_config(path: &PathBuf) -> Result<(), Box<dyn Error>> {
    println!("Recreating: {}", path.display());
    std::fs::write(path, CONFIG_JSON_DATA)?;
    println!("done.");
    Ok(())
}
pub fn load_config(path: &PathBuf) -> Result<Config, serde_json::Error> {
    let file = File::open(path).unwrap();
    let cfg:Config = serde_json::from_reader(file)?;
    Ok(cfg)
}
