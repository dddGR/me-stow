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
    # Just to look a little bit nicer
    i_ = Arguments.INIT
    upda = Arguments.UPDATE
    remo = Arguments.REMOVE
    st = Arguments.STOW
    save_conf = Arguments.SAVE_CONFIG
    fo_ = Arguments.FORCE
    resol = Arguments.RESOLVE
    repla = ResolveType.REPLACE.value
    ado = ResolveType.ADOPT.value
    copy_ba = Arguments.COPY_BACK
    verbo = Arguments.VERBOSE
    he = Arguments.HELP
    print(f"""
------------------------------------------------------------
Usage:
  me-stow [options] <packages>
    <packages>      Can pass in one or multiple packages. If non specify
                    this script will process all the packages that in source dir.
    [options]
    --{i_}          Make symlink to the file that in source folder. This is
                    similar to `gnu stow`.
      {upda}        TODO
      {remo}        Delete the symlink file on the system, can be use with
                    `--copy-back` flag to replace symlink file with actual file.
      {st}          Add new file into package.

    --{save_conf}   Save config to 'configs.json', this will automatic on
                    when run script the first time or when configs.json not found.

    --{fo_}         Override current file on system if conflicts.

    --{resol}=      Use with `init` operation, strategy to resolve conflict files.
        {repla}     - Replace current file on system with a symlink,
                      similar with the flag `--force`.
        {ado}       - (default) Copy current file on system to source folder and
                      override file on source (this is like `--adopt` on stow),
                      then user can use git to compare (or restore) them.

    --{copy_ba}     Use with `remove` operation, this will copy file on source to
                    the link files on the system. Like replace symlink file with
                    actual file.

    -v | --{verbo}  Vebose output
    -h | --{he}     Print help usage
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
    except ValueError as e:
        err_print_help_exit(e)

    match params.op:
        case Operation.HELP:
            print_help(exit=True)

        case Operation.INIT:
            for pkg_dir in params.packages:
                process_init_package(params.root, pkg_dir, params.resolve)

            # TODO: check for failure
            num = len(params.packages)
            print(f" -- [{num}] packages stowed" if num > 1 else "")

        case Operation.REMOVE:
            for pkg_dir in params.packages:
                remove_stow_package(params.root, pkg_dir, params.copy_back)
                # TODO: check for failure
                print(f"[ok] -- package '{pkg_dir.name}' removed")

        case Operation.STOW:
            pkg = params.get_package_to_stow()
            success = process_stow_package(pkg, params.stowers, params.root)
            print(f"[ok] -- [{success}] files stow to package '{pkg.name}'")

    if params.save_config:
        params.save_configuration(CONFIG_FILE)

    print("...DONE")


# ============================================================ #
# ============================================================ #


def process_init_package(dest_dir: Path, package: Path, res_type: ResolveType) -> None:
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
            process_init_package(new_dest, entry, res_type)

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


def remove_stow_package(dest_dir: Path, package: Path, restore: bool) -> None:
    """
    Remove a stowed package.

    NOTE: can run recursively

    :param restore: `True` will copy file in source into system,
                    this is like replace linked file with actual file
    :type restore: bool
    """
    for entry in package.iterdir():
        file_on_sys = dest_dir / entry.name

        if entry.is_dir():
            # recursive call
            remove_stow_package(file_on_sys, entry, restore)

        elif entry.is_file():
            if file_on_sys.samefile(entry):
                file_on_sys.unlink()

                if restore:
                    su.copy(entry, file_on_sys)

    try:
        dest_dir.rmdir()
    except OSError:
        # well, don't remove non empty folder
        pass


def process_stow_package(
    pkg_dir: Path, file_to_stows: List[Path], root_dir: Path
) -> int:
    """
    Stow all the file to the pakage direction.
    """
    success = 0

    for file in file_to_stows:
        try:
            relative = file.relative_to(root_dir)
        except ValueError as e:
            err_print_help_exit(e)

        try:
            stowed_dir = pkg_dir / relative
            stowed_dir.parent.mkdir(parents=True, exist_ok=True)
            su.copyfile(file, stowed_dir)
            file.unlink()
            file.symlink_to(stowed_dir)
        except Exception as e:
            print(f"-- [failed] -- '{file}' with error: {e}")
        else:
            success += 1

    return success


if __name__ == "__main__":
    main()
