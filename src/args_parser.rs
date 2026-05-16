/* ----------------------------------------------------- */
/* ARGS PARSER ----------------------------------------- */

use std::path::PathBuf;

use clap::ArgAction;
use clap::Parser;

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

#[derive(clap::Subcommand)]
pub enum Command {
    /// Show packages status, alias for `sync -d`
    Status {
        /// Name of package(s) to display, omit will display all in current system.
        packages: Option<Vec<String>>,
    },
    /// Sync packages in source with system
    Sync {
        /// Name of package(s) to sync. Work with basic glob [*?!..]
        /// Omit this will sync all packages that currently on the system.
        /// Alternatively, run: sync '*' (with quote) (basicly get all)
        /// will sync everything in source (can be use on fresh system).
        #[arg(verbatim_doc_comment)]
        packages: Option<Vec<String>>,

        /// Only check for differences, no file will change on system.
        #[arg(short, long, action = ArgAction::SetTrue)]
        diff: bool,

        /// Override current file on system if conflicts happen.
        #[arg(short, long, action = ArgAction::SetTrue)]
        force: bool,
    },
    /// Stow files into package
    Stow {
        /// Name of package to stow
        package: String,

        /// File(s) to stow to that package.
        /// When provide directory,
        /// This will stow every files in that dir.
        #[arg(required = true, verbatim_doc_comment)]
        paths: Vec<PathBuf>,
    },
    /// Remove selected packages
    Remove {
        /// Name of package to remove.
        packages: Option<String>, // TODO: remove multiple pks

        /// File(s) to remove. Empty will remove whole package.
        files: Option<Vec<PathBuf>>,

        /// Remove all packages.
        #[arg(long, action = ArgAction::SetTrue)]
        all: bool,

        /// Remove everything, include the files on the system.
        #[arg(short, long, action = ArgAction::SetTrue)]
        purge: bool,
    },
    /// List all packages that all currently in source.
    List {
        /// Name of package to list in detail.
        /// Omit will list all (but in simple list)
        #[arg(verbatim_doc_comment)]
        package: Option<String>,

        /// List all the files include in packages
        #[arg(short, long, action = ArgAction::SetTrue)]
        full: bool,
    },
}
