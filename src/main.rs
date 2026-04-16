// Copyright (c) 2026 ywnh1
//
// del is licensed under Mulan PSL v2.
// You can use this software according to the terms and conditions of the Mulan
// PSL v2. You may obtain a copy of Mulan PSL v2 at:
//         http://license.coscl.org.cn/MulanPSL2
//
// THIS SOFTWARE IS PROVIDED ON AN "AS IS" BASIS, WITHOUT WARRANTIES OF ANY
// KIND, EITHER EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED TO
// NON-INFRINGEMENT, MERCHANTABILITY OR FIT FOR A PARTICULAR PURPOSE.
// See the Mulan PSL v2 for more details.

pub mod help;
pub mod pack;
pub mod utils;
use colored::*;
use help::{help, help_all};
pub use pack::{CompressionResult, compress_and_hash, delete_archive, extract_and_delete};
use serde::Deserialize;
use std::env;
use std::fs::{self, File};
use std::io::{self, Write, stdout};
use std::path::{Path, PathBuf};
use std::process::{self, Command};
use std::time::Instant;
use tabled::{Table, Tabled, settings::Style};
use termimad::*;
use utils::{
    Config, append_result_to_json, delete_result_by_name, ensure_config_and_info,
    find_config_upwards, format_file_size, get_all_result, get_result_by_name, load_conf, print_md,
};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    //test();
    ensure_config_and_info(); // 确保所有依赖都存在
    let cfg = load_conf()?;
    let info_path = Path::new(&cfg.recycle).join(".information.json");
    let mut argv: Vec<String> = std::env::args().skip(1).collect();
    // 获取所有传参

    // 拿到第一个参数，转 &str
    let cmd = argv.first().map(|s| s.as_str());
    match cmd {
        Some("-l" | "--list") => list(&info_path, argv),
        Some("--list-setting") => list_setting(),
        Some("-r" | "--restore") => restore(&info_path, argv),
        Some("-d" | "--delete") => delete(&info_path, argv),
        Some("-c" | "--clear") => clear(&info_path, &cfg),
        Some("--") => {
            argv.remove(0);
            remove(&info_path, argv, &cfg);
        }
        Some("-v" | "--version") => version(&cfg),
        Some("-h" | "--help") => help(argv.get(1).map(|x| x.as_str())),
        None => help_all(),
        Some("-f" | "--force") => force(&info_path, argv, &cfg),
        Some("--config") => see_config(),
        _ => remove(&info_path, argv, &cfg),
    }
    Ok(())
}
pub fn delete(info_path: &Path, mut argv: Vec<String>) {
    argv.remove(0);
    if argv.is_empty() {
        help(Some("delete"));
    } else {
        for i in argv {
            let res = match get_result_by_name(info_path, &i) {
                Ok(None) => {
                    println!("无此内容{}", i);
                    process::exit(1);
                }
                Ok(Some(v)) => v,
                Err(e) => {
                    eprintln!("读取失败：{}", e);
                    process::exit(1);
                }
            };
            delete_archive(&res);
            delete_result_by_name(info_path, &i);
        }
    }
}

pub fn clear(info_path: &Path, cfg: &Config) {
    print!("清空回收站？（y/n）：");
    stdout().flush().unwrap();
    let mut x = String::new();
    io::stdin().read_line(&mut x).expect("读取输入失败");
    if !matches!(x.as_str().trim(), "Y" | "y") {
        process::exit(1);
    }
    match fs::remove_dir_all(cfg.recycle.clone()) {
        Ok(_) => println!("{}", "成功清空".green()),
        Err(e) => eprintln!("{}：{}", "清空失败".red(), e),
    }
    ensure_config_and_info();
}

pub fn version(cfg: &Config) {
    match load_conf() {
        Ok(cfg) => println!("版本: {}", &cfg.version.green()),
        Err(e) => {
            eprintln!("错误: {}", e);
            process::exit(1);
        }
    }
}

pub fn list_setting() {
    match load_conf() {
        Ok(cfg) => {
            println!("版本: {}", cfg.version.green());
            println!("回收站: {}", cfg.recycle.green());
            println!("压缩工具: {}", cfg.compression_tool.green());
            println!("hash/uuid: {}", cfg.index_tool.green());
            print!("\n");
            print!("查看禁删列表？（y/n）：");
            stdout().flush().unwrap();
            let mut x = String::new();
            io::stdin().read_line(&mut x).expect("读取输入失败");
            if matches!(x.as_str().trim(), "Y" | "y") {
                for i in &cfg.disabled_list {
                    println!("{}", i.blue());
                }
            }
        }
        Err(e) => {
            eprintln!("错误: {}", e);
            process::exit(1);
        }
    }
}

pub fn restore(info_path: &Path, mut argv: Vec<String>) {
    argv.remove(0);
    if argv.is_empty() {
        help(Some("restore"));
    } else {
        for i in argv {
            let res = match get_result_by_name(info_path, &i) {
                Ok(None) => {
                    println!("无此内容{}", i);
                    process::exit(1);
                }
                Ok(Some(v)) => v,
                Err(e) => {
                    eprintln!("读取失败：{}", e);
                    process::exit(1);
                }
            };
            extract_and_delete(&res);
            delete_result_by_name(info_path, &i);
        }
    }
}

pub fn force(info_path: &Path, mut argv: Vec<String>, cfg: &Config) {
    argv.remove(0);
    if argv.is_empty() {
        help(Some("force"));
    } else {
        for i in argv {
            if cfg.disabled_list.contains(&i) {
                println!("确定删除 {}？（y/n）：", i.red());
                let mut x = String::new();
                io::stdin().read_line(&mut x).expect("读取输入失败");
                if !matches!(x.as_str().trim(), "Y" | "y") {
                    continue;
                }
            }
            let result =
                compress_and_hash(&i, &cfg.compression_tool, &cfg.index_tool, &cfg.recycle)
                    .unwrap();
            append_result_to_json(&result, info_path);
        }
    }
}

pub fn see_config() {
    let config_path = find_config_upwards("config.json").unwrap();
    println!("{}", format!("{}", &config_path.display()).green());
    Command::new("cat")
        .arg(config_path)
        .status()
        .expect("执行 cat 失败");
}

pub fn remove(info_path: &Path, mut argv: Vec<String>, cfg: &Config) {
    for i in argv {
        if cfg.disabled_list.contains(&i) {
            println!("{}禁止删除", i.red());
            continue;
        }
        &append_result_to_json(
            &compress_and_hash(&i, &cfg.compression_tool, &cfg.index_tool, &cfg.recycle).unwrap(),
            info_path,
        );
    }
}

fn list(info_path: &Path, mut argv: Vec<String>) {
    argv.remove(0);

    if argv.is_empty() {
        let lst = match get_all_result(info_path) {
            Ok(v) => v,
            Err(e) => {
                eprintln!("读取失败：{}", e);
                process::exit(1);
            }
        };

        if lst.is_empty() {
            println!("{}", "空".blue());
        } else {
            CompressionResult::print_as_table(&lst);
        }
    } else {
        for i in argv {
            let res = match get_result_by_name(info_path, &i) {
                Ok(None) => {
                    println!("无此内容{}", i);
                    process::exit(1);
                }
                Ok(v) => v.unwrap(),
                Err(e) => {
                    eprintln!("读取失败：{}", e);
                    process::exit(1);
                }
            };

            res.print();
        }
    }
}

fn test() {
    let a = compress_and_hash(
        "/storage/emulated/0/用户/Android_tree.txt",
        "tzst",
        "blake3",
        "/storage/emulated/0/用户/",
    )
    .unwrap();
    let p = Path::new("/storage/emulated/0/code/rust/del/.information.json");
    append_result_to_json(&a, p);

    //Err(Io { source: Os { code: 2, kind: NotFound, message: "No such file or directory" }, path: Some("/storage/emulated/0/用户/4.9语文_20260410231158.pdf") })
    //Ok(CompressionResult { success: true, error_reason: None, hash_or_uuid: "01ddb490845fd2c6090a563af05c94d9c699a97362d21db9d57a6de5714f3b40", output_path: "/storage/emulated/0/用户/01ddb490845fd2c6090a563af05c94d9c699a97362d21db9d57a6de5714f3b40.tar.zst", compression_datetime: "2026-04-13 15:22:35", compressed_size: 180073, original_path: "/storage/emulated/0/用户/Linux_tree.txt", is_directory: false })
}
