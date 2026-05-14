/* ----------------------------------------------------- */
/* ERROR TYPES ----------------------------------------- */
pub mod error {
    use crate::log;
    use std::{
        ffi::OsString,
        fmt::{self, Debug},
        path::{Path, PathBuf},
    };
    pub type ResErr = Result<(), ErKind>;
    pub type ResType<T> = Result<T, ErKind>;

    #[derive(Debug, PartialEq)]
    #[non_exhaustive]
    pub enum ErKind {
        UserAbort(Option<&'static str>),
        Generic(&'static str),
        NotFoundConfig,
        SourceEmpty,
        /// After validation, no request package from user is valid.
        /// @ret: `src_dir` to list all packages available in src.
        NoValidPackage(Option<PathBuf>),
        /// Current system have no package that are in sync.
        /// @ret: `src_dir` to list all packages available in src.
        SysEmpty(Option<PathBuf>),
        IOFailRead(PathBuf, String),
        IOFailWrite(PathBuf, String),
        /// File/Dir already exists, and not symlink.
        IOExisted(PathBuf),
        BadConfigFile(String),
        /// Call external program but failed to execute.
        /// @ret: `program name`, `exit code`, `error message`.
        ExternProgram(String, i32, String),
        NotFoundPackage(String),
        /// Cannot find requested file(s) in the package.
        /// @ret: `package name` and `pattern` try to matches.
        NotFoundFile(OsString, String),
        PackageNotEmpty,
    }

    impl std::error::Error for ErKind {}

    impl fmt::Display for ErKind {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            #[allow(unused_imports)]
            use ErKind::*;
            use console::style;
            match self {
                SourceEmpty => write!(f, "no package in <source>"),
                NotFoundConfig => write!(f, "not found configurations file!!"),
                NoValidPackage(_) => write!(f, "nothing to run!"),
                SysEmpty(_) => write!(f, "no package currently in sync on this system!"),
                PackageNotEmpty => write!(f, "there are files remain in package!!"),
                BadConfigFile(msg) => write!(f, "invalid config: {msg}"),
                Generic(msg) => write!(f, "{msg}"),
                UserAbort(msg) => {
                    write!(f, "{}", msg.unwrap_or("user abort, peacefully exit!!"))
                }
                IOFailRead(file, msg) => {
                    let p = style(Self::trim_path(file)).blue();
                    write!(f, "read from '{p}': {msg}")
                }
                IOFailWrite(file, msg) => {
                    let p = style(Self::trim_path(file)).blue();
                    write!(f, "write to '{p}': {msg}")
                }
                IOExisted(file) => {
                    let p = style(Self::trim_path(file)).blue();
                    write!(f, "already exists: '{p}' and it not a symlink!")
                }

                NotFoundFile(pkg, file) => {
                    let fi = style(file).cyan();
                    let pk = style(pkg.display()).blue();
                    write!(f, "not found file '{fi}' in package: {pk}")
                }
                ExternProgram(name, code, msg) => {
                    let n = style(name).blue();
                    write!(f, "external '{n}' exit /w code: {code} error: {msg}")
                }
                NotFoundPackage(pkg) => {
                    let name = console::style(pkg).blue();
                    write!(f, "request package '{name}' not in <source>")
                } // _ => write!(f, "not implemented yet"),
            }
        }
    }

    impl ErKind {
        pub fn print(&self, exit: Option<i32>) {
            if matches!(self, Self::UserAbort(_)) {
                log::info(self);
            } else {
                log::error(self);
            }

            if let Some(code) = exit {
                std::process::exit(code);
            }
        }

        fn trim_path(path: &Path) -> String {
            // TODO: this is a mess
            if path.components().count() < 3 {
                path.display().to_string()
            } else {
                let n0 = path.file_name().unwrap();
                let p1 = path.parent().unwrap();
                let n1 = p1.file_name().unwrap();
                let n2 = p1.parent().unwrap().file_name().unwrap();

                format!("../{}/{}/{}", n2.display(), n1.display(), n0.display())
            }
        }
    }
}

/* ----------------------------------------------------- */
/* UTILS ----------------------------------------------- */
#[allow(dead_code)]
pub mod log {
    use console::style;
    use core::fmt::Display;
    use dialoguer::{Confirm, theme::ColorfulTheme};

    use crate::error::{ErKind, ResType};

    macro_rules! pstd {
        ($msg:expr, $ctx:expr) => {
            println!("[{}] -- {}", $ctx, $msg)
        };
    }
    macro_rules! pstderr {
        ($msg:expr, $ctx:expr) => {
            eprintln!("[{}] -- {}", $ctx, $msg)
        };
    }

    #[inline]
    pub fn fatal<S: Display>(msg: S) -> ! {
        pstderr!(msg, style("fatal").yellow().on_red());
        std::process::exit(-1)
    }

    #[inline]
    pub fn error<S: Display>(msg: S) {
        pstderr!(msg, style("error").red());
    }

    #[inline]
    pub fn warn<S: Display>(msg: S) {
        pstd!(msg, style("warn").yellow());
    }

    #[inline]
    pub fn skip<S: Display>(msg: S) {
        pstd!(msg, style("skipped").yellow());
    }

    #[inline]
    pub fn sucess<S: Display>(msg: S) {
        pstd!(msg, style(" ok ").green());
    }

    #[inline]
    pub fn info<S: Display>(msg: S) {
        pstd!(msg, style("info").cyan());
    }

    #[inline]
    pub fn msg<S: Display>(msg: S) {
        pstd!(msg, " .. ");
    }

    /// Ask user to confirmation before continue.
    /// With msg promt and default answ.
    /// When not `false_continue`, this will output `Err()`
    /// that can be use with `?` to exit function
    pub fn ask_confirm<S: Into<String>>(
        msg: S,
        default_yes: bool,
        false_continue: bool,
    ) -> ResType<bool> {
        let choice = Confirm::with_theme(&ColorfulTheme::default())
            .with_prompt(msg)
            .default(default_yes)
            .interact()
            .unwrap_or(false);

        if choice || false_continue {
            Ok(choice)
        } else {
            Err(ErKind::UserAbort(None))
        }
    }
}

pub mod fileio {
    use crate::log;

    use super::error::{ErKind, ResErr};
    use std::{fs, io, path::Path, process::Command};

    pub fn run_program<I, S>(cmd: &str, args: I) -> ResErr
    where
        I: IntoIterator<Item = S>,
        S: AsRef<std::ffi::OsStr>,
    {
        let a = Command::new(cmd).args(args).output();

        // TODO: check this.
        match a {
            Ok(output) => {
                let a = output.status;

                if a.success() {
                    Ok(())
                } else {
                    let msg = output.stderr.into_iter().map(|c| c.to_string()).collect();

                    Err(ErKind::ExternProgram(
                        cmd.to_string(),
                        a.code().unwrap_or(0xAF),
                        msg,
                    ))
                }
            }
            Err(e) => Err(ErKind::ExternProgram(cmd.to_string(), 0xAF, e.to_string())),
        }

        // match Command::new(cmd).args(args).status() {
        //     Err(e) => Err(ErKind::ExternProgram(cmd.to_string(), -1, e.to_string())),
        //     Ok(_) => Ok(()),
        // }
    }

    /// Try to create a directory, skip if already exist
    /// Otherwise return Err if failed
    #[inline]
    pub fn try_make_dir(dest: &Path, verbose: bool) -> ResErr {
        use std::io::ErrorKind::AlreadyExists;

        std::fs::create_dir_all(dest).or_else(|err| {
            if matches!(err.kind(), AlreadyExists) {
                if verbose {
                    let m = format!("already exists: {}", dest.display());
                    log::skip(m);
                }
                Ok(())
            } else {
                let m = format!("failed makedir: {err}");
                Err(ErKind::IOFailWrite(dest.to_path_buf(), m))
            }
        })
    }

    /// Get all the files in `dir` with that type `ext` and move into `dest`
    #[inline]
    pub fn move_file_with_ext(dir: &Path, ext: &str, dest: &Path) {
        for entry in dir.read_dir().expect("read_dir call failed").flatten() {
            let f_dir = entry.path();
            if f_dir
                .extension()
                .unwrap_or_default()
                .to_str()
                .unwrap_or_default()
                .cmp(ext)
                .is_eq()
                && let Err(e) = std::fs::rename(&f_dir, dest.join(entry.file_name()))
            {
                super::log::error(format!("move '{}' with: {}", f_dir.to_string_lossy(), e));
            }
        }
    }

    /// Make a symlink `d_dest/f_name`, point to `d_src/f_name`.
    #[cfg(false)]
    pub fn symlink_to_dir(d_src: &Path, d_dest: &Path, f_name: &str) -> ResErr {
        use std::io::ErrorKind;
        let f_src = d_src.join(f_name);
        let f_dst = d_dest.join(f_name);

        match f_dst.read_link() {
            Err(e) if matches!(e.kind(), ErrorKind::InvalidInput) => {
                // f_dst is a file.
                return Err(ErKind::IOExisted(f_dst));
            }
            Err(e) if matches!(e.kind(), ErrorKind::NotFound) => {}
            Err(e) => {
                return Err(ErKind::IOFailRead(f_dst, e.to_string()));
            }
            Ok(_) => {
                if let Err(e) = std::fs::remove_file(&f_dst) {
                    let m = format!("remove symlink ->'{f_name}': {e}");
                    return Err(ErKind::IOFailWrite(f_dst.clone(), m));
                }
            }
        }

        // match symlink_rs::symlink_file(f_src, f_dst) {
        //     Err(e) => Err(ErrType::FileRWFailed(format!(
        //         "cannot make symlink ->'{f_name}': {e}"
        //     ))),
        //     Ok(_) => Ok(()),
        // }

        symlink_rs::symlink_file(&f_src, &f_dst).map_err(|e| {
            let m = format!("make symlink ->'{f_name}: {e}");
            ErKind::IOFailWrite(f_dst, m)
        })
    }

    /// Read symlink and check if the file symlink point to is the
    /// same file as `f_src`.
    ///
    /// ## Arguments
    ///
    /// - `f_link` (`&Path`) - link file to check.
    /// - `f_src` (`&Path`) - source file to check.
    ///
    /// ## Returns
    ///
    /// - `True` if is the same file,
    /// - `False` when `f_link` or `f_src` not exists,
    ///   or point to different file.
    ///
    pub fn is_the_same_file(f_link: &Path, f_src: &Path) -> bool {
        if let Ok(p1) = f_link.canonicalize()
            && let Ok(p2) = f_src.canonicalize()
        {
            p1 == p2
        } else {
            false
        }
    }

    /// Cleanup directory, remove any sub dirs that are empty.
    /// Also remove provided dir if is empty after cleanup.
    pub fn rm_empty_dirs(path: &Path) -> ResErr {
        // TODO: handle symlink to a dir
        if !path.is_symlink() {
            #[cfg(not(windows))]
            let ret = rm_empty_dir_unix(path);

            #[cfg(windows)]
            let ret = rm_empty_dir_wind(path);

            ret.map_err(|err| {
                if matches!(err.kind(), io::ErrorKind::DirectoryNotEmpty) {
                    ErKind::PackageNotEmpty
                } else {
                    ErKind::IOFailWrite(path.to_path_buf(), err.to_string())
                }
            })
        } else {
            Err(ErKind::PackageNotEmpty)
        }
    }

    // ---------------------------------------------------------- //
    // ---------------------------------------------------------- //
    fn rm_empty_dir_unix(path: &Path) -> io::Result<()> {
        let subdirs = path
            .read_dir()?
            .flatten()
            .map(|e| e.path())
            .filter(|p| p.is_dir());

        for dir_path in subdirs {
            rm_empty_dir_unix(&dir_path)?;
        }

        fs::remove_dir(path)
    }

    #[cfg(windows)]
    fn rm_empty_dir_wind(_path: &Path) {
        unimplemented!()
    }
}
