/* ----------------------------------------------------- */
/* ARGS PARSER ----------------------------------------- */

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
        /// Name of packages to deploy, omit will deploy all.
        /// Can provide multiple packages.
        packages: Option<Vec<String>>,

        /// Only check for differences, no file will change on system.
        #[arg(short, long, action = clap::ArgAction::SetTrue)]
        diff: bool,

        /// Override current file on system if conflicts happen.
        #[arg(short, long, action = clap::ArgAction::SetTrue)]
        force: bool,
    },
    /// Stow files into package
    Stow {
        /// Name of package to stow
        package: String,

        /// Files to stow to that package.
        /// If provide directory, this will stow every files in that dir.
        #[arg(required = true)]
        paths: Vec<PathBuf>,
    },
    /// Remove selected packages
    Remove {
        /// Name of package to remove.
        // #[arg(required = true)]
        packages: Option<String>,

        /// File(s) to remove. Empty will remove whole package.
        files: Option<Vec<PathBuf>>,

        /// Remove all packages.
        #[arg(long, action = clap::ArgAction::SetTrue)]
        all: bool,

        /// Remove everything, include the files on the system.
        #[arg(short, long, action = clap::ArgAction::SetTrue)]
        purge: bool,
    },

    /// List all current packages
    List {
        /// Name of package to list, omit will list all
        package: Option<String>,

        /// List all the files include in packages
        #[arg(short, long, action = clap::ArgAction::SetTrue)]
        full: bool,
    },
}
