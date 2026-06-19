// Copyright (c) 2026 ywnh1
// del is licensed under Mulan PSL v2.
// You can use this software according to the terms and conditions of the Mulan PSL v2.
// You may obtain a copy of Mulan PSL v2 at:
//          http://license.coscl.org.cn/MulanPSL2
// THIS SOFTWARE IS PROVIDED ON AN "AS IS" BASIS, WITHOUT WARRANTIES OF ANY KIND,
// EITHER EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED TO NON-INFRINGEMENT,
// MERCHANTABILITY OR FIT FOR A PARTICULAR PURPOSE.
// See the Mulan PSL v2 for more details.

use clap::Parser;

#[derive(Parser, Debug)]
#[command(name = "del")]
#[command(about = "A useful tool to remove files safely", long_about = None,author, version)]
pub struct Args {
    /// 要删除的路径
    pub path: Vec<String>,
    /// 交互修改配置文件
    #[arg(short = 'c', long)]
    pub config: bool,
    /// 仅与`--force/-f`连用，递归删除文件夹
    #[arg(short = 'r', long = "recursive")]
    pub recursive: bool,
    /// 调用系统命令`rm -f`，强制永久删除，不收禁删名单限制
    #[arg(short, long)]
    pub force: bool,
    /// 交互式操作回收站，使用时会忽略其他参数
    #[arg(short, long)]
    pub interact: bool,
    /// 自动清理文件，建议经常执行释放空间
    #[arg(short, long)]
    pub autoclean: bool,
    /// 清空trash
    #[arg(short = 'C', long)]
    pub clear: bool,
}
// 记得select delete restore + config clear
