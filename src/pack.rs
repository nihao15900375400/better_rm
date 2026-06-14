use anyhow::{bail, Context, Result};
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

    fn finalize(&mut self) -> blake3::Hash {
        self.hasher.finalize()
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
/// 压缩成功后会删除源文件（`remove_file`）或源目录（`remove_dir_all`）。
///
/// # 参数
/// - `source`: 源路径（文件或目录）
/// - `output`: 输出的 tar.zstd 文件路径
/// - `level`: zstd 压缩级别
///
/// # 返回
/// BLAKE3 哈希值的十六进制字符串
pub fn pack(source: &str, output: &str, level: impl Into<i32>) -> Result<String> {
    let file = File::create(Path::new(output))
        .with_context(|| format!("无法创建输出文件: {output}"))?;
    let mut hash_writer = HashWriter::new(file);
    let enc = Encoder::new(hash_writer, level.into())
        .with_context(|| "无法创建 zstd 编码器")?;
    let mut builder = Builder::new(enc);

    let source_path = Path::new(source);
    let is_dir = source_path.is_dir();
    if is_dir {
        builder
            .append_dir_all(".", source)
            .with_context(|| format!("无法打包目录: {source}"))?;
    } else if source_path.is_file() {
        let file_name = source_path
            .file_name()
            .with_context(|| format!("无法获取文件名: {source}"))?;
        builder
            .append_path_with_name(source, file_name)
            .with_context(|| format!("无法打包文件: {source}"))?;
    } else {
        bail!("源路径不存在或不是文件/目录: {source}");
    }

    builder
        .finish()
        .with_context(|| "无法完成 tar 打包")?;

    let enc = builder
        .into_inner()
        .with_context(|| "无法获取内部编码器")?;
    hash_writer = enc
        .finish()
        .with_context(|| "无法完成 zstd 压缩")?;
    hash_writer.flush()?;

    // 压缩成功，删除源文件或目录
    if is_dir {
        fs::remove_dir_all(source)
            .with_context(|| format!("无法删除源目录: {source}"))?;
    } else {
        fs::remove_file(source)
            .with_context(|| format!("无法删除源文件: {source}"))?;
    }

    let hash = hash_writer.finalize();
    Ok(hash.to_string())
}

/// 解压 tar.zstd 归档到目标目录。
///
/// 与 `pack` 配对使用：`pack` 统一输出 tar.zstd，此函数统一解压。
pub fn unpack(input: &str, output: &str) -> Result<()> {
    let compressed_file = File::open(Path::new(input))?;
    let decoder = Decoder::new(compressed_file)?;
    let mut tar_archive = Archive::new(decoder);
    tar_archive.unpack(output)?;
    Ok(())
}
