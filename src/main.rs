mod args_parser;
mod config;

use config::Config;
use itertools::Itertools;
use me_stow::{
    error::{ErrType, ResErr, ResType},
    fileio, log,
};
use rustc_hash::FxHashMap;
use std::{
    fs, io,
    path::{Path, PathBuf},
};
use termtree::Tree;

// TODO:
// - add `packages.toml` to config for each systems.
// - f_sys is a link to a file that in the same dir(package)

/* ----------------------------------------------------- */
/* CONSTANTS ------------------------------------------- */
pub mod constants {
    pub const NAME_FILE_CFG: &str = "me-stow.toml";
}

fn main() {
    use args_parser::{Cli, Command};

    let config = Config::new();
    let cli = Cli::new();

    #[rustfmt::skip]
    let result = match cli.command {
        Command::Sync   { packages, diff, force }  => run_sync(config, packages, diff, force),
        Command::Stow   { package, paths }         => run_stow(config, package, paths),
        Command::Remove { packages, files, purge, all } => run_remove(config, packages, files, purge, all),
        Command::List   { package, full }          => run_list(config, package, full),
    };

    // Print result
    if let Err(e) = result {
        e.print_err();
    }
}

/// Get all the packages that current in source dir
fn get_all_packages(src_dir: &Path) -> ResType<FxHashMap<String, PathBuf>> {
    // GET EVERY DIR IN SOURCE THAT NOT START WITH '.'

    let mut map: FxHashMap<String, PathBuf> = FxHashMap::default();

    let entries = match src_dir.read_dir() {
        Ok(t) => t.flatten(),
        Err(e) => log::fatal(format!("cannot read source '{}': {e}", src_dir.display())),
    };

    for path in entries.map(|e| e.path()).filter(|p| p.is_dir()) {
        if let Some(name) = path.file_name()
            && let Some(name) = name.to_str().map(|n| n.to_string())
            && !name.starts_with(".")
        {
            map.insert(name, path);
        }
    }

    if map.is_empty() {
        Err(ErrType::Generic(format!(
            "no package found in current source dir: '{}'",
            src_dir.display(),
        )))
    } else {
        Ok(map)
    }
}

fn print_all_packages(pkgs: FxHashMap<String, PathBuf>, full_list: bool) {
    println!(
        "\nCurrent packages: [{:02}]",
        console::style(pkgs.len()).green()
    );

    if full_list {
        // Display everythings
        for (name, d_pkg) in pkgs.iter() {
            match tree(d_pkg) {
                Ok(tree) => println!("{tree}"),
                Err(e) => log::error(format!(
                    "cannot display package: '{}': {e}",
                    console::style(name).blue()
                )),
            }
        }
    } else {
        // Name of packages only: print 5 items in one row
        let total = pkgs.len();
        for (i, p) in pkgs.keys().sorted().enumerate() {
            let name = console::style(p).blue();
            if (i + 1) % 5 == 0 || i == total - 1 {
                println!("> {}", name)
            } else {
                print!("> {:18}", name)
            }
        }
    }
}

fn run_list(config: Config, request_pkg: Option<String>, full_list: bool) -> ResErr {
    let curr_pkgs = get_all_packages(&config.path_source)?;

    // Request to print only one package
    if let Some(pkg_name) = request_pkg {
        if let Some(pkg) = curr_pkgs.get(&pkg_name) {
            if let Ok(tree) = tree(pkg) {
                println!("\nPackage:\n{tree}")
            }
        } else {
            log::error(format!("request pkg '{pkg_name}' not in stowed packages"));
            print_all_packages(curr_pkgs, false);
        }
    } else {
        // No package provide by user, print all the packages that current in source
        // And only print package NAME if not need `full_list`
        print_all_packages(curr_pkgs, full_list);
    }

    Ok(())
}

fn run_sync(config: Config, packages: Option<Vec<String>>, diff: bool, force: bool) -> ResErr {
    let curr_pkgs = get_all_packages(&config.path_source)?;
    let total = curr_pkgs.len();
    let mut request = total;
    let mut success = 0;

    if let Some(r_pkgs) = packages {
        request = r_pkgs.len();
        // User request specific packages
        for pkg in r_pkgs {
            if let Some(d_pkg) = curr_pkgs.get(&pkg) {
                if let Err(e) = sync_package(&config, d_pkg, diff, force) {
                    log::error(format!(
                        "package '{}' failed to sync: {e}",
                        console::style(pkg).blue()
                    ));
                } else {
                    success += 1;
                }
            } else {
                log::skip(format!(
                    "request package '{}' not in current stowed packages",
                    console::style(&pkg).blue()
                ));
            }
        }
    } else {
        // No package specify, sync all
        for (name, d_pkg) in curr_pkgs.iter().sorted() {
            if let Err(e) = sync_package(&config, d_pkg, diff, force) {
                log::error(format!(
                    "package '{}' failed to sync: {e}",
                    console::style(name).blue()
                ));
            } else {
                success += 1;
            }
        }
    }

    if !diff {
        println!();
        log::sucess(format!(
            "synced [ {}/{} ] over total [{}] packages.",
            console::style(success).green(),
            console::style(request).green(),
            console::style(total).blue(),
        ));
    }
    Ok(())
}

fn sync_package(config: &Config, pkg_dir: &Path, diff: bool, force: bool) -> ResErr {
    if let Some(pkg_name) = pkg_dir.file_name() {
        let name = console::style(pkg_name.display()).blue();
        if diff {
            println!("\nStatus package [[ {name} ]]:");
        } else {
            log::info(format!("syncing package '{name}':"));
        }
    }

    let pattern = format!("{}/**/*", pkg_dir.display());
    let files_globbed = match glob::glob(&pattern) {
        Err(e) => {
            return Err(ErrType::Generic(format!(
                "bad glob pattern '{pattern}': {e}"
            )));
        }
        Ok(p) => p.flatten().filter(|p| p.is_file()),
    };

    for f_stowed in files_globbed {
        // get equivalent file on the system
        let p_relative = f_stowed
            .strip_prefix(pkg_dir)
            .expect("f_stowed get from globbing pkg_dir, this never fail");
        let f_sys = config.path_root.join(p_relative);

        // IF: file on systems is a symlink to the file on src package,
        // (aka. already stowed), simply ignore it.
        // WHEN: user just want to check differences, print result only
        let is_same_file = fileio::is_the_same_file(&f_sys, &f_stowed);
        if diff {
            let status = if is_same_file {
                console::style(" SYNCED ").green()
            } else {
                console::style("NOT SYNC").blue()
            };
            println!("[{}] -- '{}'", status, p_relative.display());
            continue;
        }

        if is_same_file {
            continue;
        }

        // WHEN: user want to force src file on to the system,
        // remove the file that currenly on the system (if any).
        // OTHERWISE: fallback to resolver method
        match force || config.resolver.is_replace() {
            true => {
                if let Err(e) = fs::remove_file(&f_sys)
                    && e.kind() != io::ErrorKind::NotFound
                {
                    log::warn(format!("cannot delete file '{}': {e}", f_sys.display()));
                    continue;
                }
            }
            false => {
                if let Err(e) = fs::rename(&f_sys, &f_stowed)
                    && e.kind() != io::ErrorKind::NotFound
                {
                    log::warn(format!("cannot move file '{}': {e}", f_sys.display()));
                    continue;
                }
            }
        }

        if let Err(e) = symlink_rs::symlink_file(&f_stowed, &f_sys) {
            log::error(format!(
                "cannot make symlink '{}' -> '{}': {e}",
                f_sys.display(),
                f_stowed.display()
            ));
            continue;
        }

        log::sucess(format!("stowed success: '{}'", f_sys.display()));
    }

    Ok(())
}

fn run_stow(config: Config, pkg_name: String, paths: Vec<PathBuf>) -> ResErr {
    // FIRST: Get the path of the package in current src packages
    // IF: the request package not found, or no packages at all (start fresh)
    // ask user to create a new one. or quit.
    let p_pkg = {
        let mut temp_path: Option<PathBuf> = None;

        println!(); // for spacing in terminal, easier to read.
        if let Ok(curr_pkgs) = get_all_packages(&config.path_source) {
            if let Some(path) = curr_pkgs.get(&pkg_name) {
                temp_path.replace(path.to_path_buf());
            } else {
                log::warn(format!(
                    "request pkg '{}' not in stowed packages",
                    console::style(&pkg_name).blue()
                ));
                print_all_packages(curr_pkgs, false);
            }
        }

        if temp_path.is_none()
            && log::ask_confirm(
                format!("Create new package with name [{}]:", &pkg_name),
                false,
                false,
            )?
        {
            let p = config.path_source.join(&pkg_name);
            fs::create_dir(&p)
                .expect("this only create ONE empty dir, if fail here mean something really wrong");
            temp_path.replace(p);
        }

        temp_path.expect("this should always be Some(path) now")
    };

    // IF: user provide directory specific (or along with file)
    // expand that directory into all the files that in it.
    let files = {
        let mut temp_v: Vec<PathBuf> = Vec::new();
        for p in paths {
            if p.is_dir() {
                let pattern = format!("{}/**/*", p.display());
                if let Ok(glob) = glob::glob(&pattern) {
                    temp_v.append(
                        glob.flatten()
                            .filter(|p| p.is_file())
                            .collect::<Vec<PathBuf>>()
                            .as_mut(),
                    );
                }
            } else if p.exists() {
                temp_v.push(p);
            } else {
                log::skip(format!("not valid path '{}'", p.display()));
            }
        }
        temp_v
    };

    log::info(format!(
        "stowing [{:02}] files: {:#?} to package: '{}'",
        console::style(files.len()).green(),
        files,
        console::style(&pkg_name).blue()
    ));

    // Try to trim the root_path from file to get relative path (like in GNU stow)
    //   and concatinate equivalent path on stow package src dir.
    // IF: the file on stowed package (f_stow_dest) exists and
    //   is the same as the file on system (f_sys).
    //   This mean that `f_sys` is already stowed.
    // BUT IF: it is not the same file, then will resolve with method below.
    // AND THEN: simply move(copy and delete) the `f_sys` to stow package dir,
    //   and replace that with a symlink to `f_stow_dest`
    let total = files.len();
    let mut success = 0;
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

        // Check `f_sys` is not alredy stowed.
        if fileio::is_the_same_file(&f_sys, &f_stow_dest) {
            log::skip(format!("file '{}' is already stowed!!", f_sys.display()));
            continue;
        }

        /* From now, when error, it's system runtime error,
        the error will mark as fatal and stop execution. */
        let parent_path = f_stow_dest.parent().expect("this should never fail.");
        if let Err(e) = fs::create_dir_all(parent_path)
            && e.kind() != io::ErrorKind::AlreadyExists
        {
            log::fatal(format!("cannot make directory: {e}"))
        }

        // WHEN: adopt, file on system will replace file in src
        //  use COPY and then REMOVE here bc: in case of f_sys is
        //  a symlink to somewhere else, then content will be copy
        //  and then only remove the f_sys.
        // (original src that f_sys link from is left untouch)
        if (!f_stow_dest.exists() || config.resolver.is_adopt())
            && let Err(e) = fs::copy(&f_sys, &f_stow_dest)
        {
            log::fatal(format!(
                "cannot copy '{}' -> '{}': {e}",
                f_sys.display(),
                f_stow_dest.display()
            ))
        }

        if let Err(e) = fs::remove_file(&f_sys) {
            log::fatal(format!("cannot remove file '{}': {e}", f_sys.display()))
        }

        if let Err(e) = symlink_rs::symlink_file(&f_stow_dest, &f_sys) {
            log::fatal(format!(
                "cannot make symlink '{}' -> '{}': {e}",
                f_sys.display(),
                f_stow_dest.display()
            ))
        }

        log::sucess(format!("stow success: '{}'", f_sys.display()));
        success += 1;

        // TODO: maybe do cleanup if fail halfway.
    }

    println!();
    log::sucess(format!(
        "stowed [ {}/{} ] to package: '{}'",
        success,
        total,
        console::style(pkg_name).blue()
    ));
    Ok(())
}

fn run_remove(
    config: Config,
    package: Option<String>,
    files: Option<Vec<PathBuf>>,
    purge: bool,
    is_remove_all: bool,
) -> ResErr {
    fn get_files_in_pkg(pkg_path: impl AsRef<Path>, pattern: Option<&str>) -> Option<Vec<PathBuf>> {
        let pat = format!(
            "{}/**/{}",
            pkg_path.as_ref().display(),
            pattern.unwrap_or("*")
        );
        let globs: Vec<PathBuf> = glob::glob(&pat)
            .expect("pattern checked")
            .flatten()
            .filter(|p| p.is_file())
            .collect();

        if globs.is_empty() { None } else { Some(globs) }
    }

    fn rm_package(
        path_root: impl AsRef<Path>,
        pkg_name: impl AsRef<str>,
        pkg_path: impl AsRef<Path>,
        rm_files: Vec<PathBuf>,
        purge: bool,
    ) -> ResErr {
        println!();
        log::info(format!(
            "removing package '{}'",
            console::style(pkg_name.as_ref()).blue()
        ));

        for f_stowed in rm_files {
            // get equivalent file on the system
            let path_rel = f_stowed
                .strip_prefix(pkg_path.as_ref())
                .expect("path is already evalutated, this never fail");
            let f_sys = path_root.as_ref().join(path_rel);

            // WHEN: `purge`, simply remove everything.
            // ELSE: copy file in src package to override file
            // on the system (opposite with when stow)
            let result = match purge {
                true => {
                    // log::info("doing remove");
                    fs::remove_file(&f_stowed).and_then(|_| fs::remove_file(f_sys))
                }
                false => {
                    // WHEN: `f_sys` is not exists or not link to `f_stowed`
                    // this mean the file only in the stow pkg, and not related
                    // to the file on the system (if that exists).
                    // log::info("doing restore");
                    if fileio::is_the_same_file(&f_sys, &f_stowed) {
                        fs::rename(&f_stowed, f_sys)
                    } else {
                        fs::remove_file(&f_stowed)
                    }
                }
            };

            // TODO: Concatinate alls the failed into a vec and return
            if let Err(err) = result
                && err.kind() != std::io::ErrorKind::NotFound
            {
                return Err(ErrType::FileRWFailed(format!(
                    "cannot {} file '<root>/{}': {}",
                    if purge { "remove" } else { "restore" },
                    path_rel.display(),
                    err
                )));
            }
        }

        Ok(())
    }

    let curr_pkgs = get_all_packages(&config.path_source)?;
    if let Some(pkg_name) = package {
        dbg!(&pkg_name);
        let Some(pkg_path) = curr_pkgs.get(&pkg_name) else {
            return Err(ErrType::Generic(format!(
                "package '{}' not in stowed packages",
                console::style(pkg_name).blue()
            )));
        };

        // This will store all the files that can (and will) be remove
        // And this only store the stowed file (not path to symlink on sys)
        // The path to symlink will be concatinate later when doing remove.
        let mut rm_files: Vec<PathBuf> = Vec::new();
        match files {
            Some(f) => {
                // When user provide file(s) to remove.
                for p_file in f {
                    // EVERYTHING NOT SUCCESS WILL FALL TO THE BOTTOM.
                    if let Ok(f_stowed) = p_file.canonicalize()
                        && f_stowed.strip_prefix(pkg_path).is_ok()
                    {
                        // User provide path that that in stow package or
                        // point to the file to the file on stow package (stowed file)
                        // pkg-path will be stripable.
                        rm_files.push(f_stowed);
                        continue;
                    } else {
                        // TRY: to match any file that in provide package
                        // When all above failed, assume that user provide only
                        // file(s) name (or relative path),
                        // try to search in package for that file.
                        let glob_finds = p_file.to_str().and_then(|p| {
                            let pattern = p.trim_start_matches('/');
                            get_files_in_pkg(pkg_path, Some(pattern))
                        });

                        if let Some(files) = glob_finds {
                            rm_files.extend(files);
                            continue;
                        }
                    }

                    log::skip(format!("not valid file: '{}'", p_file.display()));
                }
            }
            None => {
                // User not provide file to remove
                // Remove all files in package
                match get_files_in_pkg(pkg_path, None) {
                    Some(files) => rm_files = files,
                    None => return Err(ErrType::Generic("package empty".to_string())),
                }
            }
        };
        // Print result here and wait for user comfirmation
        if rm_files.is_empty() {
            log::info("nothing to remove!");
        } else {
            println!("\nRemovable files:");
            for file in &rm_files {
                log::sucess(format!(
                    "'<root>/{}'",
                    file.strip_prefix(pkg_path).unwrap().display()
                ));
            }
            if log::ask_confirm("Do remove all:", false, false)? {
                rm_package(&config.path_root, pkg_name, pkg_path, rm_files, purge)?;

                // try to remove the package, if every files has been deleted
                // the package dir will be remove, othewise, ignore
                fileio::rm_empty_dirs(pkg_path)?;
                // TODO: modify the package recored (when that implemented).
            }
        }
    } else {
        // Package is alway provide before file(s)
        // if NOT Package, files is also None (no need to check)
        if !is_remove_all {
            return Err(ErrType::Generic(
                "nothing to remove\nuse '--all' if you want to remove all packages".to_string(),
            ));
        }
        // TODO: maybe list all the current pkgs for user to confirm.
        for (pkg_name, pkg_path) in curr_pkgs.iter() {
            match get_files_in_pkg(pkg_path, None) {
                None => log::skip(format!(
                    "{}: package empty",
                    console::style(pkg_name).blue()
                )),
                Some(pkg_files) => {
                    rm_package(&config.path_root, pkg_name, pkg_path, pkg_files, purge)?;
                    fileio::rm_empty_dirs(pkg_path)?;
                }
            }
        }
    }

    Ok(())
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
