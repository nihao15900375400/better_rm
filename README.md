[中文版](./README.zh-CN.md)

# Del

**Del** is a safe file deletion and trash management tool written in Rust.  
Instead of permanently removing files like `rm`, it archives them into a designated trash directory, records metadata in SQLite, and supports restore, query, and cleanup — giving you a safety net for everyday file deletion.

## Features

- **Safe deletion** — moves files/directories to a trash archive instead of permanent removal
- **Interactive trash browser** — TUI mode (`-t`) for browsing, searching, restoring, and deleting trash entries
- **Auto-cleanup** — purge archives older than a configurable number of days (`-a`)
- **Configurable compression** — zstd compression level from -5 to 22
- **Path blacklist** — built-in protection against deleting system-critical directories
- **Force delete** — bypass the trash and use `rm` directly (`-f`)
- **Save without deleting** — move files to trash while keeping the originals (`-s`)
- **Interactive config editor** — edit settings via TUI (`-c`)
- **Internationalization** — supports 9 languages, auto-detected from system locale

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
./del_x86_64 --help

# ARM64
chmod +x del_arm64
./del_arm64 --help
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
# Initialize is automatic — just start using it.

# Move files or directories to trash
del file.txt directory/

# Browse the trash interactively (TUI)
del -t

# Edit config interactively
del -c

# Auto-clean expired archives
del -a

# Save to trash without deleting originals
del -s file.txt

# Force delete (bypass trash)
del -f file.txt
del -rf dir/

# Clear all trash archives (requires confirmation)
del -C
```

## Usage

### Command-line options

| Option | Short | Description |
|--------|-------|-------------|
| `<path>` | — | Files or directories to delete (supports multiple paths) |
| `--trash` | `-t` | Interactive TUI trash browser and manager |
| `--config` | `-c` | Interactive config editor |
| `--save` | `-s` | Archive to trash without deleting original files |
| `--recursive` | `-r` | Recursive operation (only with `--force`) |
| `--force` | `-f` | Bypass trash, directly call system `rm` |
| `--autoclean` | `-a` | Auto-clean archives older than saving_days |
| `--clear` | `-C` | Clear all trash archives (with confirmation) |
| `--verbose` | `-v` | Show detailed logs |
| `--trash-dir` | — | Set trash directory for this run |
| `--saving-days` | — | Set backup retention days for this run |
| `--add-disable` | — | Add a path to the disable list for this run |
| `--compression-level` | — | Set zstd compression level (-5 to 22) for this run |

**Examples:**

```bash
# Delete files (move to trash)
del a.txt b/

# Save to trash without deleting
del -s important.txt

# Force permanently delete
del -f secret.txt

# Force recursively delete a directory (equivalent to rm -rf)
del -rf tempdir/

# Open the TUI trash browser
del -t

# Open the interactive config editor
del -c

# Auto-clean expired archives
del -a

# Clear the entire trash (requires confirmation)
del -C

# Delete with verbose logging
del -v large_project/

# Override config for a single run
del --trash-dir /tmp/mytrash --saving-days 7 large_project/
del --compression-level 1 file.txt
del --add-disable /important/docs/
```

## Configuration

Config files are stored under `~/.config/del/`:

- **`config.toml`** — user settings
- **`trash.db`** — SQLite database (auto-created)

Edit config interactively: `del -c`

```toml
trash_dir = "~/.del_trash"
saving_days = 30
disable_list = ["/*", "~", ".", ".."]
compression_level = 3
```

- **`trash_dir`** — where archived files are stored
- **`saving_days`** — auto-cleanup threshold (used by `-a`)
- **`disable_list`** — glob patterns of paths that are protected from deletion
- **`compression_level`** — zstd compression level (-5 to 22, default: 3)

## Internationalization

Del supports 9 languages. The language is automatically detected from the system `LANG` environment variable.

| Language | `LANG` Example |
|----------|---------------|
| English | `en_US.UTF-8` |
| Simplified Chinese | `zh_CN.UTF-8` |
| Traditional Chinese | `zh_TW.UTF-8` |
| Japanese | `ja_JP.UTF-8` |
| Korean | `ko_KR.UTF-8` |
| French | `fr_FR.UTF-8` |
| Spanish | `es_ES.UTF-8` |
| Russian | `ru_RU.UTF-8` |
| Arabic | `ar_SA.UTF-8` |

To override the system locale:

```bash
RUST_I18N_LOCALE=ja del -h
```

## Safety

- **Path blacklist**: system-critical directories are protected by default
- **Restore safety**: the TUI (`-t`) refuses to overwrite existing files when restoring
- **Clear confirmation**: `-C` requires interactive `Y` confirmation
- **Symlink resolution**: all paths are resolved to absolute paths before matching against the blacklist

## License

This project is licensed under **Mulan PSL v2**.

Copyright (c) 2026 ywnh1  
See the [LICENSE](./LICENSE) file for details.
