// Copyright (c) 2026 ywnh1
// del is licensed under Mulan PSL v2.
// You can use this software according to the terms and conditions of the Mulan PSL v2.
// You may obtain a copy of Mulan PSL v2 at:
//          http://license.coscl.org.cn/MulanPSL2
// THIS SOFTWARE IS PROVIDED ON AN "AS IS" BASIS, WITHOUT WARRANTIES OF ANY KIND,
// EITHER EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED TO NON-INFRINGEMENT,
// MERCHANTABILITY OR FIT FOR A PARTICULAR PURPOSE.
// See the Mulan PSL v2 for more details.

mod args;
mod pack;
mod sql;

use anyhow::{Context, Result, ensure};
use chrono::{Duration, Utc};
use clap::Parser;
use config::{Config, expand_tilde};
use console::{Color, style};
use dialoguer::{Confirm, Input, Select, theme::ColorfulTheme};
use multi_select::multi_select;
use pack::*;
use sql::*;
use sqlx::SqlitePool;
use glob::Pattern;
use std::fs;
use std::fs::File;
use std::path::{Component, Path, PathBuf};

const CONFIG_PATH: &str = "~/.config/del/config.toml";
const DB_PATH: &str = "~/.config/del/trash.db";

/// 检查传入路径是否命中禁删名单。命中则 panic，中止全部操作。
///
/// 比较规则：
/// 1. 直接字符串比较
/// 2. 展开 `~` 后比较
/// 3. 解析成绝对路径并清理 `.`、`..` 后比较
/// 4. 若禁删名单条目包含通配符 `*`，使用 glob 模式匹配
fn check_disable_list(paths: &[String], disable_list: &[String]) {
    for path in paths {
        let expanded_path = expand_tilde(path);
        let abs_path = normalize_abs_path(&expanded_path);

        for disabled in disable_list {
            // 1. 直接字符串比较
            if path == disabled {
                panic!(
                    "禁止删除路径 \"{}\"（匹配禁删名单条目 \"{}\"），操作已中止，所有文件均未删除。",
                    path, disabled
                );
            }

            // 2. 展开 ~ 后比较
            let expanded_disabled = expand_tilde(disabled);
            if expanded_path == expanded_disabled {
                panic!(
                    "禁止删除路径 \"{}\"（展开后匹配禁删名单条目 \"{}\"），操作已中止，所有文件均未删除。",
                    path, disabled
                );
            }

            // 3. 转绝对路径 + 清理 . 和 .. 后比较
            let abs_disabled = normalize_abs_path(&expanded_disabled);
            if abs_path == abs_disabled {
                panic!(
                    "禁止删除路径 \"{}\"（绝对路径匹配禁删名单条目 \"{}\"），操作已中止，所有文件均未删除。",
                    path, disabled
                );
            }

            // 4. glob 模式匹配（仅当禁删条目包含 * 通配符时）
            if disabled.contains('*') {
                if let Ok(pattern) = Pattern::new(&abs_disabled) {
                    if pattern.matches(&abs_path) {
                        panic!(
                            "禁止删除路径 \"{}\"（通配符匹配禁删名单条目 \"{}\"），操作已中止，所有文件均未删除。",
                            path, disabled
                        );
                    }
                }
            }
        }
    }
}

/// 将路径展开为绝对路径并规范化（消除 `.` 和 `..` 组件）。
///
/// 不要求路径实际存在（与 `canonicalize` 不同）。
fn normalize_abs_path(path: &str) -> String {
    let p = Path::new(path);
    let abs = if p.is_absolute() {
        p.to_path_buf()
    } else {
        std::env::current_dir()
            .unwrap_or_else(|_| PathBuf::from("/"))
            .join(p)
    };

    // 清理 . 和 .. 组件
    let mut components: Vec<Component> = Vec::new();
    for comp in abs.components() {
        match comp {
            Component::CurDir => {} // 跳过 .
            Component::ParentDir => {
                // 弹出前一个普通组件（根目录不可弹）
                match components.last() {
                    Some(&Component::Normal(_)) | Some(&Component::ParentDir) => {
                        components.pop();
                    }
                    Some(&Component::RootDir) => {} // /.. = /
                    None => {
                        // 无法回退，保留 ..
                        components.push(Component::ParentDir);
                    }
                    _ => {} // 保留 ..
                }
            }
            c => components.push(c),
        }
    }

    let normalized: PathBuf = components.iter().collect();
    // 确保空路径变为当前目录
    if normalized.as_os_str().is_empty() {
        String::from(".")
    } else {
        normalized.to_string_lossy().to_string()
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = args::Args::parse();
    // println!("{:#?}", args);

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
                            .arg(expand_tilde(DB_PATH))
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
        if !args.path.is_empty() {
            check_disable_list(&args.path, &cfg.disable_list);
        }
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
                .arg(expand_tilde(DB_PATH))
                .status()?;
            ensure!(
                code.success(),
                "执行失败：ExitCode: {}",
                code.code().unwrap_or(-1)
            );
        }
    } else if args.autoclean {
        autoclean(&pool, &cfg).await?;
    } else {
        if !args.path.is_empty() {
            check_disable_list(&args.path, &cfg.disable_list);
            let to_del: Vec<String> = args
                .path
                .iter()
                .map(|x| Path::new(x).canonicalize().unwrap().display().to_string())
                .collect();
            let results = pack_all(&to_del, &cfg.trash, cfg.compression_level)?;
            for i in 0..to_del.len() {
                let (hash, size) = &results[i];
                insert(&pool, &to_del[i], hash, size).await?;
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
                let mut archive_path: PathBuf = PathBuf::from(cfg.trash.clone());
                archive_path.push(format!("{}.bak", i.hash));

                let original_path = Path::new(&i.path);
                let parent = original_path
                    .parent()
                    .with_context(|| format!("无法获取 {} 的父目录", i.path))?;
                fs::create_dir_all(parent)?;
                unpack(archive_path.to_str().unwrap(), parent.to_str().unwrap())?;
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
            } else if path.extension().and_then(|s| s.to_str()).unwrap_or("none") == "bak" {
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
        if i.time <= ago.timestamp_millis().into() {
            at_date.push(i.clone());
        }
    }
    remove(&pool, &at_date).await?;

    let all_hash: Vec<String> = all.iter().map(|i| i.hash.clone()).collect();
    all_files.retain(|x| -> bool {
        !all_hash.contains(&String::from(
            x.as_path()
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("0"),
        ))
    });

    all_files.iter().for_each(|x| {
        fs::remove_file(x).unwrap();
    });

    Ok(())
}
