use anyhow::Result;
use chrono::Utc;
use sqlx::sqlite::SqlitePool;

#[derive(Debug, sqlx::FromRow)]
struct TrashRow {
    id: i64,
    path: String,
    hash: String,
    time: i64,
}

pub async fn creat_table(pool: &SqlitePool) -> Result<()> {
    sqlx::query!(
        "CREATE TABLE IF NOT EXISTS trash (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            path TEXT NOT NULL,
            hash TEXT NOT NULL,
            time INTEGER NOT NULL
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

pub async fn select(pool: &SqlitePool, m: &str) -> Result<Vec<TrashRow>> {
    let rows: Vec<TrashRow> = sqlx::query_as!(
        TrashRow,
        "SELECT * FROM trash WHERE path LIKE ? ESCAPE '\\';",
        m
    )
    .fetch_all(pool)
    .await?;
    Ok(rows)
}

pub async fn select_all(pool: &SqlitePool) -> Result<Vec<TrashRow>> {
    let rows: Vec<TrashRow> = sqlx::query_as!(TrashRow, "SELECT * FROM trash;")
        .fetch_all(pool)
        .await?;
    Ok(rows)
}
