use std::path::Path;
use std::path::PathBuf;

use itertools::Itertools as _;
use rustc_hash::FxHashMap;

use crate::AppResult;
use crate::error::ErKind;
use crate::messages as ms;
use crate::util;

pub fn run(src_path: &Path, req_pk: Option<String>, full_list: bool) -> AppResult {
    let curr_pks = util::get_all_packages(src_path)?;

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

pub fn print_all_packages(packages: FxHashMap<String, PathBuf>, full_list: bool) {
    println!(
        "Available Packages in <source>: [{:02}]",
        ms::green!(packages.len())
    );

    if full_list {
        // Display everythings
        for (name, pk_path) in packages.iter() {
            match tree(pk_path) {
                Ok(tree) => println!("{tree}"),
                Err(err) => ms::error!("cannot display package: '{}': {}", ms::blue!(name), err),
            }
        }
    } else {
        // Name of packages only: print 4 items in one row
        let total = packages.len();
        for (i, p) in packages.keys().sorted().enumerate() {
            if (i + 1) % 4 == 0 || i == total - 1 {
                println!("> {}", ms::blue!(p))
            } else {
                print!("> {:32}", ms::blue!(p))
            }
        }
    }
}

fn tree<P: AsRef<Path>>(p: P) -> std::io::Result<termtree::Tree<String>> {
    use termtree::Tree;

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

fn label<P: AsRef<Path>>(p: P) -> String {
    p.as_ref().file_name().unwrap().to_str().unwrap().to_owned()
}
