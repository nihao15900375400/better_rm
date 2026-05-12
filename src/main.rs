mod utils;
mod constants;
mod conf;
mod db;
mod archive_and_hash;

use std::process::Command;
use clap::Parser;
use std::path::{Path,PathBuf};
use std::error::Error;
use utils::*;
use conf::*;
use db::*;
use archive_and_hash::*;
use sqlx::sqlite::SqlitePool;
use sqlx::Row;



/// A safe file deletion utility with trash support
///
/// Provides secure file deletion, trash management, and recovery capabilities
#[derive(Parser, Debug)]
#[command(
    author, 
    version, 
    about, 
    long_about,
)]
struct Args {
    /// Target files or directories to operate on
    path: Vec<String>, 

    /// Initialize trash directory structure (will overwrite existing trash data)
    #[arg(short = 'i', long = "init")]
    init: bool,

    /// Restore file from trash to original location
    /// 
    /// If no value is provided, restores the most recently deleted file
    /// Accepts either file name or trash entry ID
    #[arg(short = 'u', long = "undo", num_args = 0..=1)]
    undo: Option<String>,

    /// Permanently delete a file from the trash
    /// 
    /// Accepts either file name or trash entry ID
    #[arg(short = 'd', long = "delete", num_args = 0..=1)]
    delete: Option<String>,

    /// Operate recursively on directories
    /// 
    /// Not required for safe trash deletion, but mandatory when using --force
    #[arg(short = 'r', long = "recurse")]
    recurse: bool,

    /// Bypass trash and permanently delete files using system rm command
    #[arg(short = 'f', long = "force")]
    force: bool,

    /// List all files currently in the trash with pagination
    #[arg(short = 'l', long = "list")]
    list: bool,

    /// Search and display files in trash matching the given pattern
    #[arg(short = 's', long = "select", num_args = 0..=1)]
    select: Option<String>,

    /// Filter files by specified metadata field
    /// 
    /// Supported fields: name, id, hash, time, original-dir
    /// Usage: --select-from name "*.txt"
    #[arg(long = "select-from", num_args = 0..=2)]
    select_from: Option<Vec<String>>,

    /// Open configuration file in nano editor
    /// 
    /// Falls back to printing the config file path if nano is not available
    #[arg(short = 'c', long = "config")]
    config: bool,

    /// Empty the entire trash (permanently delete all files)
    /// 
    /// Requires confirmation unless used with --force
    #[arg(short = 'e', long = "empty")]
    empty: bool,
}

#[tokio::main]
async fn main() -> Result<(),Box<dyn Error>>{
    let args = Args::parse();
    
    let cfg_path: PathBuf = to_abs_path(constants::CONFIG);
    let db_path: PathBuf = to_abs_path(constants::DATABASE);
    
    if let Some(p) = cfg_path.parent() {
        std::fs::create_dir_all(p);
    }
    
    let pool = SqlitePool::connect(&format!("sqlite:{}",to_abs_path(constants::DATABASE).to_string_lossy())).await.unwrap();
    let cfg = load_config(&cfg_path).unwrap();

    if let Some(p) = PathBuf::from(&cfg.trash).parent() {
        std::fs::create_dir_all(p);
    }
    
    if args.init{
        init(&cfg_path,&db_path,&pool,&cfg).await;
    }
    if args.config{
        config(&cfg_path);
    }
    if args.empty{
        empty(&pool,&cfg).await;
    }
    Ok(())
}
async fn init(cfg_path:&PathBuf,db_path:&PathBuf,pool:&SqlitePool,cfg:&Config) -> Result<(),Box<dyn Error>> {
    if cfg_path.is_file(){
        match input("Config file is exist, cover it?(Y/n)"){
            Ok(s) if s.eq_ignore_ascii_case("y")=> create_config(cfg_path)?,
            Err(e) => return Err(Box::new(e)),
            Ok(_) => {},
        }
    }else{
        create_config(cfg_path)?;
    }
    if db_path.is_file(){
        match input("Database file is exist, cover it?(Y/n)"){
            Ok(s) if s.eq_ignore_ascii_case("y")=> empty(pool,cfg).await,
            Err(e) => return Err(Box::new(e)),
            Ok(_) => {},
        }
    }else{
        create_database(pool).await?;
    }
    Ok(())
}

fn config(cfg_path: &PathBuf) {
    if is_nano_installed() {
        let _ = Command::new("nano")
            .arg(&cfg_path)
            .status();
    } else {
        println!("nano is not installed");
        println!("config file is at {}", cfg_path.display());
    }
}

async fn empty(pool:&SqlitePool,cfg:&Config){
    println!("This may empty your trash in which files will be deleted forever");
    if let Ok(s) = input("Do you really want to do that?(Y/n)") {
    if s.eq_ignore_ascii_case("y") {
        create_database(pool).await.unwrap();
        std::fs::remove_dir_all(&cfg.trash).unwrap();
        std::fs::create_dir_all(&cfg.trash).unwrap();
    }
}
}
