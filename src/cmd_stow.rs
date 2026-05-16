use std::fs;
use std::io;
use std::path::PathBuf;

use crate::AppResult;
use crate::config;
use crate::error::ErKind;
use crate::fileio;
use crate::messages as ms;
use crate::util;

pub fn run(mut cfg: config::Config, pk_name: String, paths: Vec<PathBuf>) -> AppResult {
    // FIRST: Get the path of the package in current src packages
    // IF: the request package not found, or no packages at all (start fresh)
    // ask user to create a new one. or quit.
    let mut is_pk_new = false;
    let fmt_pkname = ms::blue!(&pk_name);
    let pk_path = {
        let res = util::get_all_packages(&cfg.path_source).and_then(|pks| {
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

                let msg = format!("Create new package with name [{}]:", fmt_pkname);
                ms::ask_confirm(msg, false, false)?;

                let path = cfg.path_source.join(&pk_name);
                fs::create_dir(&path).expect("create single dir, can this be fail??");
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
                    ms::skip!("not valid path: '{}': {}", ms::blue!(p.display()), err)
                })
                .ok()?;

            match p.is_dir() {
                true => util::get_files_in_path(&p, None).ok(),
                false => Some(vec![p]),
            }
        })
        .flatten()
        .collect();

    // TODO: improve this!!
    ms::info!(
        "stowing [{:02}] files: {:#?} to package: '{}'",
        ms::green!(files.len()),
        files,
        ms::blue!(&pk_name)
    );

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
        let fmt_fsys = ms::blue!(fsys.display());
        let fmt_root = ms::cyan!(rootp.display());

        let Ok(rel_path) = fsys.strip_prefix(rootp) else {
            ms::skip!("'{}' not in root dir: '{}'", fmt_fsys, fmt_root);
            continue;
        };
        let fstow_dest = pk_path.join(rel_path);

        // Check `f_sys` is not alredy stowed.
        if fileio::is_the_same_file(&fsys, &fstow_dest) {
            ms::skip!("file '{}' is already stowed!!", fmt_fsys);
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
            let m = format!("make symlink -> '{}': {}", fmt_fsys, e);
            return Err(ErKind::IOFailWrite(fstow_dest, m));
        }

        success += 1;
        // ms::success!("stowed: '{}'", ms::blue!(rel_path.display()));
        // TODO: maybe do cleanup if fail halfway.
    }

    println!(); // for spacing in terminal, easier to read.
    ms::success!(
        "stowed [ {}/{} ] to package: '{}'",
        ms::green!(success),
        ms::green!(total),
        fmt_pkname
    );

    if success > 0 && is_pk_new {
        cfg.packages.push(pk_name);
        cfg.packages.sort();
        Ok(Some(cfg))
    } else {
        Ok(None)
    }
}
