use anyhow::{Context, Result, bail};
use dialoguer::{Confirm, theme::ColorfulTheme};
use std::fs;
use std::fs::File;
use std::io::Write;
use std::path::Path;
use tar::Archive;
use tar::Builder;
use zstd::stream::Decoder;
use zstd::stream::write::Encoder;

/// 同时写入内部 writer 并计算 BLAKE3 哈希的包装器。
struct HashWriter<W: Write> {
    inner: W,
    hasher: blake3::Hasher,
}

impl<W: Write> HashWriter<W> {
    fn new(inner: W) -> Self {
        Self {
            inner,
            hasher: blake3::Hasher::new(),
        }
    }

    /// 完成写入，返回 BLAKE3 哈希值（消耗内部的 hasher）。
    fn finalize(&mut self) -> blake3::Hash {
        // blake3::Hasher::finalize 消耗 self，这里用 replace 取出所有权
        std::mem::replace(&mut self.hasher, blake3::Hasher::new()).finalize()
    }
}

impl<W: Write> Write for HashWriter<W> {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        let n = self.inner.write(buf)?;
        self.hasher.update(&buf[..n]);
        Ok(n)
    }

    fn flush(&mut self) -> std::io::Result<()> {
        self.inner.flush()
    }
}

/// 将文件或目录打包为 tar.zstd，删除源，并返回压缩后文件的 BLAKE3 哈希值。
///
/// 不论 `source` 是文件还是目录，统一打包成 tar.zstd 格式。
/// 压缩成功后**会删除源文件**（`remove_file`）或源目录（`remove_dir_all`）。
///
/// # 参数
/// - `source`: 源路径（文件或目录）
/// - `output`: 输出的 tar.zstd 文件路径
/// - `level`: zstd 压缩级别（1-22，0 使用默认值）
///
/// # 返回
/// BLAKE3 哈希值的十六进制字符串
pub fn pack(source: &str, output: &str, level: i32) -> Result<String> {
    // 先确认源路径存在且类型正确
    let source_path = Path::new(source);
    if !source_path
        .try_exists()
        .with_context(|| format!("无法访问源路径: {source}"))?
    {
        bail!("源路径不存在: {source}");
    }

    let is_dir = source_path.is_dir();
    let is_file = source_path.is_file();

    if !is_dir && !is_file {
        bail!("源路径不是普通文件或目录: {source}");
    }

    // 创建输出文件并构建压缩管道
    let file =
        File::create(Path::new(output)).with_context(|| format!("无法创建输出文件: {output}"))?;
    let mut hash_writer = HashWriter::new(file);
    let enc = Encoder::new(hash_writer, level).with_context(|| "无法创建 zstd 编码器")?;
    let mut builder = Builder::new(enc);

    // 根据类型打包
    if is_dir {
        let dir_name = source_path
            .file_name()
            .with_context(|| format!("无法获取目录名: {source}"))?;
        builder
            .append_dir_all(dir_name, source)
            .with_context(|| format!("无法打包目录: {source}"))?;
    } else {
        let file_name = source_path
            .file_name()
            .with_context(|| format!("无法获取文件名: {source}"))?;
        builder
            .append_path_with_name(source, file_name)
            .with_context(|| format!("无法打包文件: {source}"))?;
    }

    // 完成 tar 打包
    builder.finish().with_context(|| "无法完成 tar 打包")?;

    // 完成 zstd 压缩，收回 HashWriter
    let enc = builder.into_inner().with_context(|| "无法获取内部编码器")?;
    hash_writer = enc.finish().with_context(|| "无法完成 zstd 压缩")?;
    hash_writer.flush()?;

    // 压缩成功，删除源文件或目录
    // 重新判断类型以防止打包过程中文件系统状态变化
    if source_path.is_dir() {
        fs::remove_dir_all(source).with_context(|| format!("无法删除源目录: {source}"))?;
    } else {
        fs::remove_file(source).with_context(|| format!("无法删除源文件: {source}"))?;
    }

    let hash = hash_writer.finalize();
    Ok(hash.to_string())
}

/// 解压 tar.zstd 归档到目标目录。
///
/// 与 `pack` 配对使用：`pack` 统一输出 tar.zstd，此函数统一解压。
/// **不会删除**输入的压缩包文件，解压后原 `.tar.zst` 文件保留不变。
///
/// # 参数
/// - `input`: 输入的 tar.zstd 归档文件路径
/// - `output`: 解压目标目录
pub fn unpack(input: &str, output: &str) -> Result<()> {
    let input_path = Path::new(input);
    if !input_path
        .try_exists()
        .with_context(|| format!("无法访问压缩包: {input}"))?
    {
        bail!("压缩包文件不存在: {input}");
    }

    // 安全检查：output 不能是已存在的普通文件（必须是目录或不存在）
    let output_path = Path::new(output);
    if output_path.try_exists()? && !output_path.is_dir() {
        bail!(
            "解压目标路径已存在且不是目录: {output}\n\
             提示：解压目标必须是一个文件夹（目录），不能是文件。"
        );
    }

    let compressed_file =
        File::open(input_path).with_context(|| format!("无法打开压缩包: {input}"))?;

    let decoder = Decoder::new(compressed_file)
        .with_context(|| format!("无法创建 zstd 解码器，文件可能已损坏: {input}"))?;

    let mut tar_archive = Archive::new(decoder);
    tar_archive
        .unpack(output)
        .with_context(|| format!("无法解压到目标目录: {output}"))?;

    // 注意：不删除输入的压缩包文件，保留原文件
    Ok(())
}

/// 将字节数转换为人类可读的格式（B / KB / MB / GB / TB）。
fn human_readable_size(bytes: u64) -> String {
    const UNITS: &[&str] = &["B", "KB", "MB", "GB", "TB"];
    let mut size = bytes as f64;
    let mut unit_idx = 0;
    while size >= 1024.0 && unit_idx < UNITS.len() - 1 {
        size /= 1024.0;
        unit_idx += 1;
    }
    if unit_idx == 0 {
        format!("{} {}", bytes, UNITS[unit_idx])
    } else {
        format!("{:.2} {}", size, UNITS[unit_idx])
    }
}

/// 获取文件名的"核心名"：去掉 `.tar.zst`、`.zst`、`.tar`、`.bak` 等常见后缀。
fn stem_name(path: &Path) -> Option<String> {
    let name = path.file_name()?.to_string_lossy().to_string();
    // 按优先级从长到短匹配后缀
    for ext in &[".tar.zst", ".tar.zstd", ".bak", ".zst", ".tar"] {
        if let Some(stripped) = name.strip_suffix(ext) {
            return Some(stripped.to_string());
        }
    }
    Some(name)
}

/// 多线程打包多个文件/目录到输出目录。
///
/// 使用固定数量（= CPU 逻辑核心数）的工作线程池，每个线程完成当前任务后
/// 自动从列表中取下一条目，避免线程过多导致上下文切换开销。
///
/// 输出文件统一命名为 `{BLAKE3哈希}.bak`，若已有同名文件则自动覆盖。
/// 压缩失败时自动清理临时文件。
///
/// # 参数
/// - `sources`: 源路径列表（文件或目录）
/// - `output_dir`: 输出目录
/// - `level`: zstd 压缩级别（1-22，0 使用默认值）
///
/// # 错误
/// 任意一个线程失败会收集所有错误，打包成一个 `anyhow::Error` 返回。
/// 多线程打包多个文件/目录到输出目录，返回每个文件的 (BLAKE3 哈希, 人类可读大小) 元组。
pub fn pack_all(sources: &[String], output_dir: &str, level: i32) -> Result<Vec<(String, String)>> {
    let output_path = Path::new(output_dir);
    if !output_path.try_exists()? {
        fs::create_dir_all(output_path)
            .with_context(|| format!("无法创建输出目录: {output_dir}"))?;
    }

    let n = sources.len();
    if n == 0 {
        return Ok(Vec::new());
    }

    // 工作线程数 = min(任务数, CPU 逻辑核心数)
    let num_workers = std::cmp::min(
        n,
        std::thread::available_parallelism()
            .map(|x| x.get())
            .unwrap_or(4),
    );

    let next_idx = std::sync::atomic::AtomicUsize::new(0);
    let errors = std::sync::Mutex::new(Vec::new());

    let results = std::sync::Mutex::new(Vec::<(usize, String, String)>::new());

    // scope 使多个线程可以安全借用外部的局部变量
    std::thread::scope(|s| {
        for _ in 0..num_workers {
            s.spawn(|| {
                loop {
                    let idx = next_idx.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                    if idx >= n {
                        break;
                    }

                    let source = &sources[idx];
                    let source_path = Path::new(source);
                    let file_name = match source_path.file_name() {
                        Some(f) => f.to_string_lossy().to_string(),
                        None => {
                            errors
                                .lock()
                                .unwrap()
                                .push(anyhow::anyhow!("无法获取文件名: {source}"));
                            continue;
                        }
                    };

                    let temp_output =
                        Path::new(output_dir).join(format!(".tmp_{file_name}.tar.zst"));
                    let temp_str = temp_output.to_string_lossy().to_string();

                    match pack(source, &temp_str, level) {
                        Ok(hash) => {
                            let final_output = Path::new(output_dir).join(format!("{hash}.bak"));
                            let r = (|| -> Result<()> {
                                if final_output.exists() {
                                    fs::remove_file(&final_output)?;
                                }
                                fs::rename(&temp_output, &final_output)?;
                                Ok(())
                            })();
                            if let Err(e) = r {
                                let _ = fs::remove_file(&temp_output);
                                errors
                                    .lock()
                                    .unwrap()
                                    .push(
                                        e.context(format!("重命名临时文件到 {hash}.bak 失败")),
                                    );
                            } else {
                                let size = fs::metadata(&final_output)
                                    .map(|m| human_readable_size(m.len()))
                                    .unwrap_or_else(|_| "未知".to_string());
                                results.lock().unwrap().push((idx, hash, size));
                            }
                        }
                        Err(e) => {
                            let _ = fs::remove_file(&temp_output);
                            errors.lock().unwrap().push(e);
                        }
                    }
                }
            });
        }
    });

    let errors = errors.into_inner().unwrap();
    if errors.is_empty() {
        let mut raw = results.into_inner().unwrap();
        raw.sort_by_key(|(idx, _, _)| *idx);
        let pairs: Vec<(String, String)> = raw.into_iter().map(|(_, h, s)| (h, s)).collect();
        Ok(pairs)
    } else {
        let mut msg = String::from("以下任务压缩失败：\n");
        for e in &errors {
            msg.push_str(&format!("  - {e}\n"));
        }
        bail!("{}", msg.trim())
    }
}

/// 多线程解压多个 tar.zstd 归档到输出目录。
///
/// 使用固定数量（= CPU 逻辑核心数）的工作线程池，每个线程完成当前任务后
/// 自动从列表中取下一条目。
///
/// 解压前会扫描所有目标路径，若存在冲突则通过 `dialoguer::Confirm` 向用户询问是否覆盖。
/// **用户确认后**，已存在的路径会被删除再解压。
///
/// # 参数
/// - `inputs`: 输入压缩包路径列表
/// - `output_dir`: 解压目标目录
///
/// # 错误
/// - 用户拒绝覆盖时返回错误
/// - 任意线程失败会收集所有错误合并返回
pub fn unpack_all(inputs: &[String], output_dir: &str) -> Result<()> {
    let output_path = Path::new(output_dir);
    if !output_path.try_exists()? {
        fs::create_dir_all(output_path)
            .with_context(|| format!("无法创建输出目录: {output_dir}"))?;
    }

    // ---------- 第一步（主线程）：收集冲突，交互确认 ----------
    let conflicts: Vec<String> = inputs
        .iter()
        .filter_map(|input| {
            let p = Path::new(input);
            let stem = stem_name(p)?;
            let dest = output_path.join(&stem);
            dest.exists().then(|| dest.to_string_lossy().to_string())
        })
        .collect();

    if !conflicts.is_empty() {
        println!("以下解压目标路径已存在：");
        for dest in &conflicts {
            println!("  - {dest}");
        }
        let theme = ColorfulTheme::default();
        let proceed = Confirm::with_theme(&theme)
            .with_prompt("是否覆盖这些路径？")
            .default(false)
            .interact()?;
        if !proceed {
            bail!("操作已取消");
        }
    }

    // ---------- 第二步：工作池并行解压 ----------
    let n = inputs.len();
    if n == 0 {
        return Ok(());
    }

    let num_workers = std::cmp::min(
        n,
        std::thread::available_parallelism()
            .map(|x| x.get())
            .unwrap_or(4),
    );

    let next_idx = std::sync::atomic::AtomicUsize::new(0);
    let errors = std::sync::Mutex::new(Vec::new());

    std::thread::scope(|s| {
        for _ in 0..num_workers {
            s.spawn(|| {
                loop {
                    let idx = next_idx.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                    if idx >= n {
                        break;
                    }

                    let input = &inputs[idx];
                    let p = Path::new(input);
                    let stem = match stem_name(p) {
                        Some(s) => s,
                        None => {
                            errors
                                .lock()
                                .unwrap()
                                .push(anyhow::anyhow!("无法解压 {input}：无法提取文件名"));
                            continue;
                        }
                    };

                    let dest = output_path.join(&stem);
                    let dest_str = dest.to_string_lossy().to_string();

                    // 因用户已确认覆盖，删除已存在的路径
                    if dest.exists() {
                        let rm = if dest.is_dir() {
                            fs::remove_dir_all(&dest)
                        } else {
                            fs::remove_file(&dest)
                        };
                        if let Err(e) = rm {
                            errors
                                .lock()
                                .unwrap()
                                .push(anyhow::anyhow!("无法移除已存在的路径 {dest_str}: {e}"));
                            continue;
                        }
                    }

                    if let Err(e) = unpack(input, &dest_str) {
                        errors.lock().unwrap().push(e);
                    }
                }
            });
        }
    });

    let errors = errors.into_inner().unwrap();
    if errors.is_empty() {
        Ok(())
    } else {
        let mut msg = String::from("以下任务解压失败：\n");
        for e in &errors {
            msg.push_str(&format!("  - {e}\n"));
        }
        bail!("{}", msg.trim())
    }
}
