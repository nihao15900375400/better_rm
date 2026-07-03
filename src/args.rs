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
use clap::Parser;

#[derive(Parser, Debug)]
#[command(name = "del")]
#[command(about = "A useful tool to remove files safely", long_about = None,author, version)]
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
    /// To remove the files or dirs not included by disable list forece forever
    #[arg(short, long)]
    pub force: bool,
    /// Clean trash automatic
    #[arg(short, long)]
    pub autoclean: bool,
    /// Clear trash
    #[arg(short = 'C', long)]
    pub clear: bool,
    /// Show log
    #[arg(short, long)]
    pub verbose: bool,
}
