/*
Copyright (c) 2026 ywnh1
del is licensed under Mulan PSL v2.
You can use this software according to the terms and conditions of the Mulan
PSL v2.
You may obtain a copy of Mulan PSL v2 at:
         http://license.coscl.org.cn/MulanPSL2
THIS SOFTWARE IS PROVIDED ON AN "AS IS" BASIS, WITHOUT WARRANTIES OF ANY
KIND, EITHER EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED TO
NON-INFRINGEMENT, MERCHANTABILITY OR FIT FOR A PARTICULAR PURPOSE.
See the Mulan PSL v2 for more details.
*/
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
