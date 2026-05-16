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
use crate::conf::*;
use crate::db::Database;
use crate::utils;
use crate::utils::to_abs_path;
use bzip2::read::BzDecoder;
use bzip2::write::BzEncoder;
use flate2::read::GzDecoder;
use flate2::write::GzEncoder;
use serde::{Deserialize, Serialize};
use sqlx::Type;
use std::error::Error;
use std::fmt;
use std::fs;
use std::fs::File;
use std::path::PathBuf;
use tar::Builder;
use xz2::read::XzDecoder;
use xz2::write::XzEncoder;
use zstd::stream::Decoder;
use zstd::stream::Encoder;

macro_rules! compress {
    ($src_path:expr, $enc:expr) => {{
        use std::fs;
        use std::path::Path;

        let mut builder = Builder::new($enc);
        let src = Path::new($src_path);
        let abs_src = src.canonicalize()?;
        let abs_src_str = abs_src.to_str().ok_or_else(|| {
            std::io::Error::new(std::io::ErrorKind::InvalidData, "path is not UTF-8")
        })?;
        let archive_name = abs_src_str.strip_prefix('/').ok_or_else(|| {
            std::io::Error::new(std::io::ErrorKind::InvalidData, "invalid absolute path")
        })?;

        if abs_src.is_file() {
            builder.append_path_with_name(&abs_src, archive_name)?;
        } else if abs_src.is_dir() {
            for entry in walkdir::WalkDir::new(&abs_src)
                .into_iter()
                .filter_map(|e| e.ok())
            {
                let path = entry.path();
                if path.is_symlink() || !path.is_file() {
                    continue;
                }
                let abs_entry = path.canonicalize()?;
                let abs_entry_str = abs_entry.to_str().ok_or_else(|| {
                    std::io::Error::new(std::io::ErrorKind::InvalidData, "path is not UTF-8")
                })?;
                let entry_arch_name = abs_entry_str.strip_prefix('/').ok_or_else(|| {
                    std::io::Error::new(std::io::ErrorKind::InvalidData, "invalid absolute path")
                })?;
                builder.append_path_with_name(path, entry_arch_name)?;
            }
        }

        let encoder = builder.into_inner()?;
        encoder.finish()?;
        if abs_src.is_dir() {
            fs::remove_dir_all(abs_src)?;
        } else {
            fs::remove_file(abs_src)?;
        }
    }};
}
macro_rules! unpack {
    ($decoder:expr) => {{
        let mut archive = tar::Archive::new($decoder);
        archive.unpack("/")?;
    }};
}
pub fn unpack(db: &Database) -> Result<(), Box<dyn Error>> {
    let path = &to_abs_path(&db.present_path);
    let original_path = to_abs_path(&db.original_path);
    if original_path.exists() {
        eprintln!(
            "There has been a file or dir at {}",
            original_path.display()
        );
        return Err(Box::new(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            "file or directory exist",
        )));
    }
    let present_path = File::open(path)?;
    match db.archive_tool {
        ArchiveTool::Xz2 => unpack!(XzDecoder::new(present_path)),
        ArchiveTool::Zstd => unpack!(Decoder::new(present_path)?),
        ArchiveTool::Bz2 => unpack!(BzDecoder::new(present_path)),
        ArchiveTool::Gz => unpack!(GzDecoder::new(present_path)),
    }
    Ok(())
}
pub fn compress(src_dir: &PathBuf, cfg: &Config) -> Result<Database, Box<dyn Error>> {
    let time = utils::now_time();
    let mut present_path = to_abs_path(&cfg.trash);
    present_path.push(format!("{}.bak", time));
    match cfg.archive_tool {
        ArchiveTool::Xz2 => compress!(src_dir, XzEncoder::new(File::create(&present_path)?, 6)),
        ArchiveTool::Zstd => compress!(src_dir, Encoder::new(File::create(&present_path)?, 6)?),
        ArchiveTool::Bz2 => compress!(
            src_dir,
            BzEncoder::new(File::create(&present_path)?, bzip2::Compression::best())
        ),
        ArchiveTool::Gz => compress!(
            src_dir,
            GzEncoder::new(File::create(&present_path)?, flate2::Compression::default())
        ),
    }
    let name = src_dir
        .file_name()
        .ok_or("Invalid file name")?
        .to_str()
        .ok_or("The file name is not UTF-8")?;

    let size = fs::metadata(&present_path)
        .map_err(|e| format!("error: {}", e))?
        .len();

    Ok(Database {
        id: 0,
        name: name.to_string(),
        original_path: src_dir.display().to_string(),
        present_path: present_path.display().to_string(),
        archive_tool: cfg.archive_tool,
        size: size.try_into()?,
        time,
    })
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
            "xz2" => Some(Self::Xz2),
            "zstd" => Some(Self::Zstd),
            "bz2" => Some(Self::Bz2),
            "gz" => Some(Self::Gz),
            _ => None,
        }
    }
}
impl fmt::Display for ArchiveTool {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            ArchiveTool::Xz2 => "xz2",
            ArchiveTool::Zstd => "zstd",
            ArchiveTool::Bz2 => "bz2",
            ArchiveTool::Gz => "gz",
        };
        write!(f, "{}", s)
    }
}
impl From<String> for ArchiveTool {
    fn from(value: String) -> Self {
        Self::from_str(&value).unwrap_or(Self::Gz)
    }
}
