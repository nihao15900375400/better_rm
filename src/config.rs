use crate::consts::{CONFIG_PATH, TRASH_PATH};
use crate::util::*;
use crate::t_str;
use anyhow::{Context, Result};
use cliclack::*;
use figment::{
    Figment,
    providers::{Env, Format, Serialized, Toml},
};
use rust_i18n::t;
use serde::{Deserialize, Serialize};
use std::fs;

#[derive(Deserialize, Serialize, Debug)]
pub struct Config {
    pub trash_dir: String,
    pub saving_days: u16,
    pub disable_list: Vec<String>,
    pub compression_level: i32,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            trash_dir: to_absolute_no_fs(TRASH_PATH)
                .into_os_string()
                .into_string()
                .unwrap_or_default(),
            saving_days: 30,
            disable_list: vec!["/*".into(), "~".into(), ".".into(), "..".into()],
            compression_level: 3,
        }
    }
}

pub fn load_config() -> Result<Config> {
    Figment::new()
        .merge(Serialized::defaults(Config::default()))
        .merge(Toml::file(to_absolute_no_fs(CONFIG_PATH)))
        .merge(Env::prefixed("APP_").split("_"))
        .extract::<Config>()
        .with_context(|| t!("config.load_error"))
}

pub fn edit_config() -> Result<()> {
    let mut cfg = Figment::new()
        .merge(Serialized::defaults(Config::default()))
        .merge(Toml::file(to_absolute_no_fs(CONFIG_PATH)))
        .extract::<Config>()?;
    intro(t!("config.editing", path = CONFIG_PATH))?;
    loop {
        let menu_title = t!("config.menu_title");
        let show_label = t!("config.show_label");
        let show_desc = t!("config.show_desc");
        let edit_trash = t!("config.edit_trash_dir");
        let edit_days = t!("config.edit_days");
        let edit_disable = t!("config.edit_disable");
        let edit_compress = t!("config.edit_compress");
        let reset_label = t!("config.reset_label");
        let reset_desc = t!("config.reset_desc");
        let exit_label = t!("config.exit_label");
        let exit_desc = t!("config.exit_desc");

        let opt = select(&menu_title)
            .items(&[
                ("show", show_label.as_ref(), show_desc.as_ref()),
                ("trash_dir", edit_trash.as_ref(), ""),
                ("saving_days", edit_days.as_ref(), ""),
                ("disable_list", edit_disable.as_ref(), ""),
                ("compression_level", edit_compress.as_ref(), ""),
                ("reset", reset_label.as_ref(), reset_desc.as_ref()),
                ("exit", exit_label.as_ref(), exit_desc.as_ref()),
            ])
            .filter_mode()
            .interact()?;
        match opt {
            "show" => {
                let note_title = t!("config.note_title");
                note(
                    &note_title,
                    format!(
                        "trash_dir:{}\nsaving_days:{}\ndisable_list:{:?}\ncompression_level:{}",
                        cfg.trash_dir, cfg.saving_days, cfg.disable_list, cfg.compression_level,
                    ),
                )?;
            }
            "trash_dir" => {
                let prompt = t!("config.edit_trash_dir");
                cfg.trash_dir = to_absolute_no_fs(
                    &input(&prompt)
                        .placeholder(&cfg.trash_dir)
                        .default_input(&cfg.trash_dir)
                        .validate(|p: &String| {
                            if std::path::Path::new(p).is_dir() {
                                Ok(())
                            } else {
                                Err(t_str!("config.no_dir"))
                            }
                        })
                        .interact::<String>()?,
                )
                .display()
                .to_string();
            }
            "compression_level" => {
                let prompt = t!("config.edit_compress");
                cfg.compression_level = input(&prompt)
                    .placeholder("-5 ~ 22")
                    .default_input(&cfg.compression_level.to_string())
                    .validate(|num: &String| match num.trim().parse::<i32>() {
                        Ok(n) => {
                            if (-5..=22).contains(&n) {
                                Ok(())
                            } else {
                                Err(t_str!("config.invalid_compress"))
                            }
                        }
                        Err(_) => Err(t_str!("config.invalid_compress")),
                    })
                    .interact()?;
            }
            "disable_list" => loop {
                let sub_title = t!("config.edit_disable");
                let show_list = t!("config.show_list");
                let add_path = t!("config.add_path");
                let rem_path = t!("config.rem_path");
                let back = t!("config.back");
                let sub = select(&sub_title)
                    .items(&[
                        ("show", show_list.as_ref(), ""),
                        ("add", add_path.as_ref(), ""),
                        ("remove", rem_path.as_ref(), ""),
                        ("back", back.as_ref(), ""),
                    ])
                    .interact()?;
                match sub {
                    "show" => {
                        let note_title = t!("config.note_title");
                        note(&note_title, format!("{:#?}", cfg.disable_list))?;
                    }
                    "add" => {
                        let prompt = t!("config.path_prompt");
                        let p = input(&prompt).interact::<String>()?;
                        cfg.disable_list.push(p);
                    }
                    "remove" => {
                        if cfg.disable_list.is_empty() {
                            let empty_msg = t!("config.empty_list");
                            log::warning(&empty_msg)?;
                        } else {
                            let list_len = cfg.disable_list.len();
                            let prompt = t!("config.index_prompt", max = list_len - 1);
                            let idx: usize = input(&prompt)
                                .validate(move |s: &String| match s.trim().parse::<usize>() {
                                    Ok(i) if i < list_len => Ok(()),
                                    _ => Err(t_str!("config.bad_index")),
                                })
                                .interact()?;
                            cfg.disable_list.remove(idx);
                        }
                    }
                    "back" => break,
                    _ => unreachable!(),
                }
            },
            "saving_days" => {
                let prompt = t!("config.edit_days");
                cfg.saving_days = input(&prompt)
                    .placeholder("30")
                    .default_input(&cfg.saving_days.to_string())
                    .validate(|num: &String| match num.trim().parse::<u16>() {
                        Ok(_) => Ok(()),
                        Err(_) => Err(t_str!("config.invalid_num")),
                    })
                    .interact()?;
            }
            "reset" => {
                cfg = Config::default();

                let toml_str = toml::to_string_pretty(&cfg)?;
                fs::write(to_absolute_no_fs(CONFIG_PATH), toml_str)?;
                let reset_title = t!("config.reset_title");
                let reset_body = t!("config.reset_body");
                note(&reset_title, &reset_body)?;
            }
            "exit" => break,
            _ => unreachable!(),
        }
    }

    let toml_str = toml::to_string_pretty(&cfg)?;
    fs::write(to_absolute_no_fs(CONFIG_PATH), toml_str)?;
    outro(t!("config.saved"))?;
    Ok(())
}
