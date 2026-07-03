use crate::consts::{CONFIG_PATH, TRASH_PATH};
use crate::util::*;
use anyhow::{Context, Result};
use cliclack::*;
use figment::{
    Figment,
    providers::{Env, Format, Serialized, Toml},
};
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
        .with_context(|| "Errors when loading config files")
}
pub fn edit_config() -> Result<()> {
    let mut cfg = Figment::new()
        .merge(Serialized::defaults(Config::default()))
        .merge(Toml::file(to_absolute_no_fs(CONFIG_PATH)))
        .extract::<Config>()?;
    intro(format!("Editing {CONFIG_PATH}"))?;
    loop {
        let opt = select("Start Out")
            .items(&[
                ("show", "Show", "View present config"),
                ("trash_dir", "Edit Trash Dir", ""),
                ("saving_days", "Edit Saving Days", ""),
                ("disable_list", "Edit Disable List", ""),
                ("compression_level", "Edit Compression Level", ""),
                ("reset", "Reset to Default", "Restore default config"),
                ("exit", "Exit", ""),
            ])
            .filter_mode()
            .interact()?;
        match opt {
            "show" => note(
                "Config",
                format!(
                    "trash_dir:{}\nsaving_days:{}\ndisable_list:{:?}\ncompression_level:{}",
                    cfg.trash_dir, cfg.saving_days, cfg.disable_list, cfg.compression_level,
                ),
            )?,
            "trash_dir" => {
                cfg.trash_dir = to_absolute_no_fs(
                    &input("Edit Trash Dir")
                        .placeholder(&cfg.trash_dir)
                        .default_input(&cfg.trash_dir)
                        .validate(|p: &String| {
                            if std::path::Path::new(p.as_str()).is_dir() {
                                Ok(())
                            } else {
                                Err("No such dir exists")
                            }
                        })
                        .interact::<String>()?,
                )
                .display()
                .to_string();
            }
            "compression_level" => {
                cfg.compression_level = input("Edit Compression Level")
                    .placeholder("-5 ~ 22")
                    .default_input(cfg.compression_level.to_string().as_str())
                    .validate(|num: &String| match num.trim().parse::<i32>() {
                        Ok(n) => {
                            if (-5..=22).contains(&n) {
                                Ok(())
                            } else {
                                Err("Please input a number between -5 and 22")
                            }
                        }
                        Err(_) => Err("Please input a number between -5 and 22"),
                    })
                    .interact()?;
            }
            "disable_list" => loop {
                let sub = select("Disable List")
                    .items(&[
                        ("show", "Show List", ""),
                        ("add", "Add Path", ""),
                        ("remove", "Remove Path", ""),
                        ("back", "Back", ""),
                    ])
                    .interact()?;
                match sub {
                    "show" => {
                        note("Disable List", format!("{:#?}", cfg.disable_list))?;
                    }
                    "add" => {
                        let p = input("Path to add").interact::<String>()?;
                        cfg.disable_list.push(p);
                    }
                    "remove" => {
                        if cfg.disable_list.is_empty() {
                            log::warning("List is empty")?;
                        } else {
                            let list_len = cfg.disable_list.len();
                            let idx: usize =
                                input(format!("Index to remove (0~{}):", list_len - 1))
                                    .validate(move |s: &String| match s.trim().parse::<usize>() {
                                        Ok(i) if i < list_len => Ok(()),
                                        _ => Err("Invalid index"),
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
                cfg.saving_days = input("Edit Saving Days")
                    .placeholder("30")
                    .default_input(cfg.saving_days.to_string().as_str())
                    .validate(|num: &String| match num.trim().parse::<u16>() {
                        Ok(_) => Ok(()),
                        Err(_) => Err("Please input a valid number"),
                    })
                    .interact()?;
            }
            "reset" => {
                cfg = Config::default();

                let toml_str = toml::to_string_pretty(&cfg)?;
                fs::write(to_absolute_no_fs(CONFIG_PATH), toml_str)?;
                note("Config reset", "Config has been restored to default values")?;
            }
            "exit" => break,
            _ => unreachable!(),
        }
    }

    let toml_str = toml::to_string_pretty(&cfg)?;
    fs::write(to_absolute_no_fs(CONFIG_PATH), toml_str)?;
    outro("Config saved")?;
    Ok(())
}
