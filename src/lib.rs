/* ===================================================== */
/* ERROR TYPES ========================================= */
pub mod error {
    pub type ResErr = Result<(), ErrType>;
    pub type ResType<T> = Result<T, ErrType>;

    #[derive(Debug, PartialEq)]
    pub enum ErrType {
        Generic(String),
        FileRWFailed(String),
        BadConfigFile(String),
        ExternProgram(String),
    }

    impl std::fmt::Display for ErrType {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            let msg = match self {
                Self::Generic(s) => s,
                Self::ExternProgram(s) => s,
                Self::FileRWFailed(s) => s,
                Self::BadConfigFile(s) => s,
            };
            std::fmt::write(f, format_args!("{msg}"))
        }
    }

    impl ErrType {
        pub fn print_err(&self) {
            super::log::error(self);
        }

        /// Mark error as `Fatal`, print error and exit program
        pub fn fatal_err(&self) -> ! {
            super::log::fatal(self)
        }
    }
}

/* ===================================================== */
/* UTILS =============================================== */
#[allow(dead_code)]
pub mod log {
    use console::style;
    use core::fmt::Display;

    macro_rules! printout {
        ($msg:expr, $ctx:expr) => {
            println!("[{}] -- {}", $ctx, $msg)
        };
    }

    #[inline]
    pub fn fatal<S: Display>(msg: S) -> ! {
        printout!(msg, style("fatal").yellow().on_red());
        std::process::exit(-1)
    }

    #[inline]
    pub fn error<S: Display>(msg: S) {
        printout!(msg, style("error").red());
    }

    #[inline]
    pub fn warn<S: Display>(msg: S) {
        printout!(msg, style("warn").yellow());
    }

    #[inline]
    pub fn skip<S: Display>(msg: S) {
        printout!(msg, style("skipped").yellow());
    }

    #[inline]
    pub fn sucess<S: Display>(msg: S) {
        printout!(msg, style(" ok ").green());
    }

    #[inline]
    pub fn info<S: Display>(msg: S) {
        printout!(msg, style("info").cyan());
    }
}

pub mod fileio {
    use super::error::{ErrType, ResErr};
    use std::{path::Path, process::Command};

    pub fn run_program<I, S>(cmd: &str, args: I) -> Result<(), ErrType>
    where
        I: IntoIterator<Item = S>,
        S: AsRef<std::ffi::OsStr>,
    {
        match Command::new(cmd).args(args).status() {
            Err(e) => Err(ErrType::ExternProgram(format!("cannot run '{cmd}': {e}"))),
            Ok(_) => Ok(()),
        }
    }

    /// Try to create a directory, skip if already exist
    /// Otherwise return Err if failed
    pub fn try_make_dir(dest: &Path, verbose: bool) -> ResErr {
        if let Err(e) = std::fs::create_dir_all(dest) {
            if e.kind() != std::io::ErrorKind::AlreadyExists {
                return Err(ErrType::FileRWFailed(format!(
                    "cannot make dir '{}': {}",
                    dest.display(),
                    e
                )));
            }

            if verbose {
                super::log::skip(format!(
                    "make path '{}' already exists",
                    dest.to_str().unwrap()
                ));
            }
        }

        Ok(())
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

    pub fn symlink_to_dir(d_src: &Path, d_dest: &Path, f_name: &str) -> ResErr {
        let f_src = d_src.join(f_name);
        let f_dst = d_dest.join(f_name);

        if let Ok(f) = f_dst.read_link() {
            if f == f_dst {
                return Ok(());
            }

            if let Err(e) = std::fs::remove_file(&f_dst) {
                return Err(ErrType::FileRWFailed(format!(
                    "cannot remove old symlink ->'{f_name}': {e}"
                )));
            }
        }

        match symlink_rs::symlink_file(f_src, f_dst) {
            Err(e) => Err(ErrType::FileRWFailed(format!(
                "cannot make symlink ->'{f_name}': {e}"
            ))),
            Ok(_) => Ok(()),
        }
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
    /// - `False` when f_link is not a link, or not exists,
    ///   or point to different file.
    ///
    pub fn is_link_to_same_file(f_link: &Path, f_src: &Path) -> bool {
        if let Ok(p1) = f_link.canonicalize()
            && let Ok(p2) = f_src.canonicalize()
        {
            p1 == p2
        } else {
            false
        }
    }
}
