[English](./README.md)

# Del

**Del** 是一个基于 Rust 开发的安全文件删除与回收站管理工具。  
与直接永久删除文件的 `rm` 不同，它将目标文件/目录压缩归档至指定回收站路径，并将元数据记录到 SQLite 数据库中，支持恢复、查询、清理等功能。

## 功能特性

- **安全删除** — 将文件/目录移入回收站归档，而非永久删除
- **交互式回收站** — TUI 模式（`-t`）浏览、搜索、恢复、删除回收站条目
- **自动清理** — 自动清除超过指定保存天数的归档（`-a`）
- **可配置压缩** — zstd 压缩级别 -5 到 22
- **路径黑名单** — 内置保护，防止删除系统关键目录
- **强制删除** — 绕过回收站直接调用系统 `rm`（`-f`）
- **仅归档不删除** — 将文件移入回收站但保留原文件（`-s`）
- **交互式配置编辑** — 通过 TUI 编辑设置（`-c`）
- **国际化支持** — 支持 9 种语言，自动适配系统区域设置

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
./del_x86_64 --help

# ARM64
chmod +x del_arm64
./del_arm64 --help
```

**安装 Deb 包：**

```bash
# x86_64
sudo dpkg -i del_0.3.2_amd64.deb

# ARM64
sudo dpkg -i del_0.3.2_arm64.deb
```

### 从 crates.io 安装

```bash
cargo install better-rm
```

### 从源码编译

```bash
cargo install --path .
```

## 快速开始

```bash
# 初始化是自动的，直接使用即可。

# 将文件或目录移入回收站
del file.txt directory/

# 交互式 TUI 浏览回收站
del -t

# 交互式编辑配置
del -c

# 自动清理过期归档
del -a

# 仅归档不删除原文件
del -s file.txt

# 强制删除（绕过回收站）
del -f file.txt
del -rf dir/

# 清空整个回收站（需要确认）
del -C
```

## 使用说明

### 命令行参数

| 参数 | 简写 | 说明 |
|------|------|------|
| `<path>` | — | 要删除的文件或目录路径（支持多个） |
| `--trash` | `-t` | 交互式 TUI 回收站浏览器和管理器 |
| `--config` | `-c` | 交互式配置编辑器 |
| `--save` | `-s` | 归档到回收站但不删除原文件 |
| `--recursive` | `-r` | 递归操作（仅与 `--force` 搭配） |
| `--force` | `-f` | 绕过回收站，直接调用系统 `rm` |
| `--autoclean` | `-a` | 自动清理超过保存天数的归档 |
| `--clear` | `-C` | 清空所有回收站归档（需要确认） |
| `--verbose` | `-v` | 显示详细日志 |
| `--trash-dir` | — | 临时指定回收站目录路径 |
| `--saving-days` | — | 临时指定备份保留天数 |
| `--add-disable` | — | 临时添加路径到禁用列表 |
| `--compression-level` | — | 临时指定 zstd 压缩级别（-5 到 22） |

**使用示例：**

```bash
# 删除文件（移入回收站）
del a.txt b/

# 归档到回收站，保留原文件
del -s important.txt

# 强制永久删除
del -f secret.txt

# 强制递归删除目录（相当于 rm -rf）
del -rf tempdir/

# 打开 TUI 回收站浏览器
del -t

# 打开交互式配置编辑器
del -c

# 自动清理过期归档
del -a

# 清空整个回收站（会要求确认）
del -C

# 带日志输出删除
del -v large_project/

# 临时覆盖配置单次运行
del --trash-dir /tmp/mytrash --saving-days 7 large_project/
del --compression-level 1 file.txt
del --add-disable /important/docs/
```

## 配置说明

配置文件位于 `~/.config/del/` 目录：

- **`config.toml`** — 用户设置
- **`trash.db`** — SQLite 数据库（自动创建）

交互式编辑配置：`del -c`

```toml
trash_dir = "~/.del_trash"
saving_days = 30
disable_list = ["/*", "~", ".", ".."]
compression_level = 3
```

- **`trash_dir`** — 归档文件的存储路径
- **`saving_days`** — 自动清理的天数阈值（配合 `-a` 使用）
- **`disable_list`** — 受保护免于删除的路径通配符列表
- **`compression_level`** — zstd 压缩级别（-5 到 22，默认 3）

## 国际化支持

Del 支持 9 种语言，语言自动从系统的 `LANG` 环境变量检测。

| 语言 | `LANG` 示例 |
|------|-------------|
| 英语 | `en_US.UTF-8` |
| 简体中文 | `zh_CN.UTF-8` |
| 繁体中文 | `zh_TW.UTF-8` |
| 日语 | `ja_JP.UTF-8` |
| 韩语 | `ko_KR.UTF-8` |
| 法语 | `fr_FR.UTF-8` |
| 西班牙语 | `es_ES.UTF-8` |
| 俄语 | `ru_RU.UTF-8` |
| 阿拉伯语 | `ar_SA.UTF-8` |

如需临时覆盖系统语言：

```bash
RUST_I18N_LOCALE=ja del -h
```

## 安全机制

- **路径黑名单**：系统关键目录默认受到保护
- **恢复安全**：TUI 模式（`-t`）恢复时拒绝覆盖已有文件
- **清空确认**：`-C` 需要交互式输入 `Y` 确认
- **符号链接解析**：所有路径在与黑名单匹配前都会解析为绝对路径

## 许可证

本项目采用 **木兰宽松许可证第2版（Mulan PSL v2）**。

版权所有 (c) 2026 ywnh1  
详见 [LICENSE](./LICENSE) 文件。
