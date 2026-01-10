#!/usr/bin/env -S uv run --script
from pathlib import Path
from typing import List
import shutil as su
import sys
from classes import Params, ResolveType


# ============================================================= #
# ============================================================= #


def print_help(exit=False, exit_code=0) -> None:
    """
    Helper Messages
    """
    print("""
------------------------------------------------------------
Usage:
  me-stow [options] <packages>
    --init          TODO
      update        TODO
      remove        TODO

    --save-config   Save config to 'configs.json', this will automatic on
                    when run script the first time or when configs.json not found.

    --force         Override current file on system if conflicts

    --resolve=      Strategy to resolve conflict files
        replace     - Replace current file on system with a symlink,
                      similar with the flag --force
        adopt       - (default) Copy current file on system to source folder and
                      override file on source (this is like --adopt on stow),
                      then user can use git to compare (or restore) them.

    -v | --verbose  Vebose output
    -h | --help     Print help usage
Examples:
    me-stow --init

    me-stow --update --force

    me-stow some-package

    me-stow some-package --force

    me-stow more_package even-more-package
    
""")

    if exit:
        sys.exit(exit_code)


def err_print_help_exit(msg: str | None) -> None:
    """
    Simple wrapper to print error message and help text when error occur.
    """
    if msg:
        print(f"\n[error] -- {msg}")
    print_help(exit=True, exit_code=1)


# ============================================================= #
# GLOBAL ====================================================== #

CONFIG_FILE = Path(__file__).resolve().parent / "config.json"

# ============================================================= #
# ============================================================= #


def main():
    print("Running `me-stow`...")
    try:
        params = Params(CONFIG_FILE)
    except Warning:
        print_help(exit=True)
    except ValueError as e:
        err_print_help_exit(e)

    if params.verbose:
        params.print_all_packages()

    # TODO: op type
    for pkg in params.packages:
        process_stow_package(params.root, pkg, params.resolve)

    if params.save_config:
        params.save_configuration(CONFIG_FILE)

    # TODO: check for failure
    num = len(params.packages)
    print("...DONE" + (f" -- [{num}] packages stowed" if num > 1 else ""))


# ============================================================ #
# ============================================================ #


def process_stow_package(dest_dir: Path, package: Path, res_type: ResolveType) -> None:
    """
    Create a symlink from package (and all it's content) to destination directory.

    :NOTE: can run recursively
    """
    link_files: List[Path] = []
    for entry in package.iterdir():
        if entry.is_dir():
            new_dest = dest_dir / entry.name
            # TODO: If current dir on system is a symlink, happen if maybe use stow before
            if not new_dest.exists():
                new_dest.mkdir()
            # recursive call
            process_stow_package(new_dest, entry, resolve)

        if entry.is_file():
            link_files.append(entry)

    for file in link_files:
        dest_file = dest_dir / file.name
        try:
            # THIS ONLY PASS WHEN CONFLICTS HAPPEN
            if res_type == ResolveType.ADOPT:
                # Override source file with file current in system
                su.copyfile(dest_file, file)
            dest_file.unlink()
        except FileNotFoundError:
            pass

        dest_file.symlink_to(file)


if __name__ == "__main__":
    main()
