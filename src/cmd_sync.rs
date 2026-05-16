use std::fs;
use std::path::Path;
use std::path::PathBuf;

use crate::AppResult;
use crate::ResErr;
use crate::config::Config;
use crate::config::Resolver;
use crate::error::ErKind;
use crate::fileio;
use crate::log;
use crate::util;

pub fn run(
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

    let mut src_pks = util::get_all_packages(&cfg.path_source)?;
    let num_total = src_pks.len();
    let sync_pks: Vec<(String, PathBuf)> = req_pks
        .iter()
        .filter_map(|name| util::get_pkg_by_name(&mut src_pks, name))
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
    use std::io::ErrorKind::NotFound;

    let pk_files = util::get_files_in_path(pk_path, None)?;

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
                    && e.kind() != NotFound
                {
                    log::warn(format!("cannot delete file '{}': {e}", f_sys.display()));
                    continue;
                }
            }
            false => {
                if let Err(e) = fs::rename(&f_sys, &f_stowed)
                    && e.kind() != NotFound
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
