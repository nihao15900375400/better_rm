use std::path::{Path, PathBuf};
use crate::constants;
use crate::utils;
use crate::conf::*;
use std::error::Error;
use sqlx::sqlite::SqlitePool;
use sqlx::Row;
use std::fs;
use serde::{Serialize, Deserialize};
use sqlx::Type;
use std::fs::File;
use tar::Builder;
use flate2::write::GzEncoder;
use walkdir::WalkDir; 
use xz2::write::XzEncoder;
use zstd::stream::Encoder;
use zstd::stream::Decoder;
use flate2::read::GzDecoder;
use bzip2::read::BzDecoder;
use xz2::read::XzDecoder;
use bzip2::write::BzEncoder;
use std::fmt;
use std::convert::TryFrom;


macro_rules! compress {
    ($src_dir:expr, $output:expr, $enc:expr) => {{
        let _f = File::create($output)?;
        let mut builder = Builder::new($f);
        for entry in WalkDir::new($src_dir).into_iter().filter_map(|e| e.ok()) {
            let path = entry.path();
            let name_str = path.to_str().ok_or_else(|| {
                std::io::Error::new(std::io::ErrorKind::InvalidData, "the path is not UTF-8")
            })?;
            if path.is_file() {
                builder.append_path_with_name(path, name_str)?;
            } else if path.is_dir() {
                builder.append_dir(name_str, path)?;
            }
        }
        let encoder = builder.into_inner()?;
        encoder.finish()?;
        fs::remove_dir_all($src_dir)?;
    }};
}
macro_rules! unpack {
    ($archive_path:expr, $decoder_ctor:expr) => {{
        let file = File::open($archive_path)?;
        let decoder = $decoder_ctor(file);
        let mut archive = tar::Archive::new(decoder);
        archive.set_preserve_path(true);
        archive.set_preserve_absolute_paths(true);
        archive.set_ignore_umask(false);
        archive.unpack("/")?;
    }};
}







#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq, Type)]
#[sqlx(type_name = "TEXT")]
#[sqlx(rename_all = "snake_case")]
#[serde(rename_all = "lowercase")]
pub enum ArchiveTool {
    Xz2,
    Zstd,
    Bz2,
    Gz,
}

impl ArchiveTool {
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "xz2"  => Some(Self::Xz2),
            "zstd" => Some(Self::Zstd),
            "bz2"  => Some(Self::Bz2),
            "gz"   => Some(Self::Gz),
            _      => None,
        }
    }
}

// 只保留这一个 Display 实现，删掉文件里另一个重复的
impl fmt::Display for ArchiveTool {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            ArchiveTool::Xz2  => "xz2",
            ArchiveTool::Zstd => "zstd",
            ArchiveTool::Bz2  => "bz2",
            ArchiveTool::Gz   => "gz",
        };
        write!(f, "{}", s)
    }
}

// 关键：sqlx 要的 From<String>
impl From<String> for ArchiveTool {
    fn from(value: String) -> Self {
        Self::from_str(&value).unwrap_or(Self::Gz)
    }
}



#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum HashTool{
    Blake3 ,
    Sha2 ,
    Md5,
}

