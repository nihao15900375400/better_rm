[English](./README.md)

# Del

**Del** 是一个基于 Rust 开发的安全文件删除与回收站管理工具。  
与直接永久删除文件的 `rm` 不同，它将目标文件/目录压缩归档至指定回收站路径，并将元数据记录到 SQLite 数据库中，支持误删恢复、条件查询、安全清空与永久删除等功能。

## 功能特性

- **安全删除** — 将文件/目录移入回收站归档，而非永久删除
- **恢复功能** — 按 ID 或恢复最近一次删除的文件
- **回收站管理** — 交互式 TUI 回收站浏览器（基于 ratatui）
- **查询搜索** — 按文件名、原始路径、时间、大小等进行模糊搜索
- **自动清理** — 自动清除超过指定保存天数的归档文件
- **可配置压缩** — 支持 -5 到 22 级的压缩级别选择
- **路径黑名单** — 内置保护，防止删除系统关键目录
- **强制删除** — 在需要时绕过回收站直接调用系统 `rm`
- **多种归档格式** — 通过 tar 支持 zstd、gz、bz2、xz2 压缩

## 安装方式

### 从 Release 下载

| 类型 | 架构 | 文件 |
|------|------|------|
| 二进制 | x86_64 | `del_x86_64`（静态链接） |
| 二进制 | ARM64 | `del_arm64`（静态链接） |
| Deb 包 | x86_64 | `del_0.3.2_amd64.deb` |
| Deb 包 | ARM64 | `del_0.3.2_arm64.deb` |
| tar.xz | x86_64 | `del_0.3.2_x86_64.tar.xz` |
| tar.xz | ARM64 | `del_0.3.2_arm64.tar.xz` |

**直接使用二进制**（无需安装依赖）：

```bash
# x86_64
chmod +x del_x86_64
./del_x86_64 --init

# ARM64
chmod +x del_arm64
./del_arm64 --init
```

**安装 Deb 包：**

```bash
# x86_64
sudo dpkg -i del_0.3.2_amd64.deb

# ARM64
sudo dpkg -i del_0.3.2_arm64.deb
```

### 从源码编译

```bash
cargo install --path .
```

## 快速开始

```bash
# 1. 初始化配置文件与数据库（首次运行必需）
del --init

# 2. 将文件或目录移入回收站
del file.txt directory/

# 3. 查看回收站已删除的文件列表
del --list

# 4. 恢复最近删除的文件（或指定 ID）
del --undo
del --undo 1

# 5. 根据名称模糊搜索回收站文件
del --select "%.log"
```

## 使用说明

### 命令行参数

| 参数 | 简写 | 说明 | 示例 |
|------|------|------|------|
| `<path>` | — | 要删除的文件或目录路径（支持多个） | `del a.txt b/` |
| `--init` | `-i` | 初始化回收站目录与数据库 | `del -i` |
| `--undo` | `-u` | 按 ID 从回收站恢复（默认最近一次） | `del -u 3` |
| `--delete` | `-d` | 按 ID 永久删除回收站记录及归档 | `del -d 1 2` |
| `--trash` | `-t` | 交互式 TUI 回收站浏览器和管理器 | `del -t` |
| `--recursive` | `-r` | 递归操作（仅与 `--force` 搭配） | `del -rf dir/` |
| `--force` | `-f` | 绕过回收站，直接调用系统 `rm` | `del -f file.txt` |
| `--autoclean` | `-a` | 自动清理超过保存天数的归档 | `del -a` |
| `--clear` | `-C` | 清空所有回收站归档 | `del -C` |
| `--list` | `-l` | 分页列出所有回收站记录 | `del -l` |
| `--select` | `-s` | 按文件名模糊搜索 | `del -s "report_%.doc"` |
| `--select-from` | — | 按数据库字段搜索（name、id、time、original-path、size） | `del --select-from time "2024-05-%"` |
| `--verbose` | `-v` | 显示详细日志 | `del -v file.txt` |
| `--config` | `-c` | 使用 nano 编辑配置（或打印配置路径） | `del -c` |

### 查询语法

`--select` 和 `--select-from` 底层基于 SQLite 的 `LIKE` 操作符：

- `%` — 匹配任意长度的字符序列（包含零个字符）
- `_` — 匹配任意单个字符
- `\` — 转义字符，用于将 `%` 或 `_` 作为普通字符匹配

**示例：**
```bash
# 查找所有 .txt 结尾的文件
del --select "%.txt"

# 查找 2024 年 12 月的删除记录
del --select-from time "2024-12-%"

# 查找原始路径中包含下划线的记录
del --select-from original-path "%\_%"
```

## 配置说明

配置文件位于：`~/.config/del/config.toml`

```toml
trash_dir = "~/.del_trash"
saving_days = 30
disable_list = ["/*", "~", ".", ".."]
compression_level = 3
```

- **`trash_dir`** — 归档文件的存储路径
- **`saving_days`** — 自动清理的天数阈值（由 `--autoclean` 使用）
- **`disable_list`** — 受保护免于删除的路径通配符列表
- **`compression_level`** — zstd 压缩级别（-5 到 22，默认 3）

使用 `del --config` 可进行交互式编辑。

## 安全机制

- **路径黑名单**：系统关键目录（`/`、`/home`、`/etc` 等）默认受到保护
- **恢复安全**：`--undo` 拒绝覆盖已有文件
- **强制确认**：`--clear` 需要交互式输入 `Y` 确认
- **符号链接解析**：所有路径在与黑名单匹配前都会解析为绝对路径

## 许可证

本项目采用 **木兰宽松许可证第2版（Mulan PSL v2）**。

版权所有 (c) 2026 ywnh1  
详见 [LICENSE](./LICENSE) 文件。
