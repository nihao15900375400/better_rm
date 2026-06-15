use clap::Parser;

#[derive(Parser, Debug)]
#[command(name = "del")]
#[command(about = "A useful tool to remove files safely", long_about = None,author, version)]
pub struct Args {
    /// 要删除的路径
    pub path: Vec<String>,
    /// 交互式操作回收站
    #[arg(short, long)]
    pub list: bool,
    /// 交互修改配置文件
    #[arg(short, long)]
    pub config: bool,
    /// 仅与`--force/-f`连用，递归删除文件夹
    #[arg(short = 'r', long = "recursive")]
    pub recursive: bool,
    /// 调用系统命令`rm -f`，强制永久删除，不收禁删名单限制
    #[arg(short, long)]
    pub force: bool,
    ///
    #[arg(short, long)]
    pub interact: bool,
}
// 记得select delete restore
