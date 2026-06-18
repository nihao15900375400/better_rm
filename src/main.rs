mod args;
mod pack;
mod sql;

use anyhow::{Result, ensure};
use clap::Parser;
use config::{Config, expand_tilde};
use console::{Color, style};
use dialoguer::{Confirm, Input, MultiSelect, Select, theme::ColorfulTheme};
use multi_select::multi_select;
use pack::*;
use sql::*;
use sqlx::SqlitePool;
use std::fs::File;
use std::path::{Path,PathBuf};
use tokio;

const CONFIG_PATH: &str = "~/.config/del/config.toml";
const DB_PATH: &str = "~/.config/del/trash.db";

#[tokio::main]
async fn main() -> Result<()> {
    let args = args::Args::parse();
    println!("{:#?}", args);
    
    let db_path = expand_tilde(DB_PATH);
    let theme = ColorfulTheme::default();
    let cfg = Config::new(&expand_tilde(CONFIG_PATH))?;
    
    if !Path::new(&db_path).is_file() {
        File::create(&db_path)?;
    }
    
    let pool = SqlitePool::connect_lazy(&format!("sqlite://{}", db_path))?;
    
    if args.interact {
        let welcome = style("欢迎使用 del 交互模式！").fg(Color::Cyan).bold();
        println!("{welcome}");
        let exit = false;
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
                    remove(&pool,&res).await?;
                    continue;
                },
                1 => {
                    let all = select_all(&pool).await?;
                    let res = multi_select(&all)?;
                    restore(&res,&cfg)?;
                    remove(&pool,&res).await?;
                    continue;
                },
                2 => {
                    let new = cfg.set()?;
                    if Confirm::with_theme(&theme)
                        .with_prompt("确定保存?")
                        .default(true)
                        .interact()? {
                        new.save(&expand_tilde(CONFIG_PATH))?;
                    }
                }
                _ => {}
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
    }
    Ok(())
}

fn restore(to_restore:&[TrashRow],cfg:&Config) -> Result<()> {
    let theme: ColorfulTheme  = ColorfulTheme::default();
    let dir = Select::with_theme(&theme)
        .with_prompt("恢复到：")
        .items(
            vec![
                "各自原目录",
                "指定新目录",
                "取消",
            ]
        )
        .interact()?;
    match dir {
        0 => {
            for i in to_restore{
                let mut trash_dir: PathBuf =PathBuf::from(cfg.trash.clone());
                trash_dir.push(
                    format!("{}.bak",i.hash)
                );
                unpack(PathBuf::from(cfg.trash.clone()).to_str().unwrap(),&i.path)?;
            }
        },
        1 => {
            let output_dir = 
                Input::with_theme(&theme)
                    .with_prompt("输出到：")
                    .validate_with(|input: &String |{
                        if Path::new(input).is_dir(){
                            Ok(())
                        }else{
                            Err("无此目录")
                        }
                    })
                    .interact_text()?;
            let mut to_unpack = Vec::new();
            for i in to_restore{
                let mut trash_dir: PathBuf=PathBuf::from(cfg.trash.clone());
                trash_dir.push(
                format!("{}.bak",i.hash)
                
            );
            to_unpack.push(trash_dir.display().to_string());
        }
        unpack_all(&to_unpack,&output_dir)?;
        },
        2 => return Ok(()),
        _ => unreachable!(),
    }
    Ok(())
}

