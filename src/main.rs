use std::process::ExitCode;

use me_stow::AppResult;
use me_stow::cmd_list;
use me_stow::cmd_remove;
use me_stow::cmd_stow;
use me_stow::cmd_sync;
use me_stow::error::ErKind;

// TODO: list
// - f_sys is a link to a file that in the same dir(package).
// - file need to be stow that already in other stow pkg.
// - EXCLUDE matches pattern.
// - SYNC, but file on sys is broken symlink

fn main() -> ExitCode {
    match run_main() {
        Err(err) => {
            err.print(None);
            match err {
                ErKind::NoValidPackage(Some(src)) | ErKind::SysEmpty(Some(src)) => {
                    let _ = cmd_list::run(&src, None, false);
                }
                _ => {}
            }
            return ExitCode::FAILURE;
        }
        Ok(Some(config)) => {
            if let Err(err) = config.save() {
                err.print(None);
                return ExitCode::FAILURE;
            }
        }
        Ok(None) => {}
    }
    ExitCode::SUCCESS
}

fn run_main() -> AppResult {
    use me_stow::args_parser::{self, Command};
    use me_stow::config;

    let config = config::Config::new()?;
    let cli = args_parser::Cli::new();

    println!(); // for spacing in terminal, easier to read.
    #[rustfmt::skip]
    let rs = match cli.command {
        Command::Status { packages }
            => cmd_sync::run(config, packages, true, false),
        Command::Sync   { packages, diff, force }
            => cmd_sync::run(config, packages, diff, force),
        Command::Stow   { package, paths }
            => cmd_stow::run(config, package, paths),
        Command::Remove { packages, files, purge, all }
            => cmd_remove::run(config, packages, files, purge, all),
        Command::List   { package, full }
            => cmd_list::run(&config.path_source, package, full),
    };
    rs // need let.. and this for rustfmt::skip not error
}
