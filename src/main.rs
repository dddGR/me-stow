/* DEV ONLY */
#![allow(unused_imports)]
#![allow(unused_variables)]
#![allow(dead_code)]

mod args_parser;
mod config;

use config::Config;
use dialoguer::{Confirm, theme::ColorfulTheme};
use itertools::Itertools;
use me_stow::{fileio, log};
use rustc_hash::FxHashMap;
use std::{
    fs, io,
    path::{Path, PathBuf},
};
use termtree::Tree;

/* ===================================================== */
/* CONSTANTS =========================================== */
pub mod constants {
    pub const NAME_FILE_CFG: &str = "me-stow.toml";
}

fn main() {
    let cfg = Config::new();
    // dbg!(&c); // TODO: delete later
    let cli = args_parser::Cli::new();

    println!("Running 'me-stow'");
    match cli.command {
        args_parser::Command::Sync {
            packages,
            diff,
            force,
        } => {
            run_sync(cfg, packages, diff, force);
        }
        args_parser::Command::Stow { package, files } => {
            run_stow(cfg, package, files);
        }
        args_parser::Command::Remove { packages, purge } => {
            run_remove(cfg, packages, purge);
        }
        args_parser::Command::List { package, full } => {
            run_list(cfg, package, full);
        }
    }
}

/// Get all the packages that current in source dir
fn get_all_packages(src_dir: &Path) -> Option<FxHashMap<String, PathBuf>> {
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

    if map.is_empty() {
        log::error(format!(
            "no package found in current source dir: '{}'",
            src_dir.display(),
        ));
        None
    } else {
        Some(map)
    }
}

/* /// Get package dir, only if that package currently in source dir
fn get_package_dir<S: AsRef<str>>(src_dir: &Path, pkg_name: S) -> Option<PathBuf> {
    let curr_pkgs = get_all_packages(src_dir)?;

    match curr_pkgs.get(pkg_name.as_ref()) {
        Some(p) => Some(p.to_path_buf()),
        None => {
            log::error(format!(
                "request pkg '{}' not in current packages",
                pkg_name.as_ref()
            ));
            print_all_packages(curr_pkgs, false);
            None
        }
    }
} */

fn print_all_packages(pkgs: FxHashMap<String, PathBuf>, full_list: bool) {
    println!(
        "\nCurrent packages: [{:02}]",
        console::style(pkgs.len()).green()
    );

    if full_list {
        // Display everthings
        for pkg in pkgs.values() {
            if let Ok(tree) = tree(pkg) {
                println!("{tree}")
            } else {
                log::error(format!("cannot display package: '{}'", pkg.display()));
            }
        }
        println!();
    } else {
        // Name of packages only: print 5 items in one row
        for (i, p) in pkgs.keys().sorted().enumerate() {
            print!("{:20}{}", console::style(p).blue(), {
                if (i + 1) % 5 == 0 { "\n" } else { "" }
            });
        }
        println!("\n");
    }
}

fn run_list(config: Config, request_pkg: Option<String>, full_list: bool) -> Option<()> {
    let curr_pkgs = get_all_packages(&config.path_source)?;

    // Request to print only one package
    if let Some(pkg_name) = request_pkg {
        if let Some(pkg) = curr_pkgs.get(&pkg_name) {
            if let Ok(tree) = tree(pkg) {
                println!("Package {tree}")
            }
        } else {
            log::error(format!("request pkg '{pkg_name}' not in current packages"));
            print_all_packages(curr_pkgs, false);
        }
    } else {
        // No package provide by user, print all the packages that current in source
        // And only print package NAME if not need 'full_list'
        print_all_packages(curr_pkgs, full_list);
    }

    Some(())
}

fn run_sync(config: Config, packages: Option<Vec<String>>, diff: bool, force: bool) {
    println!(
        "Sync '{:?}' with [diff: {}] and force: {}",
        packages, diff, force
    )
}

fn run_stow(config: Config, pkg_name: String, files: Vec<PathBuf>) -> Option<()> {
    // FIRST: Get the path of the package in current src packages
    // IF: the request package not found, ask user to create a new one.
    // or quit, maybe argument is not type in correctly.
    let p_pkg = {
        let curr_pkgs = get_all_packages(&config.path_source)?;
        if let Some(path) = curr_pkgs.get(&pkg_name) {
            path.to_path_buf()
        } else {
            log::warn(format!(
                "request pkg '{}' not in current packages",
                &pkg_name
            ));
            print_all_packages(curr_pkgs, false);

            if !Confirm::with_theme(&ColorfulTheme::default())
                .with_prompt(format!("Create new package with name [{}]:", &pkg_name))
                .default(false)
                .interact()
                .unwrap_or(false)
            {
                return None;
            }

            let path = config.path_source.join(pkg_name);
            fs::create_dir(&path).expect(
                "this only create ONE empty dir, if failed here mean something really wrong",
            );
            path
        }
    };

    println!(
        "\nStowing files: {:#?} to package: '{}'",
        files,
        p_pkg.file_name().unwrap().display()
    );
    // Get the relative path from the file to root dir.
    // Try to make parent dirs in source dir if not exists.
    // Move file (on system) to src
    // Make symlink back to original dest file in system.

    // TODO: HANDLE
    // - user provide dir not file

    for f_sys in files {
        let f_stow_dest = match f_sys.strip_prefix(&config.path_root) {
            Ok(p) => p_pkg.join(p),
            Err(_) => {
                log::skip(format!(
                    "file '{}' not in root dir: '{}'",
                    f_sys.display(),
                    &config.path_root.display()
                ));
                continue;
            }
        };

        match f_sys.canonicalize() {
            Ok(f) if f == f_stow_dest => {
                // This only happend when the file on system is a symlink
                // and it's point to file that in src directory
                log::skip(format!("file '{}' is already stowed!!", f_sys.display()));
                continue;
            }
            Err(e) => {
                log::skip(format!("not valid path '{}': {}", f_sys.display(), e));
                continue;
            }
            _ => {}
        }

        let parent_path = f_stow_dest.parent().expect("this should never fail.");
        if let Err(e) = fs::create_dir_all(parent_path)
            && e.kind() != io::ErrorKind::AlreadyExists
        {
            log::fatal(format!("cannot make directory: {e}"))
        }

        // When adopt, file on system will replace file in src
        // use COPY and then REMOVE here bc: in case of f_sys is
        // a symlink to somewhere else, then content will be copy
        // and then only remove the f_sys.
        // (original src that f_sys link from is left untouch)
        if (!f_stow_dest.exists() || config.resolver.is_adopt())
            && let Err(e) = fs::copy(&f_sys, &f_stow_dest)
        {
            log::fatal(format!(
                "cannot copy '{}' -> '{}': {}",
                f_sys.display(),
                f_stow_dest.display(),
                e
            ))
        }

        if let Err(e) = fs::remove_file(&f_sys) {
            log::fatal(format!("cannot remove file '{}':", e))
        }

        if let Err(e) = symlink_rs::symlink_file(&f_stow_dest, &f_sys) {
            log::fatal(format!(
                "cannot make symlink '{}' -> '{}': {}",
                f_sys.display(),
                f_stow_dest.display(),
                e
            ))
        }

        // TODO: maybe do cleanup if fail halfway.
    }
    Some(())
}

fn run_remove(config: Config, pkg_names: Vec<String>, purge: bool) -> Option<()> {
    let curr_pkgs = get_all_packages(&config.path_source)?;

    for pkg in pkg_names {
        let Some(p_pkg) = curr_pkgs.get(&pkg) else {
            log::skip(format!("package '{}' not in current packages", pkg));
            continue;
        };

        println!("Removing package '{pkg}'");
        let mut err = false;
        let mut pattern = p_pkg.to_string_lossy().to_string();
        pattern.push_str("/**/[!.]*.*");

        // Get all files that in current package
        for f_stowed in glob::glob(&pattern)
            .expect("Failed to read glob pattern")
            .flatten()
        {
            // get equivalent file on the system
            let f_sys = if let Ok(p) = f_stowed.strip_prefix(config.path_source.join(&pkg)) {
                config.path_root.join(p)
            } else {
                continue;
            };

            // if the file on systems not a symlink to the file on src package,
            // simply ignore it.
            if !fileio::is_link_to_same_file(&f_sys, &f_stowed) {
                log::skip(format!(
                    "detect file '{}', but not in stowed package!",
                    f_sys.display()
                ));
                continue;
            }

            // WHEN: `purge`, simply remove everything.
            // ELSE: copy file in src package to override file
            // on the system (opposite with when stow)
            // WHEN: error occur, mark err flag as true.
            if purge {
                if let Err(e) = fs::remove_file(&f_sys) {
                    log::warn(format!("cannot remove file '{}': {}", f_sys.display(), e));
                    err = true;
                }
                continue;
            }

            if let Err(e) = fs::rename(&f_stowed, &f_sys) {
                log::warn(format!("cannot restore file '{}': {}", f_sys.display(), e));
                err = true;
            }
        }

        // IF: err NOT occur during previous operation,
        // remove package directory and all the files that in it.
        // IF FAIL: mark err flag is `true`
        // and concatinate message to display as result.
        let mut msg = String::new();
        if !err && let Err(e) = fs::remove_dir_all(p_pkg) {
            msg.push_str(": ");
            msg.push_str(e.to_string().as_str());
            err = true;
        }

        if err {
            log::error(format!("cannot remove package '{pkg}'{msg}"));
        } else {
            log::sucess(format!("package '{}' removed!!", pkg));
        }
    }

    Some(())
}

fn tree<P: AsRef<Path>>(p: P) -> std::io::Result<Tree<String>> {
    fn label<P: AsRef<Path>>(p: P) -> String {
        p.as_ref().file_name().unwrap().to_str().unwrap().to_owned()
    }

    let result = fs::read_dir(&p)?.filter_map(|e| e.ok()).fold(
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
