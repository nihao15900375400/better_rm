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
