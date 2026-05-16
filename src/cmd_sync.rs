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
        let name = ms::blue!(&pk_name);
        match diff_only {
            true => println!("Status package [[ {name} ]]:"),
            false => ms::info!("syncing package '{}':", name),
        }

        let syncrs = sync_package(&cfg.path_root, &pk_path, cfg.resolver, diff_only, force);
        match syncrs {
            Err(err) => ms::error!("package '{}' failed to sync: {}", name, err),
            Ok(_) if diff_only => println!(),
            Ok(_) => {
                if usr_req.is_some() && !sys_pks.contains(&pk_name) {
                    new_pks.push(pk_name);
                }
                success += 1
            }
        }
    }

    if diff_only {
        return Ok(None);
    }

    ms::success!(
        "synced [ {}/{} ] over total [{}] packages.",
        ms::green!(success),
        ms::green!(num_req),
        ms::blue!(num_total),
    );

    match new_pks.is_empty() {
        true => Ok(None),
        false => {
            let pks: &mut Vec<String> = cfg.packages.as_mut();
            pks.extend(new_pks);
            pks.sort();

            Ok(Some(cfg))
        }
    }
}

fn sync_package(
    root_path: &Path,
    pk_path: &Path,
    resolver: config::Resolver,
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
            let status = match is_same_file {
                true => ms::green!(" SYNCED "),
                false => ms::blue!("NOT SYNC"),
            };
            println!("[{}] -- '{}'", status, p_relative.display());
            continue;
        }

        if is_same_file {
            continue;
        }

        let fmt_fsys = ms::blue!(f_sys.display());
        let fmt_stowed = ms::yellow!(f_stowed.display());

        // WHEN: user want to force src file on to the system,
        // remove the file that currenly on the system (if any).
        // OTHERWISE: fallback to resolver method
        match force || resolver.is_replace() {
            true => {
                if let Err(err) = fs::remove_file(&f_sys)
                    && err.kind() != NotFound
                {
                    ms::warn!("cannot delete file '{}': {}", fmt_fsys, err);
                    continue;
                }
            }
            false => {
                if let Err(err) = fs::rename(&f_sys, &f_stowed)
                    && err.kind() != NotFound
                {
                    ms::warn!("cannot move file '{}': {}", fmt_fsys, err);
                    continue;
                }
            }
        }

        if let Err(err) = symlink_rs::symlink_file(&f_stowed, &f_sys) {
            ms::error!(
                "cannot make symlink '{}' -> '{}': {}",
                fmt_fsys,
                fmt_stowed,
                err
            );
            continue;
        }
        // ms::success!("stowed success: '{}'", fmt_fsys);
    }
    Ok(())
}
