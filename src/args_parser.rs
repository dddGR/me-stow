/* ===================================================== */
/* ARGS PARSER ========================================= */

use clap::{Parser, Subcommand};
use std::path::PathBuf;

#[derive(Parser)]
#[command(version, about, long_about = None)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Command,
}

#[allow(clippy::new_without_default)]
impl Cli {
    pub fn new() -> Self {
        Cli::parse()
    }
}

#[derive(Subcommand)]
pub enum Command {
    /// Sync packages in source with system
    Sync {
        /// Name of packages to deploy, omit will deploy all
        packages: Option<Vec<String>>,

        /// Check for differences only
        #[arg(short, long, action = clap::ArgAction::SetTrue)]
        diff: bool,

        /// Override current file on system if conflicts happen.
        #[arg(short, long, action = clap::ArgAction::SetTrue)]
        force: bool,
    },
    /// Stow files to package
    Stow {
        /// Name of package to stow
        package: String,

        /// Files to stow to that package
        files: Vec<PathBuf>,
    },
    /// Remove selected packages
    Remove {
        /// Name of packages to remove
        packages: Vec<String>,

        /// Copy file actual file back to the system
        #[arg(short, long, action = clap::ArgAction::SetTrue)]
        copyback: bool,
    },

    /// List all current packages
    List {
        /// Name of package to list, omit will list all
        package: Option<String>,

        /// List all the files include in packages
        #[arg(short, long, action = clap::ArgAction::SetTrue)]
        verbose: bool,
    },
}
