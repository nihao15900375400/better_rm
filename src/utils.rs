use crate::pack;
use pack::CompressionResult;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::env;
use std::fs;
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};
use termimad::*;
const KB: u64 = 1u64 << 10;
const MB: u64 = 1u64 << 20;
const GB: u64 = 1u64 << 30;
const TB: u64 = 1u64 << 40;

#[derive(Debug, Deserialize)]
pub struct Config {
    pub version: String,
    pub author: String,
    pub compression_tool: String,
    pub index_tool: String,
    pub recycle: String,
    pub options_supported: OptionsSupported,
    pub disabled_list: HashSet<String>,
}

#[derive(Debug, Deserialize)]
pub struct OptionsSupported {
    pub compression_tool: Vec<String>,
    pub index_tool: Vec<String>,
}

/// 加载配置文件，查找策略：
pub fn load_conf() -> Result<Config, Box<dyn std::error::Error>> {
    let config_path = find_config_upwards("config.json")?;
    load_from_path(&config_path)
}

/// 从当前目录开始向上查找指定文件，返回第一个匹配的路径
pub fn ensure_config_and_info() -> Result<(), Box<dyn std::error::Error>> {
    if let Err(_) = find_config_upwards("config.json") {
        let content = r#"{
    "version": "0.1.0",
    "author": "ywnh1",
    "compression_tool": "tar",
    "index_tool": "uuid4",
    "recycle": "/opt/del/.recycle",
    "options_supported": {
        "compression_tool": [
            "tar",
            "tar.gz",
            "tgz",
            "tar.xz",
            "txz",
            "tar.bz2",
            "tbz2",
            "tar.zst",
            "tzst",
            "tar.lz4",
            "tlz4"
        ],
        "index_tool": [
            "blake3",
            "uuid4"
        ]
    },
    "disabled_list": [
        "/usr/lib64",
        "/lib",
        "/usr/lib/libc.so",
        "/etc/pam.d",
        "/usr/lib/libdl.so",
        "/etc/bash.bashrc",
        "/etc/nsswitch.conf",
        "/var/lib",
        "/var/run",
        "/etc/gshadow",
        "/etc/rc.d",
        "/usr/bin/sudo",
        "/etc/crypttab",
        "/etc/fstab",
        "/etc/hosts",
        "/var/log",
        "/etc/resolv.conf",
        "..",
        "/etc/sudoers",
        "/lib64",
        "/run",
        "/boot/grub",
        "/usr/bin/passwd",
        "/sbin/init",
        "/etc/shadow",
        "/lib/ld-linux-aarch64.so.1",
        "/usr/local",
        "/dev/null",
        "/boot/initramfs",
        "/usr/lib",
        "/proc",
        "/lib/ld-linux.so.3",
        "/sbin",
        "/usr",
        "/etc/passwd",
        "/etc/sudoers.d",
        "/etc/hostname",
        "/usr/bin/env",
        "/etc/profile",
        "/bin",
        "/etc/login.defs",
        "/lib64/ld-linux-x86-64.so.2",
        "/sys",
        "/bin/bash",
        "/root/code",
        ".",
        "/boot",
        "/dev/tty",
        "/etc/systemd",
        "/etc/init.d",
        "/boot/vmlinuz",
        "/usr/sbin",
        "/etc/group",
        "/var",
        "/root",
        "/sbin/poweroff",
        "/usr/bin/login",
        "/bin/sh",
        "/etc/profile.d",
        "/usr/lib/libpthread.so",
        "/usr/bin",
        "/boot/efi",
        "/dev/console",
        "/",
        "/etc",
        "/etc/securetty",
        "/dev/zero",
        "/sbin/reboot",
        "/lib/libc.so",
        "/sbin/halt",
        "/bin/su",
        "/lib/libc.so.6",
        "/dev"
    ]
}"#;
        fs::write("config.json", content)?;
        let path = "/opt/del/.recycle";
        if !Path::new(path).exists() {
            fs::create_dir_all(Path::new(path).parent().unwrap())?;
            fs::write(path, "[]")?;
        }
    }
    Ok(())
}

pub fn find_config_upwards(filename: &str) -> Result<PathBuf, Box<dyn std::error::Error>> {
    let mut current_dir = env::current_dir()?;

    loop {
        let candidate = current_dir.join(filename);
        if candidate.exists() {
            return Ok(candidate);
        }

        // 尝试移动到父目录
        match current_dir.parent() {
            Some(parent) => current_dir = parent.to_path_buf(),
            None => break, // 已到达根目录
        }
    }

    Err(format!(
        "未找到配置文件 '{}'，已从当前目录向上搜索至根目录",
        filename
    )
    .into())
}

/// 从给定路径读取并解析 JSON 配置
fn load_from_path(path: &Path) -> Result<Config, Box<dyn std::error::Error>> {
    let content = fs::read_to_string(path)
        .map_err(|e| format!("读取配置文件失败 '{}': {}", path.display(), e))?;
    let config: Config = serde_json::from_str(&content)
        .map_err(|e| format!("解析配置文件失败 '{}': {}", path.display(), e))?;
    Ok(config)
}
pub fn format_file_size(size: &u64) -> String {
    let size = *size;
    if size < KB {
        format!("{}B", size)
    } else if size < MB {
        format!("{:.2}KB", size as f64 / KB as f64)
    } else if size < GB {
        format!("{:.2}MB", size as f64 / MB as f64)
    } else if size < TB {
        format!("{:.2}GB", size as f64 / GB as f64)
    } else {
        format!("{:.2}TB", size as f64 / TB as f64)
    }
}

/// 将 CompressionResult 追加到指定 JSON 文件的数组中
/// - 如果文件不存在，创建新文件并写入包含该条目的数组
/// - 如果文件存在但内容为空/无效，视为空数组并追加
pub fn append_result_to_json(
    result: &CompressionResult,
    json_path: &Path,
) -> Result<(), io::Error> {
    // 1. 读取现有数组（若文件存在且有效）
    let mut results: Vec<CompressionResult> = if json_path.exists() {
        let mut file = fs::File::open(json_path)?;
        let mut content = String::new();
        file.read_to_string(&mut content)?;

        // 如果文件为空，直接视为空数组
        if content.trim().is_empty() {
            Vec::new()
        } else {
            // 尝试解析为 JSON 数组
            serde_json::from_str(&content).unwrap_or_else(|_| Vec::new())
        }
    } else {
        Vec::new()
    };

    // 2. 追加新记录
    results.push(result.clone()); // 若不需要 clone，可改为所有权转移，但此处借用传递方便

    // 3. 写回文件（覆盖）
    let json_str = serde_json::to_string_pretty(&results)
        .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;
    fs::write(json_path, json_str)?;

    Ok(())
}

pub fn print_md(md: &str) {
    // r#""#
    let skin = MadSkin::default();
    skin.print_text(md);
}
/// 根据 name 查询对应的 CompressionResult
/// - 返回 `Ok(Some(result))` 表示找到
/// - 返回 `Ok(None)` 表示未找到
/// - 返回 `Err` 表示文件读取或解析错误
pub fn get_result_by_name(
    json_path: &Path,
    name: &str,
) -> Result<Option<CompressionResult>, io::Error> {
    if !json_path.exists() {
        return Ok(None);
    }

    let content = fs::read_to_string(json_path)?;
    if content.trim().is_empty() {
        return Ok(None);
    }

    let results: Vec<CompressionResult> = serde_json::from_str(&content)
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;

    Ok(results.into_iter().find(|r| r.name == name))
}

/// 根据 name 删除对应的 CompressionResult 条目
/// - 返回 `Ok(true)` 表示找到并删除
/// - 返回 `Ok(false)` 表示未找到（文件不存在或没有匹配项）
/// - 返回 `Err` 表示读写或解析错误
pub fn delete_result_by_name(json_path: &Path, name: &str) -> Result<bool, io::Error> {
    if !json_path.exists() {
        return Ok(false);
    }

    // 读取文件内容
    let content = fs::read_to_string(json_path)?;
    if content.trim().is_empty() {
        return Ok(false);
    }

    // 解析为数组
    let mut results: Vec<CompressionResult> = serde_json::from_str(&content)
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;

    // 记录删除前的长度
    let original_len = results.len();
    // 保留 name 不匹配的条目
    results.retain(|r| r.name != name);

    // 如果没有变化，说明未找到匹配项
    if results.len() == original_len {
        return Ok(false);
    }

    // 将更新后的数组写回文件
    let json_str = serde_json::to_string_pretty(&results)
        .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;
    fs::write(json_path, json_str)?;

    Ok(true)
}
pub fn get_all_result(json_path: &Path) -> Result<Vec<CompressionResult>, io::Error> {
    if !json_path.exists() {
        return Ok(Vec::new());
    }

    let content = fs::read_to_string(json_path)?;
    if content.trim().is_empty() {
        return Ok(Vec::new());
    }

    let results: Vec<CompressionResult> = serde_json::from_str(&content)
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;

    Ok(results)
}
