/* DEV ONLY */
#![allow(unused_imports)]
#![allow(unused_variables)]
#![allow(dead_code)]

mod args_parser;
mod config;

use config::Config;
use itertools::Itertools;
use me_stow::log;
use rustc_hash::FxHashMap;
use std::path::{Path, PathBuf};
use termtree::Tree;

/* ===================================================== */
/* CONSTANTS =========================================== */
pub mod constants {
    pub const NAME_FILE_CFG: &str = "me-stow.toml";
}

fn main() {
    println!("Running 'me-stow'");

    let c = Config::new();
    println!("Config: {:#?}", c); // TODO: delete later
    let cli = args_parser::Cli::new();

    let curr_pkgs = get_all_packages(c.path_source.as_path());

    match cli.command {
        args_parser::Command::Sync {
            packages,
            diff,
            force,
        } => {
            run_sync(c, packages, diff, force);
        }
        args_parser::Command::Stow { package, files } => {
            run_stow(c, package, files);
        }
        args_parser::Command::Remove { packages, copyback } => {
            run_remove(c, packages, copyback);
        }
        args_parser::Command::List { package, full } => {
            run_list(curr_pkgs, c, package, full);
        }
    }
}

/// Get all the packages that current in source dir
fn get_all_packages(src_dir: &Path) -> FxHashMap<String, PathBuf> {
    // GET EVERY DIR IN SOURCE THAT NOT START WITH '.'

    let mut map: FxHashMap<String, PathBuf> = FxHashMap::default();

    let entries = match src_dir.read_dir() {
        Ok(t) => t.flatten(),
        Err(e) => log::fatal(format!("cannot read source '{}': {}", src_dir.display(), e)),
    };

    for entry in entries {
        let path = entry.path();
        if path.is_dir()
            && let Some(name) = path.file_name()
            && !name.to_string_lossy().starts_with(".")
        {
            map.insert(name.to_string_lossy().to_string(), path);
        }
    }

    map
}

fn run_list(
    curr_pkgs: FxHashMap<String, PathBuf>,
    config: Config,
    request_pkg: Option<String>,
    full_list: bool,
) {
    if curr_pkgs.is_empty() {
        log::error(format!(
            "no package found in current source dir: '{}'",
            config.path_source.display(),
        ));
        return;
    }

    fn print_all_current_pkg(curr_pkgs: FxHashMap<String, PathBuf>, full_list: bool) {
        println!("\nCurrent packages: [{:02}]", curr_pkgs.len());

        // Name of pkgs only: print 5 items in one row
        if !full_list {
            for (i, p) in curr_pkgs.keys().sorted().enumerate() {
                print!("{:20}{}", p, { if (i + 1) % 5 == 0 { "\n" } else { "" } });
            }
            println!();
            return;
        }

        // Display everthings
        for pkg in curr_pkgs.values() {
            if let Ok(tree) = tree(pkg) {
                println!("{tree}")
            } else {
                log::error(format!("cannot display package: '{}'", pkg.display()));
            }
        }
    }

    // Request to print only one package
    if let Some(pkg_name) = request_pkg {
        if let Some(pkg) = curr_pkgs.get(&pkg_name) {
            if let Ok(tree) = tree(pkg) {
                println!("Package {tree}")
            }
        } else {
            log::error(format!("request pkg '{pkg_name}' not in current packages"));
            print_all_current_pkg(curr_pkgs, false);
        }
    } else {
        // No package provide by user, print all the packages that current in source
        // And only print package NAME if not need 'full_list'
        print_all_current_pkg(curr_pkgs, full_list);
    }
}

fn run_sync(config: Config, packages: Option<Vec<String>>, diff: bool, force: bool) {
    println!(
        "Sync '{:?}' with [diff: {}] and force: {}",
        packages, diff, force
    )
}

fn run_stow(config: Config, package: String, files: Vec<PathBuf>) {
    println!("Stowing files: {:?} to package: '{}'", files, package)
}

fn run_remove(config: Config, packages: Vec<String>, copyback: bool) {
    println!(
        "Removing package: {:?} with copyback: '{}'",
        packages, copyback
    )
}

fn tree<P: AsRef<Path>>(p: P) -> std::io::Result<Tree<String>> {
    fn label<P: AsRef<Path>>(p: P) -> String {
        p.as_ref().file_name().unwrap().to_str().unwrap().to_owned()
    }

    let result = std::fs::read_dir(&p)?.filter_map(|e| e.ok()).fold(
        Tree::new(label(p.as_ref().canonicalize()?)),
        |mut root, entry| {
            let dir = entry.metadata().unwrap();
            if dir.is_dir() {
                root.push(tree(entry.path()).unwrap());
            } else {
                root.push(Tree::new(label(entry.path())));
            }
            root
        },
    );
    Ok(result)
}
