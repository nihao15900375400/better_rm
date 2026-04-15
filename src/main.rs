pub mod pack;
pub mod utils;
use termimad::*;
use std::env;
use std::path::{Path, PathBuf};
use std::time::Instant;
use serde::Deserialize;
use std::process::Command;
use colored::*;
use tabled::{settings::Style, Table, Tabled};
use utils::{
    find_config_upwards, 
    delete_result_by_name, 
    ensure_config_and_info, 
    format_file_size,
    get_result_by_name, 
    append_result_to_json, 
    load_conf, 
    print_md, 
    get_all_result, 
    Config,
};
pub use pack::{
    CompressionResult,
    extract_and_delete, 
    compress_and_hash,
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
        Some("-l" | "--list") => list(&info_path,argv),
        Some("--list-setting") => list_setting(),
        Some("-r" | "--restore") => restore(&info_path,argv),
        Some("-d" | "--delete") => delete(),
        Some("-c" | "--clear") => clear(),
        Some("--") => {
            argv.remove(0);
            remove(&info_path,argv,&cfg);
        }
        Some("-v" | "--version") => version(),
        Some("-h" | "--help") => help(),
        None => help(),
        Some("-f" | "--force") => force(),
        Some("--config") => see_config(),
        _ => remove(&info_path,argv,&cfg),
    }
    Ok(())
}

pub fn list_setting() {
    //更好的see config
}
pub fn restore(info_path: &Path, mut argv: Vec<String>) {
    argv.remove(0);
    if argv.is_empty() {
        let md = r#"
# `del` - 安全易用的删除工具

## `restore` - 恢复文件
- `del [ -r|--restore <file> ]`     恢复文件（夹）到原路径

## 说明
- `del -l` 查看文件名
- `del -h` 查看帮助
- `del [ --restore|-r ]` 查看本帮助
"#;
    print_md(&md);
    }else{
        for i in argv{
        let res = match get_result_by_name(info_path,&i){
                Ok(None) => {
                    println!("无此内容{}",i);
                    std::process::exit(1);
                }
                Ok(Some(v)) => v,
                Err(e) => {
                    eprintln!("读取失败：{}", e);
                    std::process::exit(1);
                }
            };
            extract_and_delete(&res);
            delete_result_by_name(info_path,&i);
        }
    }
}
pub fn delete() {}
pub fn clear() {}
pub fn version() {}
pub fn help() {}
pub fn force() {}
pub fn see_config() {let config_path = find_config_upwards("config.json").unwrap();
    ;
    println!("{}",format!("{}",&config_path.display()).green());
    Command::new("cat")
    .arg(config_path)
    .status()
    .expect("执行 cat 失败");}
pub fn remove(info_path: &Path, mut argv: Vec<String>,cfg:&Config) {
    for i in argv{
            &append_result_to_json(&compress_and_hash(&i,&cfg.compression_tool,&cfg.index_tool,&cfg.recycle).unwrap(),info_path);
        }
}
fn list(info_path: &Path, mut argv: Vec<String>) {
    argv.remove(0);

    if argv.is_empty() {
        let lst = match get_all_result(info_path) {
            Ok(v) => v,
            Err(e) => {
                eprintln!("读取失败：{}", e);
                std::process::exit(1);
            }
        };

        if lst.is_empty() {
            println!("{}", "空".blue());
        } else {
            CompressionResult::print_as_table(&lst);
        }
    }else{
        for i in argv{
            let res = match get_result_by_name(info_path,&i){
                Ok(None) => {
                    println!("无此内容{}",i);
                    std::process::exit(1);
                }
                Ok(v) => v.unwrap(),
                Err(e) => {
                    eprintln!("读取失败：{}", e);
                    std::process::exit(1);
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
