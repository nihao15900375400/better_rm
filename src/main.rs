mod args;
mod pack;
mod sql;
// config 是外部 crate（Cargo.toml 中 path = "./config"），通过 use 引入
use config;
use anyhow::{Result, ensure};
use clap::Parser;
use dialoguer::{Confirm, Input, MultiSelect, theme::ColorfulTheme};
use pack::*;
use tokio;

#[tokio::main]
async fn main() -> Result<()> {
    let args = args::Args::parse();
    let theme = ColorfulTheme::default();
    if args.force {
        let status: std::process::ExitStatus = if args.recursive {
            std::process::Command::new("rm")
                .arg("-rf")
                .args(args.path)
                .status()?
        } else {
            std::process::Command::new("rm")
                .arg("-f")
                .args(args.path)
                .status()?
        };
        ensure!(
            status.success(),
            "执行失败：ExitCode: {}",
            status.code().unwrap_or(-1)
        );
    } else if args.config {
    }
    Ok(())
}
