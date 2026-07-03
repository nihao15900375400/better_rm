// Copyright (c) 2026 ywnh1
// del is licensed under Mulan PSL v2.
// You can use this software according to the terms and conditions of the Mulan
// PSL v2.
// You may obtain a copy of Mulan PSL v2 at:
//          http://license.coscl.org.cn/MulanPSL2
// THIS SOFTWARE IS PROVIDED ON AN "AS IS" BASIS, WITHOUT WARRANTIES OF ANY
// KIND, EITHER EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED TO
// NON-INFRINGEMENT, MERCHANTABILITY OR FIT FOR A PARTICULAR PURPOSE.
// See the Mulan PSL v2 for more details.
//
use crate::sql::Trash;
use crate::util::file_size;
use anyhow::{Result, bail};
use blake3::Hasher;
use std::fs::File;
use std::io;
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};
use tar::Builder;
use tempfile::NamedTempFile;
use zstd::{Decoder, Encoder};

/// 将文件或目录打包成 zstd 压缩的 tar 归档，存入回收站。
///
/// - 对于**文件**：将文件作为单个条目加入归档，条目名为源文件名。
/// - 对于**目录**：递归地将目录下所有内容加入归档，条目路径以目录名为前缀。
///
/// 归档文件命名为 `trash_dir/{hash}.bak`，`Trash.path` 保存**原始路径**。
pub fn pack(src: &Path, trash_dir: &Path, level: i32) -> Result<Trash> {
    let mut tmp = NamedTempFile::new_in(trash_dir)?;

    // 将源文件/目录写入 zstd 压缩的 tar 归档
    {
        let mut encoder = Encoder::new(&mut tmp, level)?;
        {
            let mut builder = Builder::new(&mut encoder);

            if src.is_file() {
                let name = src
                    .file_name()
                    .ok_or_else(|| anyhow::anyhow!("{} has no file name", src.display()))?;
                let mut file = File::open(src)?;
                builder.append_file(name, &mut file)?;
            } else if src.is_dir() {
                let name = src
                    .file_name()
                    .ok_or_else(|| anyhow::anyhow!("{} has no file name", src.display()))?;
                builder.append_dir_all(name, src)?;
            } else {
                bail!("{} is neither a file nor a directory", src.display());
            }

            builder.finish()?;
        }
        encoder.finish()?;
    }

    // 对归档内容计算 blake3 哈希（用于去重和验证）
    let hash = {
        let mut hasher = Hasher::new();
        let mut tmp_file = File::open(tmp.path())?;
        io::copy(&mut tmp_file, &mut hasher)?;
        hasher.finalize().to_string()
    };

    // 持久化归档到 trash_dir/{hash}.bak
    let archive_path = trash_dir.join(format!("{hash}.bak"));
    tmp.persist(&archive_path)?;

    let size = file_size(&archive_path)?;
    let time = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis() as i64;

    Ok(Trash {
        id: 0,
        time,
        path: src.to_string_lossy().to_string(),
        hash,
        size,
    })
}

/// 从回收站中解压归档到原始路径。
///
/// 归档格式始终为：对文件，条目名为 `filename`；
/// 对目录，所有条目以 `dirname/` 为前缀。
/// 因此统一解压到 `target_path.parent()` 即可恢复原始结构。
pub fn unpack(hash: &str, trash_dir: &Path, target_path: &Path) -> Result<()> {
    let archive_path = trash_dir.join(format!("{hash}.bak"));
    let file = File::open(&archive_path)?;
    let decoder = Decoder::new(file)?;
    let mut archive = tar::Archive::new(decoder);
    let parent = target_path
        .parent()
        .ok_or_else(|| anyhow::anyhow!("{} has no parent directory", target_path.display()))?;
    archive.unpack(parent)?;
    Ok(())
}
