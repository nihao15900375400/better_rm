use chrono::Local;
use serde::Deserialize;
use serde::Serialize;
use std::fs::{self, File};
use std::io::{self, BufWriter, Write};
use std::path::{Path, PathBuf};
use tar::Builder;
use thiserror::Error;
use uuid::Uuid;
// ============================================================================
// 错误类型定义
// ============================================================================

#[derive(Error, Debug)]
pub enum CompressionError {
    #[error("路径不存在: {0}")]
    PathNotFound(String),

    #[error("IO 错误 ({path:?}): {source}")]
    Io {
        source: io::Error,
        path: Option<PathBuf>,
    },

    #[error("不支持的压缩模式: {0}")]
    UnsupportedCompression(String),

    #[error("不支持的哈希模式: {0}")]
    UnsupportedHashMode(String),

    #[error("创建输出目录失败: {source}")]
    CreateOutputDir { source: io::Error, path: PathBuf },

    #[error("删除原文件/文件夹失败: {source}")]
    RemoveOriginal { source: io::Error, path: PathBuf },

    #[error("压缩过程失败: {0}")]
    CompressionFailed(String),

    #[error("重命名临时文件失败: {source}")]
    RenameFailed {
        source: io::Error,
        from: PathBuf,
        to: PathBuf,
    },
}

impl From<io::Error> for CompressionError {
    fn from(err: io::Error) -> Self {
        CompressionError::Io {
            source: err,
            path: None,
        }
    }
}

pub fn io_err_with_path(err: io::Error, path: impl Into<PathBuf>) -> CompressionError {
    CompressionError::Io {
        source: err,
        path: Some(path.into()),
    }
}

// ============================================================================
// 枚举配置（编译期检查）
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompressionMode {
    Tar,
    TarGz,
    TarXz,
    TarBz2,
    TarZst,
    TarLz4,
}

impl CompressionMode {
    pub fn from_str(s: &str) -> Result<Self, CompressionError> {
        match s {
            "tar" => Ok(Self::Tar),
            "tar.gz" | "tgz" => Ok(Self::TarGz),
            "tar.xz" | "txz" => Ok(Self::TarXz),
            "tar.bz2" | "tbz2" => Ok(Self::TarBz2),
            "tar.zst" | "tzst" => Ok(Self::TarZst),
            "tar.lz4" | "tlz4" => Ok(Self::TarLz4),
            _ => Err(CompressionError::UnsupportedCompression(s.to_string())),
        }
    }

    pub fn extension(&self) -> &'static str {
        match self {
            Self::Tar => "tar",
            Self::TarGz => "tar.gz",
            Self::TarXz => "tar.xz",
            Self::TarBz2 => "tar.bz2",
            Self::TarZst => "tar.zst",
            Self::TarLz4 => "tar.lz4",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HashMode {
    Blake3,
    Uuid4,
}

impl HashMode {
    pub fn from_str(s: &str) -> Result<Self, CompressionError> {
        match s {
            "blake3" => Ok(Self::Blake3),
            "uuid4" => Ok(Self::Uuid4),
            _ => Err(CompressionError::UnsupportedHashMode(s.to_string())),
        }
    }
}

// ============================================================================
// 配置结构体
// ============================================================================

#[derive(Debug, Clone)]
pub struct CompressionConfig {
    pub compression_level: Option<i32>,
    pub delete_original: bool,
    pub buffer_size: usize,
}

impl Default for CompressionConfig {
    fn default() -> Self {
        Self {
            compression_level: None,
            delete_original: true,
            buffer_size: 1024 * 1024,
        }
    }
}

// ============================================================================
// 返回结果结构体
// ============================================================================
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct CompressionResult {
    pub success: bool,
    pub error_reason: Option<String>,
    pub hash_or_uuid: String,
    pub output_path: PathBuf,
    pub compression_datetime: String,
    pub compressed_size: u64,
    pub original_path: PathBuf,
    pub is_directory: bool,
    pub name: String,
}

// ============================================================================
// Write 适配器：统计字节数
// ============================================================================

struct WriteCounter<W: Write> {
    writer: W,
    count: u64,
}

impl<W: Write> WriteCounter<W> {
    fn new(writer: W) -> Self {
        Self { writer, count: 0 }
    }

    fn into_inner(self) -> W {
        self.writer
    }

    fn count(&self) -> u64 {
        self.count
    }
}

impl<W: Write> Write for WriteCounter<W> {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        let n = self.writer.write(buf)?;
        self.count += n as u64;
        Ok(n)
    }

    fn flush(&mut self) -> io::Result<()> {
        self.writer.flush()
    }
}

// ============================================================================
// Write 适配器：同时写入底层流并计算 BLAKE3 哈希
// ============================================================================

struct TeeWriter<W: Write> {
    writer: W,
    hasher: blake3::Hasher,
}

impl<W: Write> TeeWriter<W> {
    fn new(writer: W) -> Self {
        Self {
            writer,
            hasher: blake3::Hasher::new(),
        }
    }

    fn finalize(self) -> (W, blake3::Hash) {
        (self.writer, self.hasher.finalize())
    }
}

impl<W: Write> Write for TeeWriter<W> {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        let n = self.writer.write(buf)?;
        if n > 0 {
            self.hasher.update(&buf[..n]);
        }
        Ok(n)
    }

    fn flush(&mut self) -> io::Result<()> {
        self.writer.flush()
    }
}

// ============================================================================
// 临时文件 RAII 守卫
// ============================================================================

struct TempFileGuard {
    path: Option<PathBuf>,
}

impl TempFileGuard {
    fn new(path: PathBuf) -> Self {
        Self { path: Some(path) }
    }

    fn dismiss(mut self) {
        self.path = None;
    }
}

impl Drop for TempFileGuard {
    fn drop(&mut self) {
        if let Some(ref p) = self.path {
            if let Err(e) = fs::remove_file(p) {
                eprintln!("清理临时文件失败 {:?}: {}", p, e);
            }
        }
    }
}

// ============================================================================
// 主函数：压缩 + 哈希/UUID + 删除原文件
// ============================================================================

pub fn compress_and_hash(
    path: &str,
    compression_mode: &str,
    hash_mode: &str,
    out_path: &str,
) -> Result<CompressionResult, CompressionError> {
    let mode = CompressionMode::from_str(compression_mode)?;
    let hash = HashMode::from_str(hash_mode)?;
    let config = CompressionConfig::default();

    compress_and_hash_with_config(path, mode, hash, out_path, &config)
}

pub fn compress_and_hash_with_config(
    path: &str,
    mode: CompressionMode,
    hash: HashMode,
    out_path: &str,
    config: &CompressionConfig,
) -> Result<CompressionResult, CompressionError> {
    let original_path = PathBuf::from(path);
    let metadata =
        fs::symlink_metadata(&original_path).map_err(|e| io_err_with_path(e, &original_path))?;
    let is_directory = metadata.is_dir();

    let out_dir = PathBuf::from(out_path);
    if !out_dir.exists() {
        fs::create_dir_all(&out_dir).map_err(|e| CompressionError::CreateOutputDir {
            source: e,
            path: out_dir.clone(),
        })?;
    }

    // 临时文件
    let temp_filename = format!(".tmp_{}", Uuid::new_v4());
    let temp_path = out_dir.join(&temp_filename);
    let temp_guard = TempFileGuard::new(temp_path.clone());

    // 记录压缩开始的时刻（精确到秒）
    let start_datetime = Local::now();

    let (final_identifier, compressed_size) = match hash {
        HashMode::Blake3 => {
            let temp_file =
                File::create(&temp_path).map_err(|e| io_err_with_path(e, &temp_path))?;
            let mut buf_writer = BufWriter::with_capacity(config.buffer_size, temp_file);
            let tee_writer = TeeWriter::new(buf_writer);
            let counter = WriteCounter::new(tee_writer);

            let (counter, size) = compress_with_hasher(counter, &original_path, mode, config)?;
            let (tee_writer, hash_result) = counter.into_inner().finalize();
            // 处理 into_inner 可能失败的情况（BufWriter 的 into_inner 可能因内部错误失败）
            let mut buf_writer = tee_writer.into_inner().map_err(|e| {
                CompressionError::CompressionFailed(format!("提取内部 writer 失败: {}", e))
            })?;
            buf_writer.flush()?;

            (hash_result.to_hex().to_string(), size)
        }
        HashMode::Uuid4 => {
            let temp_file =
                File::create(&temp_path).map_err(|e| io_err_with_path(e, &temp_path))?;
            let mut buf_writer = BufWriter::with_capacity(config.buffer_size, temp_file);
            let counter = WriteCounter::new(buf_writer);

            let (counter, size) = compress_without_hasher(counter, &original_path, mode, config)?;
            counter.into_inner().flush()?;

            (Uuid::new_v4().to_string(), size)
        }
    };

    // 重命名临时文件
    let final_filename = format!("{}.{}", final_identifier, mode.extension());
    let final_path = out_dir.join(&final_filename);

    fs::rename(&temp_path, &final_path).map_err(|e| CompressionError::RenameFailed {
        source: e,
        from: temp_path.clone(),
        to: final_path.clone(),
    })?;
    temp_guard.dismiss();

    // 删除原文件/文件夹
    if config.delete_original {
        let remove_result = if is_directory {
            fs::remove_dir_all(&original_path)
        } else {
            fs::remove_file(&original_path)
        };
        if let Err(e) = remove_result {
            return Err(CompressionError::RemoveOriginal {
                source: e,
                path: original_path.clone(),
            });
        }
    }
    let name = original_path
        .file_name()
        .unwrap()
        .to_str()
        .unwrap()
        .to_string();
    Ok(CompressionResult {
        success: true,
        error_reason: None,
        hash_or_uuid: final_identifier,
        output_path: final_path,
        compression_datetime: start_datetime.format("%Y-%m-%d %H:%M:%S").to_string(),
        compressed_size,
        original_path,
        is_directory,
        name,
    })
}

// ============================================================================
// 压缩核心逻辑：带哈希
// ============================================================================

fn compress_with_hasher<W: Write>(
    mut counter: WriteCounter<TeeWriter<W>>,
    source: &Path,
    mode: CompressionMode,
    config: &CompressionConfig,
) -> Result<(WriteCounter<TeeWriter<W>>, u64), CompressionError> {
    match mode {
        CompressionMode::Tar => {
            counter = build_tar(source, counter)?;
        }
        #[cfg(feature = "gz")]
        CompressionMode::TarGz => {
            use flate2::Compression;
            use flate2::write::GzEncoder;
            let level = config.compression_level.unwrap_or(6) as u32;
            let encoder = GzEncoder::new(counter, Compression::new(level));
            let encoder = build_tar(source, encoder)?;
            counter = encoder.finish()?;
        }
        #[cfg(feature = "xz")]
        CompressionMode::TarXz => {
            use xz2::write::XzEncoder;
            let level = config.compression_level.unwrap_or(6) as u32;
            let encoder = XzEncoder::new(counter, level);
            let encoder = build_tar(source, encoder)?;
            counter = encoder.finish()?;
        }
        #[cfg(feature = "bz2")]
        CompressionMode::TarBz2 => {
            use bzip2::Compression as BzCompression;
            use bzip2::write::BzEncoder;
            let level = config.compression_level.unwrap_or(6) as u32;
            let comp = BzCompression::new(level);
            let encoder = BzEncoder::new(counter, comp);
            let encoder = build_tar(source, encoder)?;
            counter = encoder.finish()?;
        }
        #[cfg(feature = "zstd")]
        CompressionMode::TarZst => {
            use zstd::stream::write::Encoder as ZstdEncoder;
            let level = config.compression_level.unwrap_or(3);
            let encoder = ZstdEncoder::new(counter, level)?;
            let encoder = build_tar(source, encoder)?;
            counter = encoder.finish()?;
        }
        #[cfg(feature = "lz4")]
        CompressionMode::TarLz4 => {
            use lz4::EncoderBuilder;
            let level = config.compression_level.unwrap_or(4) as u32;
            let (writer, result) = {
                let encoder = EncoderBuilder::new().level(level).build(counter)?;
                build_tar(source, encoder)?.finish()
            };
            result?;
            counter = writer;
        }
        #[allow(unreachable_patterns)]
        _ => {
            return Err(CompressionError::UnsupportedCompression(format!(
                "{:?}",
                mode
            )));
        }
    }
    let size = counter.count();
    Ok((counter, size))
}

// ============================================================================
// 压缩核心逻辑：不带哈希
// ============================================================================

fn compress_without_hasher<W: Write>(
    mut counter: WriteCounter<W>,
    source: &Path,
    mode: CompressionMode,
    config: &CompressionConfig,
) -> Result<(WriteCounter<W>, u64), CompressionError> {
    match mode {
        CompressionMode::Tar => {
            counter = build_tar(source, counter)?;
        }
        #[cfg(feature = "gz")]
        CompressionMode::TarGz => {
            use flate2::Compression;
            use flate2::write::GzEncoder;
            let level = config.compression_level.unwrap_or(6) as u32;
            let encoder = GzEncoder::new(counter, Compression::new(level));
            let encoder = build_tar(source, encoder)?;
            counter = encoder.finish()?;
        }
        #[cfg(feature = "xz")]
        CompressionMode::TarXz => {
            use xz2::write::XzEncoder;
            let level = config.compression_level.unwrap_or(6) as u32;
            let encoder = XzEncoder::new(counter, level);
            let encoder = build_tar(source, encoder)?;
            counter = encoder.finish()?;
        }
        #[cfg(feature = "bz2")]
        CompressionMode::TarBz2 => {
            use bzip2::Compression as BzCompression;
            use bzip2::write::BzEncoder;
            let level = config.compression_level.unwrap_or(6) as u32;
            let comp = BzCompression::new(level);
            let encoder = BzEncoder::new(counter, comp);
            let encoder = build_tar(source, encoder)?;
            counter = encoder.finish()?;
        }
        #[cfg(feature = "zstd")]
        CompressionMode::TarZst => {
            use zstd::stream::write::Encoder as ZstdEncoder;
            let level = config.compression_level.unwrap_or(3);
            let encoder = ZstdEncoder::new(counter, level)?;
            let encoder = build_tar(source, encoder)?;
            counter = encoder.finish()?;
        }
        #[cfg(feature = "lz4")]
        CompressionMode::TarLz4 => {
            use lz4::EncoderBuilder;
            let level = config.compression_level.unwrap_or(4) as u32;
            let (writer, result) = {
                let encoder = EncoderBuilder::new().level(level).build(counter)?;
                build_tar(source, encoder)?.finish()
            };
            result?;
            counter = writer;
        }
        #[allow(unreachable_patterns)]
        _ => {
            return Err(CompressionError::UnsupportedCompression(format!(
                "{:?}",
                mode
            )));
        }
    }
    let size = counter.count();
    Ok((counter, size))
}

// ============================================================================
// 构建 TAR 归档（接受所有权，返回原始 Writer）
// ============================================================================

fn build_tar<W: Write>(source: &Path, writer: W) -> Result<W, CompressionError> {
    let mut writer = writer;
    let mut builder = Builder::new(&mut writer);

    if source.is_dir() {
        builder
            .append_dir_all(".", source)
            .map_err(|e| CompressionError::CompressionFailed(format!("添加目录失败: {}", e)))?;
    } else {
        let file_name = source
            .file_name()
            .ok_or_else(|| CompressionError::CompressionFailed("无法获取文件名".to_string()))?;
        let mut file = File::open(source).map_err(|e| io_err_with_path(e, source))?;
        builder
            .append_file(file_name, &mut file)
            .map_err(|e| CompressionError::CompressionFailed(format!("添加文件失败: {}", e)))?;
    }

    builder
        .finish()
        .map_err(|e| CompressionError::CompressionFailed(format!("完成 tar 归档失败: {}", e)))?;
    drop(builder);
    Ok(writer)
}

// ============================================================================
// 单元测试
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_compress_file_blake3() -> Result<(), CompressionError> {
        let tmp_dir = TempDir::new().unwrap();
        let src = tmp_dir.path().join("test.txt");
        fs::write(&src, b"hello world").unwrap();

        let result = compress_and_hash(
            src.to_str().unwrap(),
            "tar.gz",
            "blake3",
            tmp_dir.path().to_str().unwrap(),
        )?;

        assert!(result.success);
        assert!(result.output_path.exists());
        assert_eq!(result.hash_or_uuid.len(), 64);
        assert!(!src.exists());
        Ok(())
    }

    #[test]
    fn test_compress_dir_uuid4() -> Result<(), CompressionError> {
        let tmp_dir = TempDir::new().unwrap();
        let src_dir = tmp_dir.path().join("mydir");
        fs::create_dir(&src_dir).unwrap();
        fs::write(src_dir.join("a.txt"), b"aaa").unwrap();
        fs::write(src_dir.join("b.txt"), b"bbb").unwrap();

        let result = compress_and_hash(
            src_dir.to_str().unwrap(),
            "tar",
            "uuid4",
            tmp_dir.path().to_str().unwrap(),
        )?;

        assert!(result.success);
        assert!(result.is_directory);
        assert_eq!(result.hash_or_uuid.len(), 36);
        assert!(!src_dir.exists());
        Ok(())
    }
}

/// 根据压缩结果删除对应的压缩包文件
pub fn delete_archive(result: &CompressionResult) -> Result<(), CompressionError> {
    if result.output_path.exists() {
        fs::remove_file(&result.output_path)
            .map_err(|e| io_err_with_path(e, &result.output_path))?;
    }
    Ok(())
}
/// 将压缩包解压到原始路径所在的目录，并删除压缩包
pub fn extract_and_delete(result: &CompressionResult) -> Result<(), CompressionError> {
    // 确定解压目标目录：原始路径的父目录
    let parent_dir = result.original_path.parent().ok_or_else(|| {
        CompressionError::CompressionFailed("无法获取原始路径的父目录".to_string())
    })?;

    // 打开压缩包
    let archive_file =
        File::open(&result.output_path).map_err(|e| io_err_with_path(e, &result.output_path))?;
    let mut archive = tar::Archive::new(archive_file);

    // 解压到目标目录
    archive
        .unpack(parent_dir)
        .map_err(|e| CompressionError::CompressionFailed(format!("解压失败: {}", e)))?;

    // 删除压缩包
    delete_archive(result)?;

    Ok(())
}
