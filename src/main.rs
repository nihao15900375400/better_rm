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
use clap::{CommandFactory, FromArgMatches};
use cliclack::confirm;
use rayon::prelude::*;
use rusqlite::Connection;
use rust_i18n::{i18n, t};
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

i18n!("locales", fallback = "en");

/// Return a `&'static str` translation for the given i18n key.
/// Caches each key in a LazyLock so the translation is looked up only once.
#[macro_export]
macro_rules! t_str {
    ($key:expr) => {{
        use std::sync::LazyLock;
        static CACHE: LazyLock<String> = LazyLock::new(|| ::rust_i18n::t!($key).into_owned());
        CACHE.as_str()
    }};
}

/// Parse locale from LANG environment variable (e.g. `zh_CN.UTF-8` → `zh-CN`).
fn init_locale() {
    let locale = if let Ok(l) = std::env::var("RUST_I18N_LOCALE") {
        if !l.is_empty() { Some(l) } else { None }
    } else if let Ok(lang) = std::env::var("LANG") {
        let lang = lang.split('.').next().unwrap_or(&lang);
        let l = lang.replace('_', "-");
        if !l.is_empty() && l != "C" && l != "POSIX" {
            Some(l)
        } else {
            None
        }
    } else {
        None
    };
    if let Some(ref l) = locale {
        rust_i18n::set_locale(l);
    }
}

fn main() -> Result<()> {
    init_locale();

    // Build CLI with i18n
    let cmd = args::apply_i18n(args::Cli::command());
    let matches = cmd.get_matches();
    let mut cli = args::Cli::from_arg_matches(&matches)?;

    let mut config = config::load_config()?;
    let mut conn = sql::connect_database()?;

    set_verbose(cli.verbose);

    if let Some(dir) = cli.trash_dir {
        config.trash_dir = dir;
    }
    if let Some(days) = cli.saving_days {
        config.saving_days = days;
    }
    if let Some(level) = cli.compression_level {
        config.compression_level = level;
    }
    if !cli.add_disable.is_empty() {
        config.disable_list.append(&mut cli.add_disable);
    }

    if cli.trash {
        ensure!(
            cli.path.is_empty()
                && !cli.force
                && !cli.autoclean
                && !cli.clear
                && !cli.config
                && !cli.recursive
                && !cli.save,
            "{}",
            t!("error.trash_with_path")
        );
    } else if cli.config {
        ensure!(
            cli.path.is_empty()
                && !cli.force
                && !cli.autoclean
                && !cli.trash
                && !cli.clear
                && !cli.recursive
                && !cli.save,
            "{}",
            t!("error.config_with_path")
        );
    } else if cli.autoclean {
        ensure!(
            cli.path.is_empty()
                && !cli.force
                && !cli.trash
                && !cli.config
                && !cli.recursive
                && !cli.save,
            "{}",
            t!("error.autoclean_with_path")
        );
    } else if cli.clear {
        ensure!(
            cli.path.is_empty()
                && !cli.force
                && !cli.trash
                && !cli.config
                && !cli.recursive
                && !cli.save,
            "{}",
            t!("error.clear_with_path")
        );
    }

    let path = if !cli.path.is_empty() {
        log!("{}", t!("message.checking"));
        let ps = check_allowance(&cli.path, &config.disable_list);
        if ps.is_empty() {
            eprintln!("{}", t!("message.no_path_allowed"));
        }
        ps
    } else {
        Vec::new()
    };

    if cli.force && !path.is_empty() {
        ensure!(
            !cli.trash && !cli.config && !cli.autoclean && !cli.clear && !cli.save,
            "{}",
            t!("error.force_with_flags")
        );
        let arg = if cli.recursive { "-rf" } else { "-f" };
        let paths = path.join(" ");
        log!("{}", t!("message.done"));
        let status = Command::new("rm").arg(arg).args(cli.path).status()?;
        ensure!(
            status.success(),
            "{}",
            t!("error.rm_failed", arg = arg, paths = paths)
        );
        log!("{}", t!("message.done"));
        return Ok(());
    }

    if cli.trash {
        let result = sql::select_visible(&mut conn)?;
        if result.is_empty() {
            log!("{}", t!("message.no_item"));
        } else {
            log!("{}", t!("message.restored_items", count = result.len()));
            restore(&result, &config.trash_dir)?;
        }
    }

    if cli.config {
        log!("{}", t!("message.editing"));
        config::edit_config()?;
        log!("{}", t!("message.done"));
    }

    if cli.clear {
        if confirm(t!("message.del_baks"))
            .initial_value(false)
            .interact()?
        {
            fs::File::create(consts::DB_PATH)?;
        }
    }

    if cli.autoclean {
        autoclean(&mut conn, &config.trash_dir, config.saving_days)?;
        log!("{}", t!("message.done"));
    }

    if !path.is_empty() {
        delete(
            &mut conn,
            &path,
            Path::new(&config.trash_dir),
            config.compression_level,
            cli.save,
        )?;
    }

    Ok(())
}

fn check_allowance(paths: &[String], disable_list: &[String]) -> Vec<String> {
    let mut disable = HashSet::new();
    for i in disable_list {
        for j in glob_absolute(i) {
            if let Some(p) = j.to_str() {
                disable.insert(to_absolute_no_fs(p));
            }
        }
    }
    paths
        .iter()
        .filter_map(|p| {
            let path = to_absolute_no_fs(p);
            if !path.is_dir() && !path.is_file() {
                eprintln!("{}", t!("error.no_such_path", p = p));
                return None;
            }
            if disable.contains(&path) {
                eprintln!(
                    "{}",
                    t!("error.path_disabled", path = path.display().to_string())
                );
                None
            } else {
                Some(path.to_string_lossy().to_string())
            }
        })
        .collect()
}

fn delete(
    conn: &mut Connection,
    paths: &[String],
    trash_dir: &Path,
    level: i32,
    save: bool,
) -> Result<()> {
    fs::create_dir_all(trash_dir)?;
    let infos = paths
        .par_iter()
        .map(|p| {
            let path = Path::new(p);
            safe_log!("{}", t!("message.packing", path = p));
            let res = pack::pack(path, trash_dir, level).unwrap();
            if save {
                safe_log!("{p} won't be delete");
            } else {
                safe_log!("{}", t!("message.removing", path = p));
                if path.is_file() {
                    fs::remove_file(path).unwrap();
                } else if path.is_dir() {
                    fs::remove_dir_all(path).unwrap();
                }
            }
            res
        })
        .collect::<Vec<sql::Trash>>();
    sql::insert(conn, &infos)?;
    Ok(())
}

fn restore(rows: &[Trash], trash_dir: &str) -> Result<()> {
    let trash_dir = Path::new(trash_dir);
    rows.par_iter().for_each(
        |Trash {
             id: _,
             time: _,
             path,
             hash,
             size: _,
         }| {
            safe_log!("{}", t!("message.restoring", path = path, hash = hash));
            pack::unpack(hash, trash_dir, Path::new(path)).expect("Fail to unpack")
        },
    );
    Ok(())
}

fn autoclean(conn: &mut Connection, trash_dir: &str, saving_days: u16) -> Result<()> {
    log!("{}", t!("message.cleaning"));
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
            safe_log!("{}", t!("message.removing_hash", hash = &hashes[i]));
            fs::remove_file(&paths[i]).unwrap();
        }
    });
    Ok(())
}
