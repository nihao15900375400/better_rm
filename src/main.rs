mod args;
mod pack;
mod sql;

use anyhow::{Result, ensure};
use chrono::{Utc,Duration};
use clap::Parser;
use config::{Config, expand_tilde};
use console::{Color, style};
use dialoguer::{Confirm, Input, Select, theme::ColorfulTheme};
use multi_select::multi_select;
use pack::*;
use sql::*;
use sqlx::SqlitePool;
use std::fs;
use std::fs::File;
use std::path::{Path, PathBuf};


const CONFIG_PATH: &str = "~/.config/del/config.toml";
const DB_PATH: &str = "~/.config/del/trash.test.db";

#[tokio::main]
async fn main() -> Result<()> {
    let args = args::Args::parse();
    println!("{:#?}", args);

    let db_path = expand_tilde(DB_PATH);
    let theme = ColorfulTheme::default();
    let cfg = Config::new(&expand_tilde(CONFIG_PATH))?;

    let pool = if !Path::new(&db_path).is_file() {
        File::create(&db_path)?;
        let pool = SqlitePool::connect_lazy(&format!("sqlite://{}", db_path))?;
        creat_table(&pool).await?;
        pool
    } else {
        SqlitePool::connect_lazy(&format!("sqlite://{}", db_path))?
    };

    if args.interact {
        let welcome = style("欢迎使用 del 交互模式！").fg(Color::Cyan).bold();
        println!("{welcome}");
        let mut exit = false;
        while !exit {
            let to_set = Select::with_theme(&theme)
                .with_prompt("请选择项目")
                .items(vec![
                    "永久删除某项",
                    "恢复某项",
                    "修改配置文件",
                    "清空Trash",
                    "退出",
                ])
                .interact()?;
            match to_set {
                0 => {
                    let all = select_all(&pool).await?;
                    let res = multi_select(&all)?;
                    remove(&pool, &res).await?;
                    continue;
                }
                1 => {
                    let all = select_all(&pool).await?;
                    let res = multi_select(&all)?;
                    restore(&res, &cfg)?;
                    remove(&pool, &res).await?;
                    continue;
                }
                2 => {
                    let new = cfg.set()?;
                    if Confirm::with_theme(&theme)
                        .with_prompt("确定保存?")
                        .default(true)
                        .interact()?
                    {
                        new.save(&expand_tilde(CONFIG_PATH))?;
                    }
                }
                3 => {
                    if Confirm::with_theme(&theme)
                        .with_prompt("不可恢复，确认？")
                        .default(false)
                        .interact()?
                    {
                        let code = std::process::Command::new("rm")
                            .arg(&expand_tilde(DB_PATH))
                            .status()?;
                        ensure!(
                            code.success(),
                            "执行失败：ExitCode: {}",
                            code.code().unwrap_or(-1)
                        );
                    } else {
                        continue;
                    }
                }
                _ => {
                    exit = true;
                }
            }
        }
        return Ok(());
    }
    if args.force {
        let status: std::process::ExitStatus = if args.recursive {
            std::process::Command::new("rm")
                .arg("-rf")
                .args(args.path)
                .status()?
        } else {
            std::process::Command::new("rm")
                .arg("-f")
                .args(args.path)
                .status()?
        };
        ensure!(
            status.success(),
            "执行失败：ExitCode: {}",
            status.code().unwrap_or(-1)
        );
    } else if args.config {
        let new = cfg.set()?;
        if Confirm::with_theme(&theme)
            .with_prompt("确定保存?")
            .default(true)
            .interact()?
        {
            new.save(&expand_tilde(CONFIG_PATH))?;
        }
    } else if args.clear {
        if Confirm::with_theme(&theme)
            .with_prompt("不可恢复，确认?")
            .default(false)
            .interact()?
        {
            let code = std::process::Command::new("rm")
                .arg(&expand_tilde(DB_PATH))
                .status()?;
            ensure!(
                code.success(),
                "执行失败：ExitCode: {}",
                code.code().unwrap_or(-1)
            );
        }
    } else if args.autoclean {
        autoclean(&pool,&cfg).await?;
    } else {
        if !args.path.is_empty() {
            let to_del:Vec<String> = args.path.iter()
                .map(|x|{
                Path::new(x).canonicalize().unwrap().display().to_string()
            })
            .collect();
            let hashes = pack_all(&to_del,&cfg.trash,cfg.compression_level)?;
            for i in 0..to_del.len(){
                insert(&pool,&to_del[i],&hashes[i]).await?;
            }
        }
    }
    Ok(())
}

fn restore(to_restore: &[TrashRow], cfg: &Config) -> Result<()> {
    let theme: ColorfulTheme = ColorfulTheme::default();
    let dir = Select::with_theme(&theme)
        .with_prompt("恢复到：")
        .items(vec!["各自原目录", "指定新目录", "取消"])
        .interact()?;
    match dir {
        0 => {
            for i in to_restore {
                let mut trash_dir: PathBuf = PathBuf::from(cfg.trash.clone());
                trash_dir.push(format!("{}.bak", i.hash));
                unpack(PathBuf::from(cfg.trash.clone()).to_str().unwrap(), &i.path)?;
            }
        }
        1 => {
            let output_dir = Input::with_theme(&theme)
                .with_prompt("输出到：")
                .validate_with(|input: &String| {
                    if Path::new(input).is_dir() {
                        Ok(())
                    } else {
                        Err("无此目录")
                    }
                })
                .interact_text()?;
            let mut to_unpack = Vec::new();
            for i in to_restore {
                let mut trash_dir: PathBuf = PathBuf::from(cfg.trash.clone());
                trash_dir.push(format!("{}.bak", i.hash));
                to_unpack.push(trash_dir.display().to_string());
            }
            unpack_all(&to_unpack, &output_dir)?;
        }
        2 => return Ok(()),
        _ => unreachable!(),
    }
    Ok(())
}

fn scan_all_files(root: &Path) -> std::io::Result<Vec<PathBuf>> {
    let mut files = Vec::new();
    let mut stack = vec![root.to_path_buf()];

    while let Some(dir) = stack.pop() {
        let entries = fs::read_dir(dir)?;
        for entry in entries {
            let entry = entry?;
            let path = entry.path();
            if path.is_dir() {
                stack.push(path);
            } else if path.extension().and_then(|s| s.to_str()).unwrap_or("none") == "bak"{
                files.push(path);
            }
        }
    }
    Ok(files)
}

async fn autoclean(pool: &SqlitePool, cfg: &Config) -> Result<()> {
    let all = select_all(&pool).await?;
    let mut all_files = scan_all_files(Path::new(&expand_tilde(&cfg.trash)))?;

    let ago = Utc::now() - Duration::days(cfg.save_days.into());
    let mut at_date = Vec::new();
    for i in &all {
        if i.time <= ago.timestamp_millis().into(){
            at_date.push(i.clone());
        }
    }
    remove(&pool,&at_date).await?;

    let all_hash:Vec<String> = all.iter().map(|i| i.hash.clone()).collect();
    all_files.retain(|x| -> bool {
        !all_hash.contains(
            &String::from(x
            .as_path()
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("0")
        )
        )
    } );

    all_files.iter().for_each(|x|{
        fs::remove_file(x).unwrap();
    });

    Ok(())
}
