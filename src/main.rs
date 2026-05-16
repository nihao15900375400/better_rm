mod archive;
mod conf;
mod constants;
mod db;
mod utils;

use archive::*;
use clap::Parser;
use conf::*;
use db::*;
use sqlx::sqlite::SqlitePool;
use std::error::Error;
use std::path::PathBuf;
use std::process::Command;
use tabled::{
    Table, Tabled,
    settings::{
        Alignment, Style, Width,
        object::{Columns, Rows},
    },
};
use utils::*;

/// A safe file deletion utility with trash support
///
/// Provides secure file deletion, trash management, and recovery capabilities
#[derive(Parser, Debug)]
#[command(author, version, about, long_about)]
struct Args {
    /// Target files or directories to operate on
    path: Vec<String>,

    /// Initialize trash directory structure (will overwrite existing trash data)
    #[arg(short = 'i', long = "init")]
    init: bool,

    /// Restore file from trash to original location
    /// If no value is provided, restores the most recently deleted file
    /// Accepts trash entry ID
    /// Make sure there is no file with the same name in the source folder
    #[arg(short = 'u', long = "undo", num_args = 0..)]
    undo: Option<Vec<String>>,

    /// Permanently delete a file from the trash
    /// Accepts trash entry ID
    #[arg(short = 'd', long = "delete", num_args = 0..)]
    delete: Option<Vec<String>>,

    /// Operate recursively on directories
    /// Not required for safe trash deletion, but mandatory when using --force
    #[arg(short = 'r', long = "recurse")]
    recurse: bool,

    /// Bypass trash and permanently delete files using system rm command
    /// If you want to pass in other args, write them after `--`
    /// Usage: del -rf -- -i example/
    #[arg(short = 'f', long = "force")]
    force: bool,

    /// List all files currently in the trash with pagination
    #[arg(short = 'l', long = "list")]
    list: bool,

    /// Search and display files in trash matching it's name
    /// Use `%` to express any sequence of characters.
    /// Use `_` to express any single character.
    /// Usage: --select "%.txt"
    #[arg(short = 's', long = "select", num_args = 0..=1)]
    select: Option<String>,

    /// Filter records by specifying a database field with fuzzy matching.
    /// Supported filter fields: name, id, hash, time, original-path, size
    /// Wildcard syntax:
    /// - `%`  Matches any sequence of zero or more characters
    /// - `_`  Matches exactly one single arbitrary character
    /// Escape rule:
    /// Use backslash `\` to escape literal `%` and `_`,
    /// so they are treated as normal characters instead of wildcards.
    /// Example usage:
    /// --select-from time 202_-12-%\_10:04:__
    #[arg(long = "select-from", num_args = 0..=2)]
    select_from: Option<Vec<String>>,

    /// Open configuration file in nano editor
    /// Falls back to printing the config file path if nano is not available
    #[arg(short = 'c', long = "config")]
    config: bool,

    /// Requires confirmation unless used with --force
    #[arg(short = 'e', long = "empty")]
    empty: bool,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let args = Args::parse();

    let cfg_path: PathBuf = to_abs_path(constants::CONFIG);
    let db_path: PathBuf = to_abs_path(constants::DATABASE);

    if let Some(p) = cfg_path.parent() {
        std::fs::create_dir_all(p)?;
    }
    if args.init {
        init(&cfg_path, &db_path).await?;
        return Ok(());
    }
    let pool = SqlitePool::connect(&format!(
        "sqlite:{}",
        to_abs_path(constants::DATABASE).to_string_lossy()
    ))
    .await
    .unwrap_or_else(|_| {
        eprintln!("there's no config file existing, use `del --init` to create");
        std::process::exit(1)
    });
    let cfg = load_config(&cfg_path).unwrap_or_else(|_| {
        eprintln!("there's no config file existing, use `del --init` to create");
        std::process::exit(1)
    });

    if let Some(p) = to_abs_path(&cfg.trash).parent() {
        std::fs::create_dir_all(p)?;
    }

    if args.config {
        config(&cfg_path);
        return Ok(());
    }
    if args.empty {
        empty(&pool, &cfg).await;
        return Ok(());
    }
    if args.force {
        let mut cmd = Command::new("rm");
        if args.recurse {
            cmd.arg("-r");
        }
        cmd.args(args.path);
        let _ = cmd.status();
        return Ok(());
    }
    if args.list {
        list(&pool).await?;
        return Ok(());
    }
    if let Some(name) = args.select {
        let res = select(&pool,"name",&name).await?;
        if res.is_empty(){
            eprintln!("Not found {}",name);
            std::process::exit(1);
        }
        show(res).await?;
        return Ok(());
    }
    if let Some(v) = args.select_from {
        let res = select(&pool,v.first().unwrap(),v.last().unwrap()).await?;
        if res.is_empty(){
            eprintln!("Not found {} from {}",v.last().unwrap(),v.first().unwrap());
            std::process::exit(1);
        }
        show(res).await?;
        return Ok(());
    }
    if let Some(ids) = args.delete {
        if ids.is_empty() {
            eprintln!("There should have at least one id followed");
            std::process::exit(1);
        }else{
            for id in ids{
                let res = select(&pool,"id",&id).await.unwrap();
                if res.is_empty(){
                    eprintln!("No id found");
                    std::process::exit(1);
                }
                sqlx::query!("DELETE FROM trash WHERE id = ?;", res.first().unwrap().id)
                        .execute(&pool)
                        .await;
            }
        }
        return Ok(());
    }
    if let Some(names) = args.undo {
        if names.is_empty() {
            let db = sqlx::query_as!(
                Database,
                r#"SELECT * FROM trash
ORDER BY id DESC
LIMIT 1;
"#
            )
            .fetch_all(&pool)
            .await
            .unwrap();
            match restore(db.clone()).await {
                Ok(_) => {
                    sqlx::query!("DELETE FROM trash WHERE id = ?;", db.first().unwrap().id)
                        .execute(&pool)
                        .await;
                }
                Err(e) => {
                    eprintln!("{}", e);
                    std::process::exit(1);
                }
            }
        } else {
            for i in names {
                let id = select(&pool, "id", &i).await.unwrap();
                match restore(id.clone()).await {
                    Ok(_) => {
                        sqlx::query!("DELETE FROM trash WHERE id = ?;", id.first().unwrap().id)
                            .execute(&pool)
                            .await;
                    }
                    Err(e) => {
                        eprintln!("{}", e);
                        std::process::exit(1);
                    }
                }
            }
        }
        return Ok(());
    }
    if args.path.is_empty() {
        println!("Use `del --help` to know more");
    } else {
        for rubbish in args.path {
            remove(to_abs_path(&rubbish), &pool, &cfg).await?;
        }
    }
    Ok(())
}
async fn restore(id: Vec<Database>) -> Result<(), Box<dyn Error>> {
    let is_not_id = id.is_empty();
    if is_not_id {
        eprintln!("No id found");
        std::process::exit(1);
    } else if !is_not_id && id.len() == 1 {
        println!("Found as id");
        let a = id.first().unwrap();
        println!("delete from {} at {}", a.original_path, a.time);
        unpack(a)?;
    }
    Ok(())
}
async fn init(cfg_path: &PathBuf, db_path: &PathBuf) -> Result<(), Box<dyn Error>> {
    let pool = SqlitePool::connect(&format!(
        "sqlite:{}",
        to_abs_path(constants::DATABASE).to_string_lossy()
    ))
    .await?;
    if cfg_path.is_file() {
        match input("Config file is exist, cover it?(Y/n) ") {
            Ok(s) if s.eq_ignore_ascii_case("y") => create_config(cfg_path)?,
            Err(e) => return Err(Box::new(e)),
            Ok(_) => {}
        }
    } else {
        create_config(cfg_path)?;
    }
    let cfg = load_config(&to_abs_path(constants::CONFIG))?;
    if db_path.is_file() {
        match input("Database file is exist, cover it?(Y/n) ") {
            Ok(s) if s.eq_ignore_ascii_case("y") => empty(&pool, &cfg).await,
            Err(e) => return Err(Box::new(e)),
            Ok(_) => {}
        }
    } else {
        create_database(&pool).await?;
    }
    Ok(())
}

fn config(cfg_path: &PathBuf) {
    if is_nano_installed() {
        println!("cmd: `nano {}`", cfg_path.display());
        let _ = Command::new("nano").arg(&cfg_path).status();
    } else {
        println!("nano is not installed");
        println!("config file is at {}", cfg_path.display());
    }
}

async fn empty(pool: &SqlitePool, cfg: &Config) {
    println!("This may empty your trash in which files will be deleted forever");
    if let Ok(s) = input("Do you really want to do that?(Y/n)") {
        if s.eq_ignore_ascii_case("y") {
            create_database(pool).await.unwrap();
            let trash_path = to_abs_path(&cfg.trash);
            std::fs::remove_dir_all(&trash_path).unwrap();
            std::fs::create_dir_all(&trash_path).unwrap();
        }
    }
}
#[derive(Debug, Tabled)]
pub struct TrashRow {
    pub id: i64,
    pub name: String,
    pub path: String,
    #[tabled(rename = "type")]
    pub archive_tool: ArchiveTool,
    pub size: String,
    pub time: String,
}
async fn show(files: Vec<Database>)-> Result<(), Box<dyn Error>> {
    let pager = minus::Pager::new();
    pager.set_prompt("List trash | press 'q' to exit")?;
        let view_list: Vec<TrashRow> = files
            .into_iter()
            .map(|row| TrashRow {
                id: row.id,
                name: row.name,
                path: row.original_path,
                archive_tool: row.archive_tool,
                size: format_size(row.size),
                time: row.time,
            })
            .collect();

        let mut table = Table::new(view_list);
        table.with(Style::blank());
        table.modify(Rows::new(1..), Width::truncate(30).suffix("..."));
        table.with(Alignment::left());
        table.modify(Columns::single(0), Alignment::right());
        table.modify(Columns::single(5), Alignment::center());
        let table_str = table.to_string();
        pager.push_str(table_str)?;
    minus::page_all(pager)?;
    Ok(())
}
async fn list(pool: &SqlitePool) -> Result<(), Box<dyn Error>> {
    let pager = minus::Pager::new();
    pager.set_prompt("List trash | press 'q' to exit")?;

    let files = sqlx::query_as!(Database, "SELECT * FROM trash")
        .fetch_all(pool)
        .await?;

    if files.is_empty() {
        pager.push_str("It's empty\n")?;
    } else {
        let view_list: Vec<TrashRow> = files
            .into_iter()
            .map(|row| TrashRow {
                id: row.id,
                name: row.name,
                path: row.original_path,
                archive_tool: row.archive_tool,
                size: format_size(row.size),
                time: row.time,
            })
            .collect();

        let mut table = Table::new(view_list);
        table.with(Style::blank());
        table.modify(Rows::new(1..), Width::truncate(30).suffix("..."));
        table.with(Alignment::left());
        table.modify(Columns::single(0), Alignment::right());
        table.modify(Columns::single(5), Alignment::center());
        let table_str = table.to_string();
        pager.push_str(table_str)?;
    }
    minus::page_all(pager)?;
    Ok(())
}

async fn remove(rubbish: PathBuf, pool: &SqlitePool, cfg: &Config) -> Result<(), Box<dyn Error>> {
    if cfg.disable_list.contains(&rubbish.display().to_string()) {
        eprintln!(
            "Can't remove {} because it's in the disable_list",
            rubbish.display()
        );
        std::process::exit(1);
    }
    let row = compress(&rubbish, cfg)?;
    insert(pool, &row).await?;
    Ok(())
}
