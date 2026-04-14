mod pack;
mod utils;
use pack::compress_and_hash;
use std::path::{Path, PathBuf};
use std::time::Instant;
use utils::{
    append_result_to_json, delete_result_by_name, format_file_size, get_result_by_name, load_conf,
};
fn main() {
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
    match load_conf() {
        Ok(cfg) => {
            println!("配置加载成功！");
            println!("版本: {}", cfg.version);
            println!("作者: {}", cfg.author);
            println!("回收站路径: {}", cfg.recycle);
            println!("禁用的路径数量: {}", cfg.disabled_list.len());
        }
        Err(e) => eprintln!("加载配置失败: {}", e),
    };
    println!("{:?}", get_result_by_name(p, "Android_tree.txt"));
    delete_result_by_name(p, "Android_tree.txt");
}
