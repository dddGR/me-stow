/* ----------------------------------------------------- */
/* CONFIGURATIONS READER ------------------------------- */

use std::fs::File;
use std::io;
use std::io::Write;
use std::path::Path;
use std::path::PathBuf;

use dialoguer::Input;
use dialoguer::theme::ColorfulTheme;

use crate::ResErr;
use crate::error::ErKind;
use crate::messages as ms;

/* ----------------------------------------------------- */
/* CONSTANTS ------------------------------------------- */
mod constants {
    pub const NAME_FILE_CFG: &str = "me-stow.toml";
}

#[derive(Debug, PartialEq, serde::Deserialize, serde::Serialize, Clone, Copy)]
#[serde(rename_all = "lowercase")]
pub enum Resolver {
    /// Adopt file on the system into stow src.
    Adopt,
    /// Replace file on the system with the file currently in src.
    Replace,
}

impl Resolver {
    pub fn is_adopt(&self) -> bool {
        self == &Resolver::Adopt
    }

    pub fn is_replace(&self) -> bool {
        self == &Resolver::Replace
    }
}

#[derive(Debug, serde::Deserialize, serde::Serialize)]
pub struct Config {
    /// Path to store stower packages
    pub path_source: PathBuf,
    /// Path to act as root for stow package, usually $HOME
    pub path_root: PathBuf,
    /// Method to resolve conflict
    pub resolver: Resolver,
    /// List of current packages in the system
    pub packages: Vec<String>,
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
            packages: Vec::new(),
        }
    }
}

impl Config {
    /// First try get config from default: `$HOME/.config/me-stow.toml`
    ///
    /// **CONFIG NOT EXISTS**: Create new one from default values. (if choose to do so)
    pub fn new() -> Result<Self, ErKind> {
        let f_config_default = Config::get_path_default();
        match Self::new_from_file(&f_config_default) {
            Ok(config) => Ok(config),
            Err(err) if matches!(err, ErKind::NotFoundConfig) => {
                // log::warn("not found configurations file!!");
                ms::warn!(err);
                if !ms::ask_confirm("Create new config file:", true, true).expect("checked") {
                    return Err(ErKind::UserAbort(Some(
                        "Create config file and run again.\nExit...!!",
                    )));
                }

                let config = Self::default();
                if ms::ask_confirm("Save new config file:", true, true).expect("checked") {
                    config.save_to_file(&f_config_default)?;
                }

                Ok(config)
            }
            Err(e) => Err(e),
        }
    }

    fn get_path_default() -> PathBuf {
        dirs::config_dir()
            .expect("stop here if failed")
            .join(constants::NAME_FILE_CFG)
    }

    fn new_from_file(path: &Path) -> Result<Self, ErKind> {
        let f_toml = File::open(path).map_err(|e| {
            if e.kind() == io::ErrorKind::NotFound {
                ErKind::NotFoundConfig
            } else {
                ErKind::IOFailRead(path.to_path_buf(), e.to_string())
            }
        })?;

        let content = io::read_to_string(f_toml)
            .map_err(|e| ErKind::IOFailRead(path.to_path_buf(), e.to_string()))?;

        toml::from_str::<Self>(&content).map_err(|e| ErKind::BadConfigFile(e.to_string()))
    }

    fn save_to_file(&self, path: &Path) -> ResErr {
        let cfg_str = toml::ser::to_string(self).unwrap();

        File::create(path)
            .and_then(|mut fd| fd.write_all(cfg_str.as_bytes()))
            .map_err(|err| ErKind::IOFailWrite(path.to_path_buf(), err.to_string()))?;

        Ok(())
    }

    pub fn save(&self) -> ResErr {
        let config_path = Config::get_path_default();
        self.save_to_file(&config_path)
    }
}
