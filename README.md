# Del

一款基于 Rust 的安全文件删除与回收站管理工具。

## 项目简介

`del` 提供了比系统原生 `rm` 更安全的文件删除方案 —— 将目标文件/目录压缩归档至指定回收站路径，并记录元数据至 SQLite 数据库，通过交互模式实现误删恢复、永久删除、安全清空等功能。

## 快速开始

```bash
# 编译
cargo build --release

# 删除文件或目录（移入回收站）
del file.txt directory/

# 交互式管理模式（恢复、永久删除、清空等）
del -i

# 强制永久删除（绕过回收站）
del -f file.txt

# 递归强制删除
del -rf directory/

# 修改配置文件
del -c

# 清空回收站
del -C

# 自动清理过期文件
del -a
```

## 命令行参数

| 参数 | 简写 | 说明 |
|:---|:---|:---|
| `<path>...` | — | 要删除的文件或目录路径（移入回收站） |
| `--interact` | `-i` | 进入交互式管理模式（恢复、永久删除、修改配置、清空） |
| `--force` | `-f` | 直接永久删除，不受禁删列表限制 |
| `--recursive` | `-r` | 仅与 `-f` 连用，递归强制删除目录 |
| `--config` | `-c` | 交互式修改配置文件 |
| `--clear` | `-C` | 清空回收站（不可恢复） |
| `--autoclean` | `-a` | 自动清理超过 `save_days` 天的回收站记录 |

## 交互模式

通过 `del -i` 进入，提供以下操作：

- **永久删除某项** — TUI 多选表格，勾选后从回收站永久删除
- **恢复某项** — TUI 多选表格，勾选后恢复到原路径或指定目录
- **修改配置文件** — 交互式设置压缩级别、回收站路径、禁删列表等
- **清空 Trash** — 删除数据库及全部归档（不可恢复）

## 配置文件

配置文件位于 `~/.config/del/config.toml`：

```toml
compression_level = 7   # zstd 压缩级别 (0~22)
trash = "~/.del_trash"  # 回收站目录
save_days = 30          # autoclean 保留天数（0 = 永不清除）
disable_list = [        # 禁删路径保护
    "/", "/bin", "/sys",
    "/home", "/root",
    "~", ".", "..",
]
```

## 安全机制

- **禁删列表** (`disable_list`)：精确匹配拦截，防止误删系统关键路径。
- **回收站归档**：删除时自动压缩归档至回收站，文件可恢复。
- **路径展开**：`~` 和相对路径统一转换为绝对路径。
- **覆盖检测**：恢复时若目标路径已存在同名文件，中止操作避免覆盖。

## 项目结构

```
rust_del/
├── Cargo.toml          # 根 crate (del)
├── LICENSE             # 木兰宽松许可证 v2
├── src/
│   ├── main.rs         # 入口与主要逻辑
│   ├── args.rs         # 命令行参数定义 (clap)
│   ├── pack.rs         # tar.zstd 压缩/解压与哈希计算
│   └── sql.rs          # SQLite 数据库操作
├── config/             # 配置管理子 crate
│   └── src/lib.rs
└── multi_select/       # TUI 多选表格子 crate
    └── src/
        ├── lib.rs      # 多选表格核心实现
        └── main.rs     # 演示入口
```

## 技术栈

- **Rust** 2024 Edition
- **tokio** 异步运行时
- **clap** 命令行参数解析
- **sqlx** (SQLite) 元数据存储
- **tar + zstd** 压缩归档
- **BLAKE3** 内容哈希
- **ratatui + crossterm** TUI 多选表格
- **cliclack + dialoguer** 交互式 CLI

## 许可证

本项目基于 [木兰宽松许可证 v2 (Mulan PSL v2)](http://license.coscl.org.cn/MulanPSL2) 开源。

```
Copyright (c) 2026 ywnh1
del is licensed under Mulan PSL v2.
```
