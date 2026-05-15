pub const CONFIG: &str = "~/.config/del/config.json";
pub const DATABASE: &str = "~/.config/del/trash.db";
pub const CONFIG_JSON_DATA: &str = r#"{
    "_comment_trash": "Path of recycle station, use absolute path.",
    "_comment_archive_tool": "Choose one from: xz2 / zstd / bz2 / gz",
    "_comment_disable_list": "Prohibit deletion for specified directories or files, fill in absolute path only. Common protected system paths are preset below.",

    "trash": "~/.del_trash",
    "archive_tool": "gz",
    "disable_list": [
        "/",
        "/home",
        "/root",
        "/etc",
        "/usr",
        "/var",
        "/boot",
        "/lib",
        "/lib64",
        "/proc",
        "/sys",
        "/dev",
        "/tmp",
        ".",
        "..",
        "~"
    ]
}"#;
