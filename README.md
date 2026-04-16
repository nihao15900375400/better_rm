# del

## 项目概述

`del` 是一款基于 Rust 编写的命令行文件回收站管理工具。与传统 `rm` 命令的直接删除机制不同，`del` 采用“压缩归档 + 元数据索引 + 安全恢复”的工作流，将目标文件或目录打包至独立回收站路径，并持久化记录原始位置、体积、时间戳与校验标识。工具内置系统级防误删保护、多算法压缩支持、终端富文本交互与智能配置查找机制，适用于开发者、系统管理员及终端重度用户的安全文件操作场景。

---

## 核心特性

| 特性维度 | 技术实现与说明 |
|:---|:---|
| **安全归档工作流** | 文件压缩成功后自动移除原始路径，归档文件以唯一标识命名，避免覆盖与冲突。 |
| **多算法压缩支持** | 底层基于 `tar` 归档，通过编译特性可选 `gzip`, `xz`, `bzip2`, `zstd`, `lz4` 等算法。 |
| **唯一标识索引** | 支持 `BLAKE3` 内容哈希或 `UUID4` 随机标识，确保归档文件名全局唯一。 |
| **自动化元数据管理** | 归档信息结构化存储于 `.information.json`，支持按名称精确查询、批量列表展示与记录清理。 |
| **系统级防误删保护** | 内置关键路径 `HashSet` 黑名单，拦截高危操作；提供 `--force` 强制绕过机制并触发二次确认。 |
| **终端富交互体验** | 集成 `termimad` 渲染 Markdown 帮助文档，`tabled` 生成结构化数据表格，`colored` 提供语义化彩色输出。 |
| **智能配置定位** | 从当前工作目录逐级向上搜索至根目录，优先加载首个 `config.json`，支持项目级与环境级配置隔离。 |

---

## 架构与设计亮点

1. **模块化职责分离**
   - `main.rs`：命令参数解析、路由分发与核心业务逻辑编排。
   - `pack.rs`：压缩/解压引擎、流式哈希计算、RAII 临时文件守卫与错误类型定义。
   - `utils.rs`：配置解析、JSON 索引读写、路径查找算法与终端格式化辅助。
   - `help.rs`：内嵌 Markdown 帮助文本，支持子命令级详情展示。

2. **I/O 与内存优化**
   - 采用 `BufWriter` 缓冲写入降低系统调用开销。
   - `WriteCounter` 实时统计压缩后体积，避免二次 `metadata` 查询。
   - `TeeWriter` 实现流式数据分叉，在写入压缩流的同时同步计算 `BLAKE3` 哈希，零额外内存拷贝。

3. **异常安全机制**
   - 引入 `TempFileGuard` RAII 守卫，确保压缩过程中断或发生错误时自动清理残留的 `.tmp_*` 临时文件。
   - 错误类型通过 `thiserror` 宏统一封装，携带明确的路径上下文与操作阶段信息，便于故障排查。

4. **数据一致性模型**
   - 元数据索引采用全量读取-追加-覆盖写入模型。当前为同步单线程设计，适用于常规交互式操作场景。

---

## 安装与构建

### 环境要求
- Rust 工具链：`rustc >= 1.60`，配套 `cargo`
- 目标平台：Linux / macOS / Windows (需兼容终端环境)

### 编译指南
```bash
# 克隆仓库
git clone <repository_url>
cd del

# 基础构建 (仅启用标准 tar 支持)
cargo build --release

# 全特性构建 (启用所有压缩算法)
cargo build --release --features gz,xz,bz2,zstd,lz4

# 验证安装
./target/release/del --version
```
构建完成后，可将二进制文件链接至 `$PATH` 目录（如 `/usr/local/bin/del`）以实现全局调用。

---

## 配置管理

### 查找策略
程序启动时调用 `find_config_upwards`，从 `std::env::current_dir()` 开始逐级向上遍历父目录，直至根目录。返回首个匹配的 `config.json`。若全程未找到，将在当前运行目录自动生成默认配置并初始化回收站结构。

### 默认配置结构
```json
{
  "version": "0.1.0",
  "author": "ywnh1",
  "compression_tool": "tar",
  "index_tool": "uuid4",
  "recycle": "/opt/del/.recycle",
  "options_supported": {
    "compression_tool": ["tar", "tar.gz", "tgz", "tar.xz", "txz", "tar.bz2", "tbz2", "tar.zst", "tzst", "tar.lz4", "tlz4"],
    "index_tool": ["blake3", "uuid4"]
  },
  "disabled_list": [
    "/etc", "/usr", "/bin", "/sbin", "/lib", "/lib64", "/boot",
    "/usr/bin/sudo", "/etc/passwd", "/etc/shadow", "..."
  ]
}
```

### 核心字段说明
| 字段 | 类型 | 作用 |
|:---|:---|:---|
| `recycle` | `String` | 回收站物理根目录，归档文件与索引文件均存放于此。 |
| `compression_tool` | `String` | 默认压缩格式标识，需与编译特性匹配。 |
| `index_tool` | `String` | 归档命名策略：`blake3` (内容哈希) 或 `uuid4` (随机标识)。 |
| `disabled_list` | `HashSet<String>` | 受保护路径/文件名集合，O(1) 时间复杂度拦截默认删除操作。 |

---

## 命令行参考

### 命令速查表
| 命令 / 参数 | 长格式 | 功能说明 |
|:---|:---|:---|
| `del <path>` | 无 | 默认行为：压缩文件/目录至回收站，移除原始路径。 |
| `del -l` | `--list` | 列表模式：表格展示所有归档记录。支持附加名称参数查看单条详情。 |
| `del -r` | `--restore` | 恢复模式：按名称解压归档至原始父目录，完成后清除索引记录。 |
| `del -d` | `--delete` | 删除模式：永久移除指定归档文件及其索引记录。不可逆。 |
| `del -c` | `--clear` | 清空模式：递归删除整个回收站目录，重建空索引。需二次确认。 |
| `del -f` | `--force` | 强制模式：绕过 `disabled_list` 拦截，强制添加受保护文件至回收站。 |
| `del --config` | 无 | 显示当前生效的 `config.json` 绝对路径并输出文件内容。 |
| `del --list-setting` | 无 | 打印运行时配置摘要，并可选择性展示完整禁删列表。 |
| `del -v` | `--version` | 显示工具版本号（读取自配置文件）。 |
| `del -h` | `--help` | 帮助系统：无参数显示全局概览，附加子命令名显示详细用法。 |
| `del --` | 无 | 参数分隔符：显式终止选项解析，用于处理以 `-` 开头的特殊文件名。 |

### 典型使用示例
```bash
# 1. 基础归档
del project_log.txt
del src/ tests/

# 2. 查看与检索
del -l                      # 列表展示
del -l project_log          # 查看指定归档元数据

# 3. 恢复与清理
del -r project_log          # 恢复至原始路径
del -d project_log          # 永久删除该归档
del -c                      # 清空整个回收站

# 4. 强制与安全操作
del -f /etc/sudoers         # 强制归档受保护文件 (触发 y/n 确认)
del -- -critical_file.txt   # 处理以连字符开头的文件名
```

---

## 编译特性控制

压缩算法的启用受 Rust 条件编译控制。若需支持特定格式，需在 `Cargo.toml` 中声明依赖并映射特性：

```toml
[dependencies]
flate2 = { version = "1.0", features = ["zlib"] }
xz2      = "0.1"
bzip2    = "0.4"
zstd     = "0.13"
lz4      = "1.26"

[features]
gz   = ["flate2"]
xz   = ["xz2"]
bz2  = ["bzip2"]
zstd = ["zstd"]
lz4  = ["lz4"]
```
运行时若调用未启用特性的压缩格式，底层将返回 `UnsupportedCompression` 错误。解压逻辑同样通过 `#[cfg(feature = "...")]` 进行分支控制，确保二进制体积与功能按需匹配。

---

## 注意事项与最佳实践

1. **原始文件行为**：代码中 `CompressionConfig::delete_original` 默认值为 `true`。归档成功后，原始文件或目录将被自动移除。若需保留原文件，需在代码层面调整该配置或通过其他工具联动。
2. **权限与路径规划**：默认回收站路径 `/opt/del/.recycle` 通常需 `root` 权限写入。普通用户运行前，建议修改 `config.json` 中的 `recycle` 字段至用户目录（如 `~/.local/share/del/recycle`）。
3. **恢复路径约束**：`--restore` 操作严格遵循归档时记录的原始父目录。若原始路径已被删除或权限变更，解压可能失败。执行前请确认目标环境状态。
4. **索引一致性**：`.information.json` 采用全量覆盖写入。高频并发或脚本批量调用可能引发竞态条件。当前版本定位为交互式 CLI 工具，不建议在无人值守的高并发流水线中直接使用。
5. **外部命令依赖**：`del --config` 依赖系统 `cat` 命令输出配置内容。在纯 Windows 环境或极简容器中，需确保兼容终端或替换实现逻辑。
6. **错误处理机制**：核心操作在失败时会调用 `process::exit(1)` 或触发 `unwrap()` 终止。在自动化脚本中调用时，建议通过 `$?` 检查退出码以判断执行状态。

---

## 许可证

本项目遵循 **Mulan Permissive Software License, Version 2 (Mulan PSL v2)** 开源协议。
Copyright (c) 2026 ywnh1. 完整条款请参阅：[Mulan PSL v2](http://license.coscl.org.cn/MulanPSL2)

> 本工具按“原样”提供，不提供任何形式的明示或暗示担保。使用者需自行承担操作风险，建议在执行清空或强制删除前验证数据备份状态。