// Copyright (c) 2026 ywnh1
// del is licensed under Mulan PSL v2.
// You can use this software according to the terms and conditions of the Mulan PSL v2.
// You may obtain a copy of Mulan PSL v2 at:
//          http://license.coscl.org.cn/MulanPSL2
// THIS SOFTWARE IS PROVIDED ON AN "AS IS" BASIS, WITHOUT WARRANTIES OF ANY KIND,
// EITHER EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED TO NON-INFRINGEMENT,
// MERCHANTABILITY OR FIT FOR A PARTICULAR PURPOSE.
// See the Mulan PSL v2 for more details.

use anyhow::{Context, Result, bail};
use cliclack::{input, intro, log, outro, select};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

const DEFAULT_CONFIG: &str = r#"
compression_level = 7  # 压缩程度，0~22

trash = "~/.del_trash"  # 回收站目录路径，修改后可能导致以前的文件丢失

save_days = 30  # autoclean 中不自动删除的时间，0为永远不自动删除

disable_list = [
    "/",
    "/bin",
    "/sys",
    "/home",
    "/root",
    "~",
    ".",
    "..",
]
"#;

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct Config {
    pub compression_level: i32,
    pub trash: String,
    pub save_days: u16,
    pub disable_list: Vec<String>,
}

impl Config {
    /// 从文件读取配置，若文件不存在则创建默认配置并写入。
    /// 同时确保回收站目录存在，不存在则自动创建。
    pub fn new(path: &str) -> Result<Self> {
        let path = Path::new(path);
        if !path.is_file() {
            if let Some(parent) = path.parent() {
                fs::create_dir_all(parent)
                    .with_context(|| format!("无法创建配置目录: {}", parent.display()))?;
            }
            fs::write(path, DEFAULT_CONFIG)
                .with_context(|| format!("无法写入默认配置文件: {}", path.display()))?;
        }
        let text = fs::read_to_string(path)
            .with_context(|| format!("无法读取配置文件: {}", path.display()))?;
        let mut cfg = toml::from_str::<Config>(&text).with_context(|| "配置文件 TOML 格式错误")?;

        // 验证并修正压缩级别范围
        if !(0..=22).contains(&cfg.compression_level) {
            cfg.compression_level = 7;
        }

        // 展开路径中的 ~
        cfg.trash = expand_tilde(&cfg.trash);

        // 确保回收站目录存在
        ensure_trash_dir(&cfg.trash)?;

        Ok(cfg)
    }

    /// 将当前配置保存到文件
    pub fn save(&self, path: &str) -> Result<()> {
        let toml_str = toml::to_string_pretty(self).with_context(|| "序列化配置失败")?;
        fs::write(path, toml_str).with_context(|| format!("无法写入配置文件: {}", path))?;
        Ok(())
    }

    /// 交互式设置配置项，返回修改后的 Config（不自动写回文件）
    pub fn set(&self) -> Result<Self> {
        let mut cfg = self.clone();

        intro("配置设置")?;

        let result = loop {
            let choice = select("请选择一项进行设置")
                .items(&[
                    (SettingType::View, "查看当前配置", ""),
                    (SettingType::CompressionLevel, "压缩级别 (0~22)", ""),
                    (SettingType::Trash, "回收站路径", ""),
                    (SettingType::SaveDays, "自动清理保留天数", ""),
                    (SettingType::DisableList, "禁删列表", ""),
                    (SettingType::Save, "保存并退出", ""),
                    (SettingType::Cancel, "取消修改", ""),
                ])
                .filter_mode()
                .interact()?;

            match choice {
                SettingType::View => {
                    cfg.view();
                }
                SettingType::CompressionLevel => {
                    cfg = cfg.set_compression_level()?;
                }
                SettingType::Trash => {
                    cfg = cfg.set_trash()?;
                }
                SettingType::SaveDays => {
                    cfg = cfg.set_save_days()?;
                }
                SettingType::DisableList => {
                    cfg = cfg.set_disable_list()?;
                }
                SettingType::Save => break cfg,
                SettingType::Cancel => break self.clone(),
            }
        };

        outro("设置完成")?;
        Ok(result)
    }

    fn view(&self) {
        log::info(&format!("压缩级别: {}", self.compression_level)).ok();
        log::info(&format!("回收站路径: {}", self.trash)).ok();
        log::info(&format!("自动清理天数: {} 天", self.save_days)).ok();

        log::remark("禁删列表:").ok();
        if self.disable_list.is_empty() {
            log::remark("  (空)").ok();
        } else {
            for path in &self.disable_list {
                log::remark(&format!("  • {}", path)).ok();
            }
        }

        let _: Result<String, _> = input("按回车继续").required(false).interact();
    }

    fn set_compression_level(&self) -> Result<Self> {
        let lv: String = input("设定压缩等级 (0~22，留空保持当前值)")
            .placeholder(&format!("{}", self.compression_level))
            .validate(|input: &String| {
                if input.is_empty() {
                    return Ok(());
                }
                match input.trim().parse::<i32>() {
                    Ok(v) if (0..=22).contains(&v) => Ok(()),
                    Ok(_) => Err("数值超出范围，请输入 0~22 之间的整数"),
                    Err(_) => Err("请输入有效数字"),
                }
            })
            .interact()?;

        let mut new = self.clone();
        let trimmed = lv.trim();
        if !trimmed.is_empty() {
            new.compression_level = trimmed.parse::<i32>()?;
        }
        Ok(new)
    }

    fn set_trash(&self) -> Result<Self> {
        let path_val: String = input("设定回收站路径（留空保持当前值）")
            .placeholder(&self.trash)
            .validate(|input: &String| {
                if input.is_empty() {
                    return Ok(());
                }
                let expanded = expand_tilde(input);
                let p = Path::new(&expanded);
                // 如果路径已存在但不是目录则拒绝
                if p.exists() && !p.is_dir() {
                    return Err("该路径已存在但不是一个目录");
                }
                Ok(())
            })
            .interact()?;

        let mut new = self.clone();
        let trimmed = path_val.trim();
        if !trimmed.is_empty() {
            new.trash = expand_tilde(trimmed);
            // 自动创建回收站目录
            ensure_trash_dir(&new.trash)?;
            log::step(&format!("回收站目录已创建: {}", new.trash)).ok();
        }
        Ok(new)
    }

    fn set_save_days(&self) -> Result<Self> {
        let days: String = input("设定自动清理保留天数 (0 表示永不自动清理，留空保持当前值)")
            .placeholder(&format!("{}", self.save_days))
            .validate(|input: &String| {
                if input.is_empty() {
                    return Ok(());
                }
                match input.trim().parse::<u16>() {
                    Ok(_) => Ok(()),
                    Err(_) => Err("请输入有效数字 (0~65535)"),
                }
            })
            .interact()?;

        let mut new = self.clone();
        let trimmed = days.trim();
        if !trimmed.is_empty() {
            new.save_days = trimmed.parse::<u16>()?;
        }
        Ok(new)
    }

    fn set_disable_list(&self) -> Result<Self> {
        let mut new = self.clone();

        loop {
            log::remark("当前禁删列表:").ok();
            if new.disable_list.is_empty() {
                log::remark("  (空)").ok();
            } else {
                for path in &new.disable_list {
                    log::remark(&format!("  • {}", path)).ok();
                }
            }

            let action = select("禁删列表操作")
                .items(&[
                    (DisableListAction::Add, "添加路径", ""),
                    (DisableListAction::Remove, "删除路径", ""),
                    (DisableListAction::Back, "返回主菜单", ""),
                ])
                .filter_mode()
                .interact()?;

            match action {
                DisableListAction::Add => {
                    let p: String = input("输入要添加的路径")
                        .placeholder("/example/path")
                        .validate(|val: &String| {
                            if val.trim().is_empty() {
                                Err("路径不能为空")
                            } else {
                                Ok(())
                            }
                        })
                        .interact()?;
                    let path = p.trim().to_string();
                    if new.disable_list.contains(&path) {
                        log::warning("该路径已存在于列表中，未重复添加").ok();
                    } else {
                        new.disable_list.push(path.clone());
                        log::step(&format!("已添加: {}", path)).ok();
                    }
                    let _: String = input("按回车继续").required(false).interact()?;
                }
                DisableListAction::Remove => {
                    if new.disable_list.is_empty() {
                        log::remark("列表为空，没有可删除的项").ok();
                        let _: String = input("按回车继续").required(false).interact()?;
                        continue;
                    }

                    // 构建选项：每个路径 + 返回选项
                    let mut items: Vec<(usize, String, &str)> = new
                        .disable_list
                        .iter()
                        .enumerate()
                        .map(|(idx, p)| (idx, format!("删除: {}", p), ""))
                        .collect();
                    let back_idx = items.len();
                    items.push((back_idx, "← 返回".to_string(), ""));

                    let choice = select("选择要删除的路径").items(&items).interact()?;

                    if choice != back_idx {
                        let removed = new.disable_list.remove(choice);
                        log::step(&format!("已删除: {}", removed)).ok();
                    }
                    let _: String = input("按回车继续").required(false).interact()?;
                }
                DisableListAction::Back => break,
            }
        }

        Ok(new)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SettingType {
    View,
    CompressionLevel,
    Trash,
    SaveDays,
    DisableList,
    Save,
    Cancel,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum DisableListAction {
    Add,
    Remove,
    Back,
}

/// 展开路径开头的 `~` 为用户主目录
pub fn expand_tilde(path: &str) -> String {
    if let Some(rest) = path.strip_prefix("~/") {
        if let Ok(home) = std::env::var("HOME") {
            return format!("{}/{}", home, rest);
        }
    } else if path == "~" {
        if let Ok(home) = std::env::var("HOME") {
            return home;
        }
    }
    path.to_string()
}

/// 确保回收站目录存在：不存在则递归创建，存在但不是目录则报错
fn ensure_trash_dir(path: &str) -> Result<()> {
    let p = Path::new(path);
    if p.exists() {
        if !p.is_dir() {
            bail!("回收站路径已存在但不是一个目录: {}", path);
        }
    } else {
        fs::create_dir_all(p).with_context(|| format!("无法创建回收站目录: {}", path))?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;
    use std::fs;
    use std::path::PathBuf;

    fn temp_config_path(name: &str) -> PathBuf {
        let dir = env::temp_dir().join("del_config_test");
        fs::create_dir_all(&dir).ok();
        dir.join(name)
    }

    fn cleanup_config(path: &PathBuf) {
        let _ = fs::remove_file(path);
    }

    fn cleanup_trash_dir(path: &str) {
        let _ = fs::remove_dir_all(path);
    }

    #[test]
    fn test_expand_tilde_with_home() {
        let home = env::var("HOME").unwrap();
        assert_eq!(expand_tilde("~/test"), format!("{}/test", home));
        assert_eq!(expand_tilde("~/"), format!("{}/", home));
        assert_eq!(expand_tilde("~"), home);
    }

    #[test]
    fn test_expand_tilde_no_tilde() {
        assert_eq!(expand_tilde("/usr/bin"), "/usr/bin");
        assert_eq!(expand_tilde("relative/path"), "relative/path");
        assert_eq!(expand_tilde(""), "");
    }

    #[test]
    fn test_expand_tilde_tilde_in_middle() {
        // ~ 不在开头，不展开
        assert_eq!(expand_tilde("/home/~/test"), "/home/~/test");
    }

    #[test]
    fn test_new_config_creates_file_and_dir() {
        let path = temp_config_path("test_new.toml");
        cleanup_config(&path);

        let cfg = Config::new(path.to_str().unwrap()).unwrap();
        assert!(path.exists());
        assert_eq!(cfg.compression_level, 7);
        assert!(cfg.save_days == 30);
        assert!(!cfg.disable_list.is_empty());
        // ~ 应该被展开
        assert!(!cfg.trash.starts_with('~'));
        // 回收站目录应该存在
        assert!(Path::new(&cfg.trash).is_dir());

        cleanup_config(&path);
        cleanup_trash_dir(&cfg.trash);
    }

    #[test]
    fn test_new_config_clamps_compression_level() {
        let path = temp_config_path("test_clamp.toml");
        cleanup_config(&path);

        // 使用一个确定存在的目录作为 trash 路径
        let trash_dir = env::temp_dir().join("del_test_clamp_trash");
        let _ = fs::create_dir_all(&trash_dir);

        let bad_toml = format!(
            r#"
compression_level = 99
trash = "{}"
save_days = 30
disable_list = []
"#,
            trash_dir.display()
        );
        fs::write(&path, bad_toml).unwrap();
        let cfg = Config::new(path.to_str().unwrap()).unwrap();
        assert_eq!(cfg.compression_level, 7);

        cleanup_config(&path);
        cleanup_trash_dir(trash_dir.to_str().unwrap());
    }

    #[test]
    fn test_save_and_reload() {
        let path = temp_config_path("test_save.toml");
        cleanup_config(&path);

        let mut cfg = Config::new(path.to_str().unwrap()).unwrap();
        cfg.compression_level = 15;
        cfg.trash = "/tmp/del_test_save_trash".to_string();
        cfg.save_days = 90;
        cfg.disable_list = vec!["/a".into(), "/b".into()];

        // 创建 trash 目录以便 reload 时验证通过
        fs::create_dir_all(&cfg.trash).ok();
        cfg.save(path.to_str().unwrap()).unwrap();

        let reloaded = Config::new(path.to_str().unwrap()).unwrap();
        assert_eq!(reloaded.compression_level, 15);
        assert_eq!(reloaded.trash, "/tmp/del_test_save_trash");
        assert_eq!(reloaded.save_days, 90);
        assert_eq!(reloaded.disable_list, vec!["/a", "/b"]);

        cleanup_config(&path);
        cleanup_trash_dir(&cfg.trash);
    }

    #[test]
    fn test_default_config_structure() {
        let path = temp_config_path("test_default.toml");
        cleanup_config(&path);

        let cfg = Config::new(path.to_str().unwrap()).unwrap();

        // 默认禁删列表应包含关键系统路径
        assert!(cfg.disable_list.contains(&"/".to_string()));
        assert!(cfg.disable_list.contains(&"/home".to_string()));
        assert!(cfg.disable_list.contains(&"/bin".to_string()));
        assert!(cfg.disable_list.contains(&"/sys".to_string()));

        cleanup_config(&path);
        cleanup_trash_dir(&cfg.trash);
    }

    #[test]
    fn test_ensure_trash_dir_creates() {
        let dir = env::temp_dir().join("del_test_ensure_create");
        let _ = fs::remove_dir_all(&dir);

        assert!(!dir.exists());
        ensure_trash_dir(dir.to_str().unwrap()).unwrap();
        assert!(dir.is_dir());

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_ensure_trash_dir_already_exists() {
        let dir = env::temp_dir().join("del_test_ensure_exists");
        fs::create_dir_all(&dir).unwrap();

        // 已存在目录，应无错误
        ensure_trash_dir(dir.to_str().unwrap()).unwrap();

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_ensure_trash_dir_is_file() {
        let file = env::temp_dir().join("del_test_ensure_file");
        fs::write(&file, "not a dir").unwrap();

        // 已存在文件，应报错
        let result = ensure_trash_dir(file.to_str().unwrap());
        assert!(result.is_err());

        let _ = fs::remove_file(&file);
    }
}
