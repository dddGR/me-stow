use std::fs;
use std::path::Path;
use std::path::PathBuf;

use crate::AppResult;
use crate::ResErr;
use crate::config;
use crate::error::ErKind;
use crate::fileio;
use crate::messages as ms;
use crate::util;

pub fn run(
    mut cfg: config::Config,
    usr_req_pk: Option<String>,
    usr_req_files: Option<Vec<PathBuf>>,
    purge: bool,
    is_remove_all: bool,
) -> AppResult {
    let mut src_pks = util::get_all_packages(&cfg.path_source)?;
    let rm_results = match usr_req_pk {
        None => {
            // Package is alway provide before file(s)
            // if NOT Package, files is also None (no need to check)
            if !is_remove_all {
                return Err(ErKind::Generic(
                    "nothing to remove\nuse '--all' if you want to remove all packages",
                ));
            } // TODO: maybe add str field
            // TODO: maybe list all the current pkgs for user to confirm.

            src_pks
                .into_iter()
                .map(|(name, path)| {
                    let a = util::get_files_in_path(&path, None)
                        .and_then(|files| rm_package(&cfg.path_root, &name, &path, files, purge))
                        .and_then(|_| fileio::rm_empty_dirs(&path));

                    (name, a)
                })
                .collect()
        }
        Some(pk_name) => {
            let (pk_name, pk_path) = util::get_pkg_by_name(&mut src_pks, &pk_name)
                .ok_or(ErKind::NotFoundPackage(pk_name))?
                .swap_remove(0);

            // This will store all the files that can (and will) be remove
            // And this only store the stowed file (not path to symlink on sys)
            // The path to symlink will be concatinate later when doing remove.
            let mut rm_files: Vec<PathBuf> = Vec::new();
            match usr_req_files {
                None => {
                    // User not provide file to remove
                    // Remove all files in package
                    rm_files = util::get_files_in_path(&pk_path, None)?;
                }
                Some(files) => {
                    // When user provide file(s) to remove.
                    // TODO: use collect to improve this.
                    for path in files {
                        // EVERYTHING NOT SUCCESS WILL FALL TO THE BOTTOM.
                        if let Ok(f_stowed) = path.canonicalize()
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
                            let glob_finds = path.to_str().and_then(|p| {
                                let pattern = p.trim_start_matches('/');
                                util::get_files_in_path(&pk_path, Some(pattern)).ok()
                            });

                            if let Some(files) = glob_finds {
                                rm_files.extend(files);
                                continue;
                            }
                        }

                        ms::skip!("not valid file: '{}'", path.display());
                    }
                }
            };

            // Print result here and wait for user comfirmation
            if rm_files.is_empty() {
                return Err(ErKind::NoValidPackage(None));
            }
            println!("\nRemovable files:");
            for file in &rm_files {
                let p = file.strip_prefix(&pk_path).unwrap().display();
                ms::success!("'../{}'", p);
            }
            ms::ask_confirm("Remove all files:", false, false)?;
            rm_package(&cfg.path_root, &pk_name, &pk_path, rm_files, purge)?;

            // try to remove the package, if every files has been deleted
            // the package dir will be remove, othewise, ignore
            let rs = fileio::rm_empty_dirs(&pk_path);
            vec![(pk_name, rs)]
        }
    };

    // TODO: do this can really happen??
    if rm_results.is_empty() {
        return Ok(None);
    }

    for (pkg, rm_result) in rm_results.into_iter() {
        match rm_result {
            Err(ErKind::PackageNotEmpty) => {}
            Err(err) => ms::error!(err),
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
    ms::info!("run remove on: '{}'", ms::blue!(pk_name));
    for f_stowed in rm_files {
        // get equivalent file on the system
        let path_rel = f_stowed
            .strip_prefix(pk_path)
            .expect("path is already evalutated, this should not fail");
        let f_sys = path_root.join(path_rel);

        // WHEN: `purge`, simply remove everything.
        // ELSE: copy file in src package to override file
        // on the system (opposite with when stow)
        let result = match purge {
            true => fs::remove_file(&f_stowed).and_then(|_| fs::remove_file(f_sys)),
            false => {
                // WHEN: `f_sys` is not exists or not link to `f_stowed`
                // this mean the file only in the stow pkg, and not related
                // to the file on the system (if that exists).
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
