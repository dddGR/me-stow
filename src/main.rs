/* DEV ONLY */
#![allow(unused_imports)]
#![allow(unused_variables)]
#![allow(dead_code)]

use std::path::PathBuf;

mod args_parser;
mod config;

/* ===================================================== */
/* CONSTANTS =========================================== */
pub mod constants {
    pub const NAME_FILE_CFG: &str = "me-stow.toml";
}

fn main() {
    println!("Running 'me-stow'");

    let config = config::Config::new();
    println!("Config: {:?}", config); // TODO: delete later
    let cli = args_parser::Cli::new();

    match cli.command {
        args_parser::Command::Sync {
            packages,
            diff,
            force,
        } => {
            run_sync(packages, diff, force);
        }
        args_parser::Command::Stow { package, files } => {
            run_stow(package, files);
        }
        args_parser::Command::Remove { packages, copyback } => {
            run_remove(packages, copyback);
        }
        args_parser::Command::List { package, verbose } => {
            run_list(package, verbose);
        }
    }
}

fn run_list(package: Option<String>, verbose: bool) {
    println!("List packages")
}

fn run_sync(packages: Option<Vec<String>>, diff: bool, force: bool) {
    println!("Sync '{:?}' with [{}] and force: {}", packages, diff, force)
}

fn run_stow(package: String, files: Vec<PathBuf>) {
    println!("Stowing files: {:?} to package: '{}'", files, package)
}

fn run_remove(packages: Vec<String>, copyback: bool) {
    println!(
        "Removing package: {:?} with copyback: '{}'",
        packages, copyback
    )
}
