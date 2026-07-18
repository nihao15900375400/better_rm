use clap::Parser;

#[derive(Parser, Debug)]
#[command(name = "del")]
#[command(author, version)]
/// A useful tool to remove files safely
pub struct Cli {
    /// Files or dirs to delete
    pub path: Vec<String>,
    /// Show and edit trash interactive
    #[arg(short, long)]
    pub trash: bool,
    /// Show and edit config interactive
    #[arg(short = 'c', long)]
    pub config: bool,
    /// Only used with `--force` | `-f`
    #[arg(short = 'r', long = "recursive")]
    pub recursive: bool,
    /// Permanently delete files/dirs (bypass trash, use system rm)
    #[arg(short, long)]
    pub force: bool,
    /// Auto-clean archives older than saving_days
    #[arg(short, long)]
    pub autoclean: bool,
    /// Clear all trash archives
    #[arg(short = 'C', long)]
    pub clear: bool,
    /// Show detailed logs
    #[arg(short, long)]
    pub verbose: bool,
    /// Temporary trash directory path
    #[arg(long)]
    pub trash_dir: Option<String>,
    /// Temporary backup retention days
    #[arg(long)]
    pub saving_days: Option<u16>,
    /// Temporarily add a path to the disable list
    #[arg(long)]
    pub add_disable: Vec<String>,
    /// Temporary zstd compression level (-5 to 22)
    #[arg(long)]
    pub compression_level: Option<i32>,
    /// Save them in trash without delete them
    #[arg(long, short)]
    pub save: bool,
}

use crate::t_str;
use clap::Command;

/// Apply i18n translations to the clap Command (help text).
pub fn apply_i18n(cmd: Command) -> Command {
    cmd.about(t_str!("app.about"))
        .mut_arg("path", |a| a.help(t_str!("args.path")))
        .mut_arg("trash", |a| a.help(t_str!("args.trash")))
        .mut_arg("config", |a| a.help(t_str!("args.config")))
        .mut_arg("recursive", |a| a.help(t_str!("args.recursive")))
        .mut_arg("force", |a| a.help(t_str!("args.force")))
        .mut_arg("autoclean", |a| a.help(t_str!("args.autoclean")))
        .mut_arg("clear", |a| a.help(t_str!("args.clear")))
        .mut_arg("verbose", |a| a.help(t_str!("args.verbose")))
        .mut_arg("trash_dir", |a| a.help(t_str!("args.trash_dir")))
        .mut_arg("saving_days", |a| a.help(t_str!("args.saving_days")))
        .mut_arg("add_disable", |a| a.help(t_str!("args.add_disable")))
        .mut_arg("compression_level", |a| {
            a.help(t_str!("args.compression_level"))
        })
        .mut_arg("save", |a| a.help(t_str!("args.save")))
}
