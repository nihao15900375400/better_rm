use anyhow::Result;
use dirs;
use glob::glob;
use std::env;
use std::fs;
use std::os::unix::fs::MetadataExt;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use timeago::Formatter;

/// 获取文件大小
pub fn file_size(path: &Path) -> Result<String> {
    let meta = fs::metadata(path)?;
    let mut bytes = meta.size();
    const UNITS: &[&str] = &["B", "KiB", "MiB", "GiB", "TiB"];
    if bytes == 0 {
        return Ok("0 B".to_string());
    }
    let mut unit_idx = 0;
    while bytes >= 1024 && unit_idx < UNITS.len() - 1 {
        bytes /= 1024;
        unit_idx += 1;
    }
    Ok(format!("{} {}", bytes, UNITS[unit_idx]))
}

/// 递归创建文件
pub fn create_file_all(path: &Path) -> Result<()> {
    if let Some(p) = path.parent() {
        fs::create_dir_all(p)?;
    }
    if !path.exists() {
        fs::File::create(path)?;
    }
    Ok(())
}
/// 将路径展开为绝对路径，不访问文件系统（不验证路径是否存在）。
/// 支持的展开：
/// - `~` 开头 → 替换为 home 目录（使用 `dirs::home_dir`）
/// - 相对路径 → 基于当前工作目录拼接（使用 `std::env::current_dir()`）
/// - 已经是绝对路径 → 原样返回
pub fn to_absolute_no_fs(path: &str) -> PathBuf {
    let path = path.trim();
    if path.starts_with("~") {
        if let Some(home) = dirs::home_dir() {
            if path == "~" {
                return home;
            }
            let rest = &path[1..].trim_start_matches(|c| c == '/' || c == '\\');
            return home.join(rest);
        }
    }

    let p = Path::new(path);

    if p.is_absolute() {
        p.to_path_buf()
    } else {
        let cwd = env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
        cwd.join(p)
    }
}

/// 展开通配符模式（如 "dir/*.rs", "*.json", "~/*.txt"），返回匹配项的绝对路径列表。
/// 使用 `glob` 库进行文件系统匹配，并对结果调用 `canonicalize()` 获得规范绝对路径。
/// 如果模式中包含 `~`，会先用 `to_absolute_no_fs` 将其转换为绝对模式后再匹配。

pub fn glob_absolute(pattern: &str) -> Vec<PathBuf> {
    let absolute_pattern = to_absolute_no_fs(pattern);
    let pattern_str = absolute_pattern.to_string_lossy().to_string();

    match glob(&pattern_str) {
        Ok(paths) => paths
            .filter_map(|entry| entry.ok().and_then(|p| p.canonicalize().ok()))
            .collect(),
        Err(_) => vec![],
    }
}
/// 毫秒时间戳转为英文相对时间：2 days ago / 5 minutes ago
pub fn timestamp_human(ts: i64) -> String {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis() as i64;
    let diff_ms = (now - ts).max(0) as u64;
    let diff = Duration::from_millis(diff_ms);
    let fmt = Formatter::new();
    fmt.convert(diff)
}

/// 获取 N 天之前的毫秒 Unix 时间戳
pub fn days_ago(days: u16) -> i64 {
    const DAY_MS: i64 = 86400000;
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis() as i64;
    now - days as i64 * DAY_MS
}
