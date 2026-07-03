[中文版](./README.zh-CN.md)

# Del

**Del** is a safe file deletion and trash management tool written in Rust.  
Instead of permanently removing files like `rm`, it archives them into a designated trash directory, records metadata in SQLite, and supports restore, query, and cleanup — giving you a safety net for everyday file deletion.

## Features

- **Safe deletion** — moves files/directories to a trash archive instead of permanent removal
- **Restore** — recover deleted files by ID or the most recent entry
- **Trash management** — interactive TUI trash browser (powered by ratatui)
- **Query & search** — fuzzy search by filename, original path, time, size, and more
- **Auto-cleanup** — automatically purge archives older than a configurable number of days
- **Configurable compression** — choose compression level from -5 to 22
- **Path blacklist** — built-in protection against deleting system-critical directories
- **Force delete** — bypass the trash and use `rm` directly when needed
- **Multiple archive formats** — supports zstd, gz, bz2, xz2 compression via tar

## Installation

### Download from releases

| Package | Architecture | File |
|---------|-------------|------|
| Binary | x86_64 | `del_x86_64` (statically linked) |
| Binary | ARM64 | `del_arm64` (statically linked) |
| Deb | x86_64 | `del_0.3.2_amd64.deb` |
| Deb | ARM64 | `del_0.3.2_arm64.deb` |
| tar.xz | x86_64 | `del_0.3.2_x86_64.tar.xz` |
| tar.xz | ARM64 | `del_0.3.2_arm64.tar.xz` |

**Using the binary directly** (no dependencies required):

```bash
# x86_64
chmod +x del_x86_64
./del_x86_64 --init

# ARM64
chmod +x del_arm64
./del_arm64 --init
```

**Installing from deb:**

```bash
# x86_64
sudo dpkg -i del_0.3.2_amd64.deb

# ARM64
sudo dpkg -i del_0.3.2_arm64.deb
```

### Build from source

```bash
cargo install --path .
```

## Quick Start

```bash
# 1. Initialize config and database (required on first run)
del --init

# 2. Move files or directories to trash
del file.txt directory/

# 3. List trashed files
del --list

# 4. Restore the most recent deletion (or by ID)
del --undo
del --undo 1

# 5. Fuzzy search by filename
del --select "%.log"
```

## Usage

### Command-line options

| Option | Short | Description | Example |
|--------|-------|-------------|---------|
| `<path>` | — | Files or directories to delete (supports multiple paths) | `del a.txt b/` |
| `--init` | `-i` | Initialize trash directory and database | `del -i` |
| `--undo` | `-u` | Restore from trash by ID (default: most recent) | `del -u 3` |
| `--delete` | `-d` | Permanently delete trash records and archives by ID | `del -d 1 2` |
| `--trash` | `-t` | Interactive TUI trash browser and manager | `del -t` |
| `--recursive` | `-r` | Recursive operation (only with `--force`) | `del -rf dir/` |
| `--force` | `-f` | Bypass trash, directly call system `rm` | `del -f file.txt` |
| `--autoclean` | `-a` | Auto-clean archives older than saving_days | `del -a` |
| `--clear` | `-C` | Clear all trash archives | `del -C` |
| `--list` | `-l` | List all trash records in a paged view | `del -l` |
| `--select` | `-s` | Fuzzy search by filename | `del -s "report_%.doc"` |
| `--select-from` | — | Search by database field (name, id, time, original-path, size) | `del --select-from time "2024-05-%"` |
| `--verbose` | `-v` | Show detailed logs | `del -v file.txt` |
| `--config` | `-c` | Edit config with nano (or print config path) | `del -c` |

### Query syntax

`--select` and `--select-from` use SQLite `LIKE` operator under the hood:

- `%` — matches any sequence of characters (including zero)
- `_` — matches any single character
- `\` — escape character for literal `%` or `_`

**Examples:**
```bash
# Find all .txt files
del --select "%.txt"

# Find records from December 2024
del --select-from time "2024-12-%"

# Find records with underscore in original path
del --select-from original-path "%\_%"
```

## Configuration

Config file: `~/.config/del/config.toml`

```toml
trash_dir = "~/.del_trash"
saving_days = 30
disable_list = ["/*", "~", ".", ".."]
compression_level = 3
```

- **`trash_dir`** — where archived files are stored
- **`saving_days`** — auto-cleanup threshold (used by `--autoclean`)
- **`disable_list`** — glob patterns of paths that are protected from deletion
- **`compression_level`** — zstd compression level (-5 to 22, default: 3)

Use `del --config` to edit interactively.

## Safety

- **Path blacklist**: system-critical directories (`/`, `/home`, `/etc`, etc.) are protected by default
- **Restore safety**: `--undo` refuses to overwrite existing files
- **Force confirmation**: `--clear` requires interactive `Y` confirmation
- **Symlink resolution**: all paths are resolved to absolute paths before matching against the blacklist

## License

This project is licensed under **Mulan PSL v2**.

Copyright (c) 2026 ywnh1  
See the [LICENSE](./LICENSE) file for details.
