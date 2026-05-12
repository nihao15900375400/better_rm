use std::path::{Path, PathBuf};
use std::io::{self, Read, Write};
use crate::constants;
use std::error::Error;
use sqlx::sqlite::SqlitePool;
use sqlx::Row;
use std::fs;

pub async fn create_database(pool: &SqlitePool) -> Result<(), Box<dyn Error>> {
    println!("Creating: {}",constants::DATABASE);
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
    ).execute(pool)
    .await?;
    println!("done.");
    Ok(())
}
