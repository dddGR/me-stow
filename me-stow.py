#!/usr/bin/env -S uv run --script
from pathlib import Path
from typing import List
import shutil as su
import sys
from classes import Operation, Params, ResolveType, Arguments


# ============================================================= #
# ============================================================= #


def print_help(exit=False, exit_code=0) -> None:
    """
    Helper Messages
    """
    print(f"""
------------------------------------------------------------
Usage:
  me-stow [options] <packages>
    <packages>      Can pass in one or multiple packages. If non specify
                    this script will process all the packages that in source dir.
    [options]
    --{Arguments.INIT}          Make symlink to the file that in source folder. This is
                    similar to `gnu stow`.
      {Arguments.UPDATE}        TODO
      {Arguments.REMOVE}        Delete the symlink file on the system, can be use with
                    `--copy-back` flag to replace symlink file with actual file.

    --{Arguments.SAVE_CONFIG}   Save config to 'configs.json', this will automatic on
                    when run script the first time or when configs.json not found.

    --{Arguments.FORCE}         Override current file on system if conflicts.

    --{Arguments.RESOLVE}=      Use with `init` operation, strategy to resolve conflict files.
        {ResolveType.REPLACE.value}     - Replace current file on system with a symlink,
                      similar with the flag --force.
        {ResolveType.ADOPT.value}       - (default) Copy current file on system to source folder and
                      override file on source (this is like --adopt on stow),
                      then user can use git to compare (or restore) them.

    --{Arguments.COPY_BACK}     Use with `remove` operation, this will copy file on source to
                    the link files on the system. Like replace symlink file with
                    actual file.

    -v | --{Arguments.VERBOSE}  Vebose output
    -h | --{Arguments.HELP}     Print help usage
Examples:
    me-stow --init

    me-stow --update --resolve=replace

    me-stow some-package

    me-stow more_package even-more-package

    me-stow some-package --remove
    
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
        print(f"Running: {params.op.value}")
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
        if entry.is_file():
            link_files.append(entry)

        elif entry.is_dir():
            new_dest = dest_dir / entry.name
            if new_dest.is_symlink():
                # This also make sure the new_dest exist and is a symlink
                match res_type:
                    case ResolveType.REPLACE:
                        new_dest.unlink()
                    case ResolveType.ADOPT:
                        try:
                            new_dest = new_dest.resolve(strict=True)
                        except FileNotFoundError:  # Broken link
                            new_dest.unlink()
                    case _:
                        raise ValueError("Unhandle type: this should not happend!")

            new_dest.mkdir(exist_ok=True)
            # recursive call
            process_stow_package(new_dest, entry, res_type)

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
        except su.SameFileError:
            # File already linked and good
            continue

        dest_file.symlink_to(file)


if __name__ == "__main__":
    main()
