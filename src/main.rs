mod args_parser;
mod config;

use crate::config::{Config, Resolver};
use me_stow::{
    error::{ErKind, ResErr, ResType},
    fileio, log,
};

use itertools::Itertools;
use rustc_hash::FxHashMap;
use std::os::unix::ffi::OsStrExt;
// use std::fmt::Display;
use std::{
    fs, io,
    path::{Path, PathBuf},
};
use termtree::Tree;

/// Final result, use to print report
type AppResult = ResType<Option<Config>>;

// TODO:
// - f_sys is a link to a file that in the same dir(package).
// - file need to be stow that already in other stow pkg.
// - EXCLUDE matches pattern.

fn main() {
    match run_main() {
        Err(err) => {
            err.print(None);
            match err {
                ErKind::NoValidPackage(Some(src)) | ErKind::SysEmpty(Some(src)) => {
                    let _ = run_list(&src, None, false);
                }
                _ => {}
            }
        }
        Ok(some) => {
            if let Some(config) = some {
                // log::info("save config file");
                config.save();
            }
        }
    }
}

fn run_main() -> AppResult {
    use args_parser::{Cli, Command};

    let config = Config::new()?;
    let cli = Cli::new();

    println!(); // for spacing in terminal, easier to read.
    #[rustfmt::skip]
    let rs = match cli.command {
        Command::Status { packages }                => run_sync(config, packages, true, false),
        Command::Sync   { packages, diff, force }   => run_sync(config, packages, diff, force),
        Command::Stow   { package, paths }          => run_stow(config, package, paths),
        Command::Remove { packages, files, purge, all } => run_remove(config, packages, files, purge, all),
        Command::List   { package, full }           => run_list(&config.path_source, package, full),
    };
    rs // need let.. and this for rustfmt::skip not error
}

fn run_list(src_path: &Path, req_pk: Option<String>, full_list: bool) -> AppResult {
    let curr_pks = get_all_packages(src_path)?;

    // Request to print only one package
    if let Some(pk_name) = req_pk {
        if let Some(pk) = curr_pks.get(&pk_name) {
            if let Ok(tree) = tree(pk) {
                println!("\nPackage:\n{tree}")
            }
        } else {
            print_all_packages(curr_pks, false);
            return Err(ErKind::NotFoundPackage(pk_name));
        }
    } else {
        // No package provide by user, print all the packages that current in source
        // And only print package NAME if not need `full_list`
        print_all_packages(curr_pks, full_list);
    }

    Ok(None)
}

fn run_sync(
    mut cfg: Config,
    usr_req: Option<Vec<String>>,
    diff_only: bool,
    force: bool,
) -> AppResult {
    let sys_pks: &Vec<String> = cfg.packages.as_ref();
    let req_pks = match usr_req.as_ref() {
        Some(p) => p,
        None if sys_pks.is_empty() => return Err(ErKind::SysEmpty(Some(cfg.path_source))),
        None => sys_pks,
    };
    let num_req = req_pks.len();

    let mut src_pks = get_all_packages(&cfg.path_source)?;
    let num_total = src_pks.len();
    let sync_pks: Vec<(String, PathBuf)> = req_pks
        .iter()
        .filter_map(|name| get_pkg_by_name(&mut src_pks, name))
        .flatten()
        .collect();

    if sync_pks.is_empty() {
        return Err(ErKind::NoValidPackage(Some(cfg.path_source)));
    }

    let num_req = num_req.max(sync_pks.len());
    let mut success = 0;
    let mut new_pks: Vec<String> = Vec::new();
    for (pk_name, pk_path) in sync_pks {
        let name = console::style(&pk_name).blue();
        if diff_only {
            println!("\nStatus package [[ {name} ]]:");
        } else {
            log::info(format!("syncing package '{name}':"));
        }

        if let Err(err) = sync_package(&cfg.path_root, &pk_path, cfg.resolver, diff_only, force) {
            log::error(format!("package '{}' failed to sync: {err}", name))
        } else if !diff_only {
            if usr_req.is_some() && !sys_pks.contains(&pk_name) {
                new_pks.push(pk_name);
            }
            success += 1
        };
    }

    if diff_only {
        return Ok(None);
    }

    println!(); // for spacing in terminal, easier to read.
    log::sucess(format!(
        "synced [ {}/{} ] over total [{}] packages.",
        console::style(success).green(),
        console::style(num_req).green(),
        console::style(num_total).blue(),
    ));

    if !new_pks.is_empty() {
        let pks: &mut Vec<String> = cfg.packages.as_mut();
        pks.extend(new_pks);
        pks.sort();

        Ok(Some(cfg))
    } else {
        Ok(None)
    }
}

fn sync_package(
    root_path: &Path,
    pk_path: &Path,
    resolver: Resolver,
    diff_only: bool,
    force: bool,
) -> ResErr {
    let pk_files = get_files_in_path(pk_path, None)?;

    for f_stowed in pk_files {
        // get equivalent file on the system
        let p_relative = f_stowed
            .strip_prefix(pk_path)
            .expect("f_stowed get from globbing pkg_dir, this never fail");
        let f_sys = root_path.join(p_relative);

        // IF: file on systems is a symlink to the file on src package,
        // (aka. already stowed), simply ignore it.
        // WHEN: user just want to check differences, print result only
        let is_same_file = fileio::is_the_same_file(&f_sys, &f_stowed);
        if diff_only {
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
        match force || resolver.is_replace() {
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

fn run_stow(mut cfg: Config, pk_name: String, paths: Vec<PathBuf>) -> AppResult {
    // FIRST: Get the path of the package in current src packages
    // IF: the request package not found, or no packages at all (start fresh)
    // ask user to create a new one. or quit.
    let mut is_pk_new = false;
    let pk_path = {
        let res = get_all_packages(&cfg.path_source).and_then(|pks| {
            Ok(pks
                .get(&pk_name)
                .ok_or(ErKind::NotFoundPackage(pk_name.to_string()))?
                .to_path_buf())
        });

        match res {
            Ok(path) => path,
            Err(err) => {
                match err {
                    ErKind::NotFoundPackage(_) => err.print(None),
                    ErKind::SourceEmpty => {}
                    _ => return Err(err),
                }

                log::ask_confirm(
                    format!("Create new package with name [{}]:", &pk_name),
                    false,
                    false,
                )?;

                let path = cfg.path_source.join(&pk_name);
                fs::create_dir(&path)
                    .expect("pkg not exists, we create a new one, this should not fail.");
                is_pk_new = true;
                path
            }
        }
    };

    // expand that directory into all the files that in it.
    let files: Vec<PathBuf> = paths
        .into_iter()
        .filter_map(|p| {
            let p = p
                .canonicalize()
                .inspect_err(|err| {
                    let msg = format!(
                        "not valid path: '{}': {err}",
                        console::style(p.display()).blue()
                    );
                    log::skip(msg);
                })
                .ok()?;

            if p.is_dir() {
                get_files_in_path(&p, None).ok()
            } else {
                Some(vec![p])
            }
        })
        .flatten()
        .collect();

    log::info(format!(
        "stowing [{:02}] files: {:#?} to package: '{}'",
        console::style(files.len()).green(),
        files,
        console::style(&pk_name).blue()
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
    let rootp = &cfg.path_root;
    for fsys in files {
        // for log only
        let fmt_fsys = console::style(fsys.display()).blue();
        let fmt_root = console::style(rootp.display()).cyan();

        let Ok(rel_path) = fsys.strip_prefix(rootp) else {
            log::skip(format!("'{fmt_fsys}' not in root dir: '{fmt_root}'"));
            continue;
        };
        let fstow_dest = pk_path.join(rel_path);

        // Check `f_sys` is not alredy stowed.
        if fileio::is_the_same_file(&fsys, &fstow_dest) {
            log::skip(format!("file '{fmt_fsys}' is already stowed!!"));
            continue;
        }

        /* From now, when error, it's system runtime error,
        the error will mark as fatal and stop execution. */
        let parent_path = fstow_dest.parent().expect("this should never fail.");
        if let Err(e) = fs::create_dir_all(parent_path)
            && matches!(e.kind(), io::ErrorKind::AlreadyExists)
        {
            #[rustfmt::skip]
            return Err(ErKind::IOFailWrite(parent_path.to_path_buf(), e.to_string()));
        }

        // WHEN: adopt, file on system will replace file in src
        //  use COPY and then REMOVE here bc: in case of f_sys is
        //  a symlink to somewhere else, then content will be copy
        //  and then only remove the f_sys.
        // (original src that f_sys link from is left untouch)
        if (!fstow_dest.exists() || cfg.resolver.is_adopt())
            && let Err(e) = fs::copy(&fsys, &fstow_dest)
        {
            let msg = format!("copy from {fmt_fsys} err: {e}");
            return Err(ErKind::IOFailWrite(fstow_dest, msg));
        }

        if let Err(e) = fs::remove_file(&fsys) {
            let msg = format!("remove err: {e}");
            return Err(ErKind::IOFailWrite(fsys, msg));
        }

        if let Err(e) = symlink_rs::symlink_file(&fstow_dest, &fsys) {
            log::fatal(format!(
                "cannot make symlink '{}' -> '{}': {e}",
                fsys.display(),
                fstow_dest.display()
            ))
        }

        log::sucess(format!(
            "stowed: '{}'",
            console::style(rel_path.display()).blue()
        ));
        success += 1;

        // TODO: maybe do cleanup if fail halfway.
    }

    println!(); // for spacing in terminal, easier to read.
    log::sucess(format!(
        "stowed [ {}/{} ] to package: '{}'",
        success,
        total,
        console::style(&pk_name).blue()
    ));

    if success > 0 && is_pk_new {
        cfg.packages.push(pk_name);
        cfg.packages.sort();
        return Ok(Some(cfg));
    }
    Ok(None)
}

fn run_remove(
    mut cfg: Config,
    usr_req_pk: Option<String>,
    usr_req_files: Option<Vec<PathBuf>>,
    purge: bool,
    is_remove_all: bool,
) -> AppResult {
    let mut src_pks = get_all_packages(&cfg.path_source)?;
    let rm_results = if let Some(pk_name) = usr_req_pk {
        // let Some(pk_path) = src_pks.get(&pk_name) else {
        //     return Err(ErrType::Generic(format!(
        //         "package '{}' not in <source>",
        //         console::style(pk_name).blue()
        //     )));
        // };

        let (pk_name, pk_path) = get_pkg_by_name(&mut src_pks, &pk_name)
            .ok_or(ErKind::NotFoundPackage(pk_name))?
            .swap_remove(0);

        // This will store all the files that can (and will) be remove
        // And this only store the stowed file (not path to symlink on sys)
        // The path to symlink will be concatinate later when doing remove.
        let mut rm_files: Vec<PathBuf> = Vec::new();
        match usr_req_files {
            Some(f) => {
                // When user provide file(s) to remove.
                // TODO: use collect to improve this.
                for p_file in f {
                    // EVERYTHING NOT SUCCESS WILL FALL TO THE BOTTOM.
                    if let Ok(f_stowed) = p_file.canonicalize()
                        && f_stowed.strip_prefix(&pk_path).is_ok()
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
                            get_files_in_path(&pk_path, Some(pattern)).ok()
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
                rm_files = get_files_in_path(&pk_path, None)?;
            }
        };
        // Print result here and wait for user comfirmation
        if rm_files.is_empty() {
            // log::info("nothing to remove!");
            return Err(ErKind::NoValidPackage(None));
        }
        println!("\nRemovable files:");
        for file in &rm_files {
            let p = file.strip_prefix(&pk_path).unwrap().display();
            log::sucess(format!("'../{p}'"));
        }
        log::ask_confirm("Remove all files:", false, false)?;
        rm_package(&cfg.path_root, &pk_name, &pk_path, rm_files, purge)?;

        // try to remove the package, if every files has been deleted
        // the package dir will be remove, othewise, ignore
        let rs = fileio::rm_empty_dirs(&pk_path);
        vec![(pk_name, rs)]
    } else {
        // Package is alway provide before file(s)
        // if NOT Package, files is also None (no need to check)
        if !is_remove_all {
            return Err(ErKind::Generic(
                "nothing to remove\nuse '--all' if you want to remove all packages",
            ));
        } // TODO: maybe add str field
        // TODO: maybe list all the current pkgs for user to confirm.
        #[cfg(any())]
        for (pk_name, pk_path) in src_pks.iter() {
            // match get_files_in_path(pk_path, None) {
            //     None => log::skip(format!("{}: package empty", console::style(pk_name).blue())),
            //     Some(pk_files) => {
            //         rm_package(&config.path_root, pk_name, pk_path, pk_files, purge)?;
            //         fileio::rm_empty_dirs(pk_path)?;
            //     }
            // }

            let pk_files = get_files_in_path(pk_path, None)?;
            rm_package(&config.path_root, pk_name, pk_path, pk_files, purge)?;
            fileio::rm_empty_dirs(pk_path)?;
        }
        src_pks
            .into_iter()
            .map(|(name, path)| {
                let a = get_files_in_path(&path, None)
                    .and_then(|files| rm_package(&cfg.path_root, &name, &path, files, purge))
                    .and_then(|_| fileio::rm_empty_dirs(&path));

                // let a = fileio::rm_empty_dirs(path);

                (name, a)
            })
            .collect()
    };

    // TODO: do this can really happen??
    if rm_results.is_empty() {
        return Ok(None);
    }

    for (pkg, rm_result) in rm_results.into_iter() {
        match rm_result {
            Err(ErKind::PackageNotEmpty) => {}
            Err(err) => {
                log::error(err);
            }
            Ok(_) => {
                if let Ok(a) = cfg.packages.binary_search(&pkg) {
                    cfg.packages.swap_remove(a);
                }
            }
        }
    }
    cfg.packages.sort();
    Ok(Some(cfg))
}

fn rm_package(
    path_root: &Path,
    pk_name: &str,
    pk_path: &Path,
    rm_files: Vec<PathBuf>,
    purge: bool,
) -> ResErr {
    println!();
    #[rustfmt::skip]
    log::info(format!("run remove on: '{}'", console::style(pk_name).blue()));

    for f_stowed in rm_files {
        // get equivalent file on the system
        let path_rel = f_stowed
            .strip_prefix(pk_path)
            .expect("path is already evalutated, this never fail");
        let f_sys = path_root.join(path_rel);

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
            let msg = format!("{} file: {err}", if purge { "remove" } else { "restore" });
            return Err(ErKind::IOFailWrite(f_stowed, msg));
        }
    }

    Ok(())
}

/// Extract pkgs info from hashmap if match
/// Also print out info.
fn get_pkg_by_name(
    src_pks: &mut FxHashMap<String, PathBuf>,
    name: impl AsRef<str>,
) -> Option<Vec<(String, PathBuf)>> {
    let pks: Vec<(String, PathBuf)> = src_pks
        .extract_if(|k, _| wildmatch::WildMatch::new(name.as_ref()).matches(k))
        .collect();

    let num = pks.len();
    if num == 0 {
        log::error(format!(
            "not found in <source>, package: '{}'",
            console::style(name.as_ref()).blue()
        ));
        return None;
    } else if num > 1 {
        let name = console::style(name.as_ref()).blue();
        // TODO: maybe print out expansion name.
        log::info(format!("expand to [{num:02}] packages /w key: '{}'", name));
    }
    Some(pks)
}

/// Get all the packages that current in source dir
fn get_all_packages(src_dir: &Path) -> ResType<FxHashMap<String, PathBuf>> {
    // GET EVERY DIR IN SOURCE THAT NOT START WITH '.'
    let entries = src_dir
        .read_dir()
        .map_err(|err| {
            let msg = format!("get all packages: {err}");
            ErKind::IOFailRead(src_dir.to_path_buf(), msg)
        })?
        .flatten();

    let pkgs: FxHashMap<String, PathBuf> = entries
        .filter_map(|f| {
            f.metadata().ok()?.is_dir().then(|| {
                let n = f.file_name();
                if n.as_bytes().first()? == &b'.' {
                    return None;
                }
                let name = n.into_string().ok()?;
                Some((name, f.path()))
            })?
        })
        .collect();

    if pkgs.is_empty() {
        Err(ErKind::SourceEmpty)
    } else {
        Ok(pkgs)
    }
}

/// Get all files in provided path.
fn get_files_in_path(path: &Path, pattern: Option<&str>) -> ResType<Vec<PathBuf>> {
    let req_name = pattern.unwrap_or("*");
    let pat = format!("{}/**/{req_name}", path.display(),);
    let globs: Vec<PathBuf> = glob::glob(&pat)
        .expect("pattern checked") // TODO: check for bad pattern
        .flatten()
        .filter(|p| p.is_file())
        .collect();

    if !globs.is_empty() {
        Ok(globs)
    } else if !matches!(req_name, "*") {
        let pk_name = path.file_name().unwrap().to_os_string();
        Err(ErKind::NotFoundFile(pk_name, req_name.to_string()))
    } else {
        Err(ErKind::Generic("package empty!!!"))
    }
}

fn print_all_packages(packages: FxHashMap<String, PathBuf>, full_list: bool) {
    println!(
        "Available Packages in <source>: [{:02}]",
        console::style(packages.len()).green()
    );

    if full_list {
        // Display everythings
        for (name, pk_path) in packages.iter() {
            match tree(pk_path) {
                Ok(tree) => println!("{tree}"),
                Err(e) => log::error(format!(
                    "cannot display package: '{}': {e}",
                    console::style(name).blue()
                )),
            }
        }
    } else {
        // Name of packages only: print 5 items in one row
        let total = packages.len();
        for (i, p) in packages.keys().sorted().enumerate() {
            let name = console::style(p).blue();
            if (i + 1) % 5 == 0 || i == total - 1 {
                println!("> {}", name)
            } else {
                print!("> {:17}", name)
            }
        }
    }
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
