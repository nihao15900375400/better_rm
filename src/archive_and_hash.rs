use std::path::{Path, PathBuf};
use std::io::{self, Read, Write};
use crate::constants;
use crate::utils;
use std::error::Error;
use sqlx::sqlite::SqlitePool;
use sqlx::Row;
use std::fs;
use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "lowercase")]
pub enum ArchiveTool{
    Tar ,
    Xz2 ,
    Zstd ,
    Bz2 ,
    Gz,
}
#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "lowercase")]
pub enum HashTool{
    Blake3 ,
    Sha2 ,
    Md5,
}

