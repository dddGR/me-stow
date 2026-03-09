/* ===================================================== */
/* CONFIGURATIONS READER =============================== */

use dialoguer::{Confirm, Input, theme::ColorfulTheme};
use me_stow::error::ErrType;
use std::fs::File;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};

#[derive(Debug, serde::Deserialize, serde::Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Resolver {
    Adopt,
    Replace,
}

#[derive(Debug, serde::Deserialize, serde::Serialize)]
pub struct Config {
    /// Path to store stower packages
    path_source: PathBuf,
    /// Path to act as root for stow package, usually $HOME
    path_root: PathBuf,
    /// Method to resolve conflict
    resolver: Resolver,
}

impl Default for Config {
    fn default() -> Self {
        let source: String = Input::with_theme(&ColorfulTheme::default())
            .with_prompt("Input source directory: ")
            .interact_text()
            .unwrap();
        let root: String = Input::with_theme(&ColorfulTheme::default())
            .with_prompt("Input root directory (default: $HOME): ")
            .default(dirs::home_dir().unwrap().to_string_lossy().to_string())
            .interact_text()
            .unwrap();
        Self {
            path_source: PathBuf::from(source),
            path_root: PathBuf::from(root),
            resolver: Resolver::Adopt,
        }
    }
}

impl Config {
    /// First try get config from default: `$HOME/.config/me-stow.toml`
    ///
    /// **CONFIG NOT EXISTS**: Create new one from default values. (if choose to do so)
    pub fn new() -> Self {
        let f_config_default = dirs::config_dir()
            .expect("stop here if failed")
            .join(super::constants::NAME_FILE_CFG);

        Self::new_from_file(f_config_default.as_path()).unwrap_or_else(|e| {
            e.print_err();

            if !Confirm::with_theme(&ColorfulTheme::default())
                .with_prompt("Create new config file:")
                .default(true)
                .interact()
                .unwrap_or(false)
            {
                println!("Manual create config file and run again.\nExit...!!");
                std::process::exit(0)
            }

            let config = Self::default();

            if Confirm::with_theme(&ColorfulTheme::default())
                .with_prompt("Save new config file:")
                .default(true)
                .interact()
                .unwrap_or(true)
            {
                // config.save_config(f_config_default.as_path());
                config.save_config(Path::new("save_config.toml")); // TODO: change after test
            }

            config
        })
    }

    fn new_from_file(dir: &Path) -> Result<Self, ErrType> {
        let mut f_toml = match File::open(dir) {
            Ok(f) => f,
            Err(msg) => {
                return Err(ErrType::BadConfigFile(format!(
                    "config file not found '{}': {}",
                    dir.display(),
                    msg
                )));
            }
        };

        let mut content = String::with_capacity(512); // Approximate
        if let Err(msg) = f_toml.read_to_string(&mut content) {
            return Err(ErrType::BadConfigFile(format!(
                "cannot read '{}': {}",
                dir.display(),
                msg
            )));
        }

        match toml::from_str::<Config>(&content) {
            Ok(config) => Ok(config),
            Err(e) => Err(ErrType::BadConfigFile(e.message().to_string())),
        }
    }

    fn save_config(&self, dir: &Path) {
        let str_config = toml::ser::to_string(self).unwrap();

        let mut f_toml = File::create(dir).unwrap();
        f_toml.write_all(str_config.as_bytes()).unwrap();
    }
}
