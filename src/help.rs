// Copyright (c) 2026 ywnh1
//
// del is licensed under Mulan PSL v2.
// You can use this software according to the terms and conditions of the Mulan
// PSL v2. You may obtain a copy of Mulan PSL v2 at:
//         http://license.coscl.org.cn/MulanPSL2
//
// THIS SOFTWARE IS PROVIDED ON AN "AS IS" BASIS, WITHOUT WARRANTIES OF ANY
// KIND, EITHER EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED TO
// NON-INFRINGEMENT, MERCHANTABILITY OR FIT FOR A PARTICULAR PURPOSE.
// See the Mulan PSL v2 for more details.

use crate::utils::print_md;
const list_help: &str = r#"
---
# `del` - 安全易用的文件回收站工具
> 一个基于 Rust 实现的命令行文件管理工具，支持压缩存储、哈希校验、配置化管理的"回收站"功能。

## `list` - 列出回收站内容

### 功能说明
查看回收站中已存档的文件/文件夹记录，支持按名称筛选查看详细信息。

### 用法示例
```bash
del -l                      # 列出所有回收站条目（表格形式）
# --list 长命令
del -l filename             # 查看指定名称的存档详情
del -l file1 file2          # 批量查看多个存档详情
```

### 输出说明
- 无参数时：以表格形式展示所有存档的 `名称 | 原始路径 | 压缩大小 | 创建时间 | 哈希值`
- 带参数时：展示单个存档的完整数据

### 注意事项
- 若回收站为空，会显示蓝色提示"空"
- 查询不存在的名称会报错并退出

---
"#;
const list_setting_help: &str = r#"
---
# `del` - 安全易用的文件回收站工具
> 一个基于 Rust 实现的命令行文件管理工具，支持压缩存储、哈希校验、配置化管理的"回收站"功能。

## `list-setting` - 查看程序配置

### 功能说明
显示当前工具的运行时配置信息，包括版本、回收站路径、压缩算法、索引方式等，并可选择性查看禁删列表。

### 用法示例
```bash
del --list-setting          # 查看完整配置信息
```

### 输出内容
```
版本: 0.1.0
回收站: /your/recycle/bin
压缩工具: ...
hash/uuid: ...
```

### 交互选项
- 执行后会询问 `查看禁删列表？（y/n）：`
- 输入 `y` 或 `Y` 将逐行显示 `disabled_list` 中的受保护文件名（蓝色高亮）

### 注意事项
- 配置信息来源于 `config.json`，若加载失败会报错退出
- 禁删列表用于防止误删重要文件

---
"#;
const restore_help: &str = r#"
---
# `del` - 安全易用的文件回收站工具
> 一个基于 Rust 实现的命令行文件管理工具，支持压缩存储、哈希校验、配置化管理的"回收站"功能。

## `restore` - 恢复存档文件

### 功能说明
将回收站中的压缩存档解压并还原到原始路径，恢复完成后自动删除该存档记录。

### 用法示例
```bash
del -r filename             # 恢复单个文件/文件夹
# --restore 长命令
del -r file1 file2          # 批量恢复多个存档
del -r                      # 无参数时显示本功能帮助
```

### 执行流程
1. 根据名称从 `.information.json` 中读取存档元数据
2. 调用 `extract_and_delete` 执行解压还原
3. 从索引文件中移除该记录，完成"恢复即删除"逻辑

### 注意事项
- 恢复路径为存档时记录的原始路径，请确保目标路径可写
- 若名称不存在或元数据损坏，会报错并退出
- 恢复操作不可逆，请提前确认

---
"#;
const delete_help: &str = r#"
---
# `del` - 安全易用的文件回收站工具
> 一个基于 Rust 实现的命令行文件管理工具，支持压缩存储、哈希校验、配置化管理的"回收站"功能。

## `delete` - 永久删除存档记录

### 功能说明
从回收站中彻底移除指定名称的存档（包括压缩文件和索引记录），**不可恢复**。

### 用法示例
```bash
del -d filename             # 删除单个存档
# --delete 长命令
del -d file1 file2          # 批量删除多个存档
del -d                      # 无参数时显示本功能帮助
```

### 执行流程
1. 查询名称对应的存档元数据
2. 调用 `delete_archive` 删除物理压缩文件
3. 调用 `delete_result_by_name` 从索引 JSON 中移除记录

### 注意事项
- ⚠️ 此操作为**永久删除**，无法通过 `restore` 恢复
- 删除不存在的名称会报错退出

---
"#;
const clear_help: &str = r#"
---
## `clear` - 清空整个回收站
> 一个基于 Rust 实现的命令行文件管理工具，支持压缩存储、哈希校验、配置化管理的"回收站"功能。

### 功能说明
一键清空回收站目录及索引文件，**彻底删除所有存档**，操作前需二次确认。

### 用法示例
```bash
del -c                      # 触发清空流程
del --clear                 # 同上，长格式
```

### 执行流程
1. 提示 `清空回收站？（y/n）：` 等待用户确认
2. 若输入非 `y/Y`，直接退出
3. 调用 `fs::remove_dir_all` 删除整个回收站目录
4. 重建目录结构并初始化空的 `.information.json`

### 注意事项
- ⚠️ **高危操作**：所有存档将永久丢失，请提前备份重要数据
- 清空后索引文件会重建，但内容为空
- 若目录权限不足可能导致清空失败

---
"#;
const version_help: &str = r#"
---
# `del` - 安全易用的文件回收站工具
> 一个基于 Rust 实现的命令行文件管理工具，支持压缩存储、哈希校验、配置化管理的"回收站"功能。

## `version` - 查看版本信息

### 功能说明
显示当前安装的 `del` 工具版本号，版本信息来源于配置文件。

### 用法示例
```bash
del -v                      # 短格式查看版本
del --version               # 长格式查看版本
```

### 输出示例
```
版本: 0.1.0
```

### 注意事项
- 若 `config.json` 加载失败，会报错并退出
- 版本号用于兼容性检查和故障排查

---
"#;
const help_help: &str = r#"
---
# `del` - 安全易用的文件回收站工具
> 一个基于 Rust 实现的命令行文件管理工具，支持压缩存储、哈希校验、配置化管理的"回收站"功能。

## help` - 查看帮助文档

### 功能说明
显示工具的帮助信息，支持查看全局帮助或指定子命令的详细用法。

### 用法示例
```bash
del -h                      # 显示全局帮助（所有命令概览）
del --help                  # 同上
del -h list                 # 查看 list 子命令的详细说明
del --help restore          # 查看 restore 子命令的详细说明
```

### 支持的子命令帮助
| 参数 | 说明 |
|------|------|
| `list` | 列出回收站内容 |
| `list-setting` | 查看配置信息 |
| `restore` | 恢复存档文件 |
| `delete` | 永久删除存档 |
| `clear` | 清空回收站 |
| `version` | 查看版本 |
| `help` | 查看帮助（本项） |
| `force` | 强制删除（忽略保护） |
| `config` | 查看配置文件路径及内容 |
| `remove` | 默认删除行为说明 |

### 注意事项
- 无参数或参数不匹配时，默认显示全局帮助
- 帮助内容使用 Markdown 渲染，支持终端富文本显示

---
"#;
const force_help: &str = r#"
---
# `del` - 安全易用的文件回收站工具
> 一个基于 Rust 实现的命令行文件管理工具，支持压缩存储、哈希校验、配置化管理的"回收站"功能。

## `force` - 强制删除（绕过保护）

### 功能说明
强制将文件添加到回收站，**即使文件名在 `disabled_list` 中**，操作前会二次确认。

### 用法示例
```bash
del -f protected_file       # 强制删除受保护的文件
# --force 长命令
del --f file1 file2         # 批量强制删除
del -f                      # 无参数时显示本功能帮助
```

### 执行流程
1. 遍历输入的文件名列表
2. 若文件名在 `disabled_list` 中，提示 `确定删除 <name>？（y/n）：`
3. 用户确认后，调用 `compress_and_hash` 压缩并生成元数据
4. 将结果追加到 `.information.json`

### 注意事项
- 二次确认防止误操作，输入非 `y/Y` 则跳过该文件
- 压缩失败会 `unwrap()` 导致程序退出，建议提前检查文件权限

---
"#;
const config_help: &str = r#"
---
# `del` - 安全易用的文件回收站工具
> 一个基于 Rust 实现的命令行文件管理工具，支持压缩存储、哈希校验、配置化管理的"回收站"功能。

## `config` - 查看配置文件

### 功能说明
显示当前使用的 `config.json` 配置文件的绝对路径，并用 `cat` 命令输出其内容。

### 用法示例
```bash
del --config                # 查看配置文件路径及内容
```

### 输出示例
```
/path/to/your/config
{
    "version": "0.1.0",
    "author": "ywnh1",
    "compression_tool": "tar",
    "index_tool": "uuid4",
    "recycle": "your/recycle/bin",
    "options_supported": {
        "compression_tool": [
            "tar",
            "tar.gz",
            "tgz",
            "tar.xz",
            "txz",
            "tar.bz2",
            "tbz2",
            "tar.zst",
            "tzst",
            "tar.lz4",
            "tlz4"
        ],
        "index_tool": [
            "blake3",
            "uuid4"
        ]
    },
    "disabled_list": [...]
}
```

### 执行流程
1. 调用 `find_config_upwards` 从当前目录向上查找 `config.json`
2. 打印配置文件的绝对路径（绿色高亮）
3. 调用系统 `cat` 命令输出文件内容

### 注意事项
- 若未找到配置文件，`find_config_upwards` 会 `unwrap()` 导致崩溃
- 依赖系统 `cat` 命令，Windows 环境需确保兼容（或使用替代方案）
- 配置文件内容直接输出，无格式化，建议使用 `jq` 等工具进一步处理

---
"#;
const remove_help: &str = r#"
---
# `del` - 安全易用的文件回收站工具
> 一个基于 Rust 实现的命令行文件管理工具，支持压缩存储、哈希校验、配置化管理的"回收站"功能。

## `remove` - 默认行为：删除文件到回收站

### 功能说明
**无子命令时的默认行为**：将指定文件/文件夹压缩后存入回收站，并记录元数据。

### 用法示例
```bash
del file.txt                # 删除单个文件
del dir1/ dir2/             # 批量删除多个路径
del *.log                   # 使用 shell 通配符批量删除
```

### 执行流程
1. 遍历所有输入路径
2. 若路径在 `disabled_list` 中，提示 `文件名 禁止删除` 并跳过
3. 调用 `compress_and_hash` 执行压缩 + 哈希计算
4. 将结果通过 `append_result_to_json` 写入 `.information.json`

### 压缩与哈希策略
- 压缩工具：由 `config.json` 的 `compression_tool` 指定（如 `zstd`, `gzip`）
- 索引方式：由 `index_tool` 指定（如 `sha256`, `uuid`），用于生成唯一存档名
- 存档路径：`{recycle}/{index}.{ext}`，避免命名冲突

### 注意事项
- 受 `disabled_list` 保护的文件需使用 `--force` 才能删除
- 压缩失败或写入索引失败会导致 `unwrap()` 崩溃，建议生产环境增加错误处理
- 原始文件在压缩成功后**不会被自动删除**，需手动清理或使用其他工具联动

---
"#;
const main_help: &str = r#"
---
# del - 安全易用的命令行文件回收站工具

```
用法: del [COMMAND] [OPTIONS] [ARGS...]
```

> 一个基于 Rust 实现的命令行文件管理工具，支持将文件压缩存储到回收站、按名称恢复、哈希校验、配置化管理等功能。

---

## 命令速查

```
删除文件      del <file>                    默认行为，压缩存档到回收站
列出存档      del -l, --list                查看回收站中的存档记录
恢复文件      del -r, --restore <name>      解压存档并还原到原始路径
永久删除      del -d, --delete <name>       彻底移除存档（不可恢复）
清空回收站    del -c, --clear               一键清空所有存档
强制删除      del -f, --force <file>        绕过禁删列表，强制添加存档
查看配置      del --config                  显示配置文件路径及内容
查看设置      del --list-setting            显示运行时配置与禁删列表
查看版本      del -v, --version             显示当前工具版本号
查看帮助      del -h, --help [cmd]          显示全局帮助或子命令详解
参数分隔      del -- <file>                 处理以 - 开头的特殊文件名
```

---

## 命令详解

### 默认行为：删除文件到回收站
```
del file.txt
del dir1/ dir2/
del -- -file.txt
```
- 将指定文件/目录压缩后存入回收站，记录元数据到 `.information.json`
- 原始文件不会自动删除，需手动清理
- 受 `disabled_list` 保护的文件会被跳过，使用 `--force` 可绕过

### -l, --list [name]
```
del -l
del -l filename
```
- 无参数：以表格形式列出所有存档（名称/原路径/大小/时间/哈希）
- 带参数：显示指定存档的详细元数据

### -r, --restore <name> [name...]
```
del -r filename
del -r file1 file2
```
- 将存档解压还原到原始记录路径
- 恢复成功后自动删除该存档记录
- 请确保目标路径可写，避免覆盖现有文件

### -d, --delete <name> [name...]
```
del -d filename
del -d file1 file2
```
- 永久删除指定存档：同时移除压缩文件与索引记录
- 操作不可逆，请谨慎使用
- 受保护文件需配合 `--force` 使用

### -c, --clear
```
del -c
```
- 清空整个回收站目录及索引文件
- 执行前需二次确认（输入 y 确认）
- 高危操作，所有存档将永久丢失

### -f, --force <file> [file...]
```
del -f protected_file
```
- 强制将文件添加到回收站，即使名称在 `disabled_list` 中
- 对受保护文件会提示二次确认
- 仅绕过"添加存档"时的保护逻辑

### --config
```
del --config
```
- 显示当前使用的 `config.json` 绝对路径
- 并用 `cat` 命令输出配置文件内容
- 配置文件从当前目录向上递归查找

### --list-setting
```
del --list-setting
```
- 显示版本、回收站路径、压缩工具、索引方式等配置
- 可选查看 `disabled_list` 禁删列表内容
- 配置修改后立即生效，无需重启

### -v, --version
```
del -v
```
- 显示当前工具版本号（来源于配置文件）

### -h, --help [command]
```
del -h
del -h list
del -h restore
```
- 无参数：显示本全局帮助
- 带子命令参数：显示对应命令的详细用法说明

### -- (参数分隔符)
```
del -- -file.txt
del -- ./special-name
```
- 显式分隔命令参数与文件名
- 用于处理以 `-` 开头或含特殊字符的文件名


> 提示: 所有命令均支持 -h <subcommand> 查看子命令专属帮助
"#;
pub fn help(argv: Option<&str>) {
    match argv {
        None => help_all(),
        Some("list") => print_md(list_help),
        Some("list-setting") => print_md(list_setting_help),
        Some("restore") => print_md(restore_help),
        Some("delete") => print_md(delete_help),
        Some("clear") => print_md(clear_help),
        Some("version") => print_md(version_help),
        Some("help") => print_md(help_help),
        Some("force") => print_md(force_help),
        Some("config") => print_md(config_help),
        Some("remove") => print_md(remove_help),
        _ => std::process::exit(1),
    }
}

pub fn help_all() {
    print_md(main_help);
}
