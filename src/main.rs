/* DEV ONLY */
#![allow(unused_imports)]
#![allow(unused_variables)]
#![allow(dead_code)]

mod args_parser;

use args_parser::{Cli, Command};

fn main() {
    let cli = Cli::new();
    println!("Running 'me-stow'");

    match &cli.command {
        Command::Sync {
            packages: package,
            diff,
            force,
        } => {
            println!(
                "Sync '{:?}' with [{}] and force: {}",
                package, diff, force
            )
        }
        Command::Stow { package, files } => {
            println!(
                "Stowing files: {:?} to package: '{}'",
                files, package
            )
        }
        Command::Remove { packages, copyback } => {
            println!(
                "Removing package: {:?} with copyback: '{}'",
                packages, copyback
            )
        }
        Command::List { .. } => {
            println!("List packages")
        }
    }
}
