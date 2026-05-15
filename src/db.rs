use std::path::{Path, PathBuf};
use std::io::{self, Read, Write};
use crate::constants;
use crate::conf::*;
use crate::archive::*;
use std::error::Error;
use sqlx::sqlite::SqlitePool;
use sqlx::Row;
use std::fs;
use serde::{Serialize, Deserialize};


#[derive(Debug,sqlx::FromRow,Serialize, Deserialize,Clone)]
pub struct Database{
    pub id: i64,
    pub name: String,
    pub original_path: String,
    pub present_path: String,
    pub archive_tool: ArchiveTool,
    pub size: i64,
    pub time: String,
}

pub async fn create_database(pool: &SqlitePool) -> Result<(), sqlx::Error> {
    println!("Creating: {}",constants::DATABASE);
    sqlx::query!(
        r#"DROP TABLE IF EXISTS trash;
        CREATE TABLE IF NOT EXISTS trash (
            id INTEGER PRIMARY KEY,
            name TEXT NOT NULL,
            original_path TEXT NOT NULL,
            present_path TEXT NOT NULL,
            archive_tool TEXT NOT NULL,
            size INTEGER NOT NULL,
            time TEXT NOT NULL UNIQUE
        )"#
    ).execute(pool)
    .await?;
    println!("done.");
    Ok(())
}
pub async fn insert(pool: &SqlitePool,row: &Database) -> Result<(), sqlx::Error>{
    sqlx::query!(
        r#"
INSERT INTO trash
(
    name,
    original_path,
    present_path,
    archive_tool,
    size,
    time
)
VALUES
(?, ?, ?, ?, ?, ?);
"#,row.name,
row.original_path,
row.present_path,
row.archive_tool,
row.size,
row.time,
    ).execute(pool)
    .await?;
    Ok(())
}
async fn select(
    pool: &SqlitePool,
    column: &str,
    like: &str
) -> Result<Vec<Database>, sqlx::Error> {
    let sql = match column {
        "name" => r#"SELECT * FROM trash WHERE name LIKE ?"#,
        "id" => r#"SELECT * FROM trash WHERE id LIKE ?"#,
        "time" => r#"SELECT * FROM trash WHERE time LIKE ?"#,
        "original-path" => r#"SELECT * FROM trash WHERE original_path LIKE ? ESCAPE '\'"#,
        "size" => r#"SELECT * FROM trash WHERE size LIKE ? ESCAPE '\'"#,
        _ => return Ok(Vec::new()),
    };

    let files = sqlx::query_as::<_, Database>(sql)
        .bind(like)
        .fetch_all(pool)
        .await?;

    Ok(files)
}

