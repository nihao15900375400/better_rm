/*
Copyright (c) 2026 ywnh1
del is licensed under Mulan PSL v2.
You can use this software according to the terms and conditions of the Mulan
PSL v2.
You may obtain a copy of Mulan PSL v2 at:
         http://license.coscl.org.cn/MulanPSL2
THIS SOFTWARE IS PROVIDED ON AN "AS IS" BASIS, WITHOUT WARRANTIES OF ANY
KIND, EITHER EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED TO
NON-INFRINGEMENT, MERCHANTABILITY OR FIT FOR A PARTICULAR PURPOSE.
See the Mulan PSL v2 for more details.
*/
use chrono::Local;
use std::env;
use std::io::{self, Write};
use std::path::PathBuf;
use which::which;

pub fn now_time() -> String {
    Local::now().format("%Y-%m-%d_%H:%M:%S").to_string()
}

pub fn input(msg: &str) -> Result<String, io::Error> {
    print!("{}", msg);
    io::stdout().flush()?;

    let mut ipt = String::new();
    io::stdin().read_line(&mut ipt)?;

    Ok(ipt.trim().to_string())
}
pub fn to_abs_path(raw: &str) -> PathBuf {
    let mut p = if raw.starts_with('~') {
        let home = env::var_os("HOME").map(PathBuf::from).unwrap_or_default();
        let rest = &raw[1..];
        home.join(rest.strip_prefix('/').unwrap_or(rest))
    } else {
        PathBuf::from(raw)
    };

    if !p.is_absolute() {
        let cwd = env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
        p = cwd.join(p);
    }

    let mut parts = Vec::new();
    for comp in p.components() {
        match comp {
            std::path::Component::CurDir => continue,
            std::path::Component::ParentDir => {
                if !parts.is_empty() && parts.last().unwrap() != &std::path::Component::RootDir {
                    parts.pop();
                }
            }
            _ => parts.push(comp),
        }
    }

    let mut res = PathBuf::new();
    for c in parts {
        res.push(c);
    }
    res
}
pub fn is_nano_installed() -> bool {
    which("nano").is_ok()
}

pub fn format_size(size: i64) -> String {
    const UNIT_BASE: f64 = 1024.0;
    let units = ["B", "KB", "MB", "GB", "TB"];

    let mut size_val = size as f64;
    let mut unit_idx = 0;

    while size_val >= UNIT_BASE && unit_idx < units.len() - 1 {
        size_val /= UNIT_BASE;
        unit_idx += 1;
    }

    if size_val.fract() < 1e-6 {
        format!("{:.0}{}", size_val, units[unit_idx])
    } else {
        format!("{:.1}{}", size_val, units[unit_idx])
    }
}
