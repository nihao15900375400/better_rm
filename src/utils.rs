use which::which;
use std::env;
use std::path::{Path, PathBuf};
use std::io::{self, Read, Write};
use crate::constants;
use std::error::Error;
use sqlx::sqlite::SqlitePool;
use sqlx::Row;
use std::fs;

pub fn input(msg: &str) -> Result<String, io::Error> {
    print!("{}", msg);
    io::stdout().flush()?;

    let mut ipt = String::new();
    io::stdin().read_line(&mut ipt)?;

    Ok(ipt.trim().to_string())
}
pub fn to_abs_path(raw: &str) -> PathBuf {
    let mut p = if raw.starts_with('~') {
        let home = env::var_os("HOME").map(PathBuf::from).unwrap_or_default();
        let rest = &raw[1..];
        home.join(rest.strip_prefix('/').unwrap_or(rest))
    } else {
        PathBuf::from(raw)
    };

    if !p.is_absolute() {
        let cwd = env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
        p = cwd.join(p);
    }

    let mut parts = Vec::new();
    for comp in p.components() {
        match comp {
            std::path::Component::CurDir => continue,
            std::path::Component::ParentDir => {
                if !parts.is_empty() && parts.last().unwrap() != &std::path::Component::RootDir {
                    parts.pop();
                }
            }
            _ => parts.push(comp),
        }
    }

    let mut res = PathBuf::new();
    for c in parts {
        res.push(c);
    }
    res
}
pub fn is_nano_installed() -> bool {
    which("nano").is_ok()
}
pub fn is_cat_installed() -> bool {
    which("cat").is_ok()
}
pub fn create_config(path: PathBuf) -> Result<(), Box<dyn Error>> {
    if let Some(p) = path.parent() {
        std::fs::create_dir_all(p)?;
    }
    println!("Recreating: {}", path.display());
    std::fs::write(path, constants::CONFIG_JSON_DATA)?;
    println!("done.");
    Ok(())
}

pub async fn create_database(pool:SqlitePool) -> Result<(), Box<dyn Error>> {
    println!("Recreating: {}",constants::DATABASE);
    sqlx::query!(
        "DROP TABLE IF EXISTS trash;
        CREATE TABLE IF NOT EXISTS trash (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            name TEXT NOT NULL,
            hash TEXT NOT NULL,
            original_path TEXT NOT NULL,
            present_path TEXT NOT NULL,
            archive_tool TEXT NOT NULL,
            size INTEGER NOT NULL,
            time TEXT NOT NULL
        )"
    ).execute(&pool)
    .await?;
    println!("done.");
    Ok(())
}
