use anyhow::Result;
use chrono::{Local, TimeZone, Utc};
use multi_select::TableRow;
use sqlx::sqlite::SqlitePool;

#[derive(Debug, sqlx::FromRow, Clone, PartialEq)]
pub struct TrashRow {
    pub id: i64,
    pub path: String,
    pub hash: String,
    pub time: i64,
    pub size: String,
}

pub fn fmt_time(s: i64) -> String {
    let sec: i64 = s / 1000;
    let nsec: u32 = ((s % 1000) * 1_000_000) as u32;
    let dt: chrono::prelude::DateTime<Local> = Local.timestamp_opt(sec, nsec).unwrap();
    dt.format("%Y-%m-%d %H:%M:%S%.3f").to_string()
}

impl TableRow for TrashRow {
    fn header() -> Vec<String> {
        vec!["name".into(), "size".into(), "time".into()]
    }
    fn row(&self) -> Vec<String> {
        vec![
            std::path::Path::new(&self.path)
                .file_name()
                .unwrap()
                .to_str()
                .unwrap()
                .into(),
            self.size.clone(),
            fmt_time(self.time),
        ]
    }
}

pub async fn creat_table(pool: &SqlitePool) -> Result<()> {
    sqlx::query!(
        "CREATE TABLE IF NOT EXISTS trash (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            path TEXT NOT NULL,
            hash TEXT NOT NULL,
            time BIGINT NOT NULL,
            size TEXT NOT NULL
        );"
    )
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn insert(pool: &SqlitePool, path: &str, hash: &str) -> Result<()> {
    let time = Utc::now().timestamp_millis();
    sqlx::query!(
        "INSERT INTO trash (path,hash,time) VALUES (?,?,?);",
        path,
        hash,
        time
    )
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn select_all(pool: &SqlitePool) -> Result<Vec<TrashRow>> {
    let rows: Vec<TrashRow> = sqlx::query_as!(TrashRow, "SELECT * FROM trash;")
        .fetch_all(pool)
        .await?;
    Ok(rows)
}

pub async fn remove(pool: &SqlitePool, to_del: &[TrashRow]) -> Result<()> {
    for i in to_del {
        sqlx::query!("DELETE FROM trash WHERE id = ?;", i.id,)
            .execute(pool)
            .await?;
    }
    Ok(())
}
