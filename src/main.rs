// Copyright (c) 2026 ywnh1
// del is licensed under Mulan PSL v2.
// You can use this software according to the terms and conditions of the Mulan
// PSL v2.
// You may obtain a copy of Mulan PSL v2 at:
//          http://license.coscl.org.cn/MulanPSL2
// THIS SOFTWARE IS PROVIDED ON AN "AS IS" BASIS, WITHOUT WARRANTIES OF ANY
// KIND, EITHER EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED TO
// NON-INFRINGEMENT, MERCHANTABILITY OR FIT FOR A PARTICULAR PURPOSE.
// See the Mulan PSL v2 for more details.
//
mod args;
mod config;
mod consts;
mod pack;
mod sql;
mod util;

use crate::sql::Trash;
use crate::util::*;
use anyhow::{Result, ensure};
use clap::Parser;
use cliclack::confirm;
use rayon::prelude::*;
use rusqlite::Connection;
use std::fs;
use std::io::Write;
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{LazyLock, Mutex};
use std::{collections::HashSet, process::Command};

static STDERR_LOCK: LazyLock<Mutex<std::io::Stderr>> =
    LazyLock::new(|| Mutex::new(std::io::stderr()));
static VERBOSE: AtomicBool = AtomicBool::new(false);
fn set_verbose(enable: bool) {
    VERBOSE.store(enable, Ordering::Relaxed);
}

macro_rules! safe_log {
    ($($arg:tt)*) => {{
        if VERBOSE.load(std::sync::atomic::Ordering::Relaxed) {
            let mut guard = STDERR_LOCK.lock().unwrap();
            let stderr = &mut *guard;
            writeln!(stderr, $($arg)*).unwrap();
            stderr.flush().unwrap();
        }
    }};
}

macro_rules! log {
     ($($arg:tt)*) => {{
        if VERBOSE.load(Ordering::Relaxed) {
             eprintln!($($arg)*);
         }
     }};
 }

fn main() -> Result<()> {
    let cli = args::Cli::parse();
    let config = config::load_config()?;
    let mut conn = sql::connect_database()?;
    set_verbose(cli.verbose);

    if cli.trash {
        ensure!(
            cli.path.is_empty()
                && !cli.force
                && !cli.autoclean
                && !cli.clear
                && !cli.config
                && !cli.recursive,
            "`--trash`|`-t` can't be used with paths or other flags"
        );
    } else if cli.config {
        ensure!(
            cli.path.is_empty()
                && !cli.force
                && !cli.autoclean
                && !cli.trash
                && !cli.clear
                && !cli.recursive,
            "`--config`|`-c` can't be used with paths or other flags"
        );
    } else if cli.autoclean {
        ensure!(
            cli.path.is_empty() && !cli.force && !cli.trash && !cli.config && !cli.recursive,
            "`--autoclean`|`-a` can't be used with paths or other flags"
        );
    } else if cli.clear {
        ensure!(
            cli.path.is_empty() && !cli.force && !cli.trash && !cli.config && !cli.recursive,
            "`--clear`|`-C` can't be used with paths or other flags"
        );
    }

    let path = if !cli.path.is_empty() {
        log!("Checking if the path is allowed");
        let ps = check_allowance(&cli.path, &config.disable_list);
        if ps.is_empty() {
            eprintln!("No path is allowed");
        }
        ps
    } else {
        Vec::new()
    };

    if cli.force && !path.is_empty() {
        ensure!(
            !cli.trash && !cli.config && !cli.autoclean && !cli.clear,
            "Except `--recursive`|`-r`, `--force`|`-f` can't use with other flags"
        );
        let arg = if cli.recursive { "-rf" } else { "-f" };
        let paths = path.join(" ");
        log!("Running `rm {arg} {paths}`");
        let status = Command::new("rm").arg(arg).args(cli.path).status()?;
        ensure!(status.success(), format!("Fail to run `rm {arg} {paths}`"));
        log!("Done");
        return Ok(());
    }

    if cli.trash {
        let result = sql::select_visible(&mut conn)?;
        if result.is_empty() {
            log!("No item to restore.");
        } else {
            log!("Restored {} item(s):", result.len());
            restore(&result, &config.trash_dir)?;
        }
    }

    if cli.config {
        log!("Start editing");
        config::edit_config()?;
        log!("Done");
    }

    if cli.clear {
        if confirm("Delete all baks forever?")
            .initial_value(false)
            .interact()?
        {
            fs::File::create(consts::DB_PATH)?;
        }
    }

    if cli.autoclean {
        autoclean(&mut conn, &config.trash_dir, config.saving_days)?;
        log!("Done");
    }

    if !path.is_empty() {
        delete(
            &mut conn,
            &path,
            Path::new(&config.trash_dir),
            config.compression_level,
        )?;
    }

    Ok(())
}

fn check_allowance(paths: &[String], disable_list: &[String]) -> Vec<String> {
    let mut disable = HashSet::new();
    for i in disable_list {
        for j in glob_absolute(&i) {
            if let Some(p) = j.to_str() {
                disable.insert(to_absolute_no_fs(p));
            }
        }
    }
    paths
        .into_iter()
        .filter_map(|p| {
            let path = to_absolute_no_fs(&p);
            if !path.is_dir() && !path.is_file() {
                eprintln!("No such file or dir: {p}");
                return None;
            }
            if disable.contains(&path) {
                eprintln!(
                    "{} is found in disable list. So it won't be deleted.",
                    path.display()
                );
                None
            } else {
                Some(path.to_string_lossy().to_string())
            }
        })
        .collect()
}

fn delete(conn: &mut Connection, paths: &[String], trash_dir: &Path, level: i32) -> Result<()> {
    fs::create_dir_all(trash_dir)?;
    let infos = paths
        .into_par_iter()
        .map(|p| {
            let path = Path::new(p);
            safe_log!("Packing {p}");
            let res = pack::pack(path, trash_dir, level).unwrap();
            safe_log!("Removing {p}");
            if path.is_file() {
                fs::remove_file(path).unwrap();
            } else if path.is_dir() {
                fs::remove_dir_all(path).unwrap();
            }
            res
        })
        .collect::<Vec<sql::Trash>>();
    sql::insert(conn, &infos)?;
    Ok(())
}

fn restore(rows: &[Trash], trash_dir: &str) -> Result<()> {
    let trash_dir = Path::new(trash_dir);
    rows.into_par_iter().for_each(
        |Trash {
             id: _,
             time: _,
             path,
             hash,
             size: _,
         }| {
            safe_log!("Restoring {path} from {hash}.bak");
            pack::unpack(hash, trash_dir, Path::new(path)).expect("Fail to unpack")
        },
    );
    Ok(())
}

fn autoclean(conn: &mut Connection, trash_dir: &str, saving_days: u16) -> Result<()> {
    log!("Removing expired baks");
    let expired = sql::select_days_age(conn, saving_days)?;
    sql::delete(conn, &expired)?;
    let trash_path = Path::new(trash_dir);
    let mut hashes = Vec::new();
    let mut paths = Vec::new();
    for file in fs::read_dir(trash_path)? {
        let file = file?;
        let path = file.path();
        if path.is_file() {
            if let Some(hash) = path.file_stem() {
                hashes.push(hash.display().to_string());
                paths.push(path);
            }
        }
    }
    let exist = sql::exist_hash(conn, &hashes)?;
    (0..exist.len()).into_par_iter().for_each(|i| {
        if !exist[i] {
            safe_log!("Removing {}.bak", hashes[i]);
            fs::remove_file(&paths[i]).unwrap();
        }
    });
    Ok(())
}
