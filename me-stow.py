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
  me-stow [operation] [options] <packages>
    <packages>      Can pass in one or multiple packages. If non specify
                    | this script will process all the packages that in source dir.
    
    [operation]     NOTE: only one op or omit
    --{i_}          Run this when you want to put stowed file to system.
                    | Make symlink to the file that in source folder.
                    | This is similar to `gnu stow`.
    --{st}          Run this when you want to stow file on system to source.
                    | Add new file that currently in system into package.
                    | Make new one if package is not currently exist.
    --{remo}        Use this when you want to remove link file in system.
                    | Delete the symlink file on the system (not on source).
                    | can be use with `--copy-back` flag to replace symlink
                    | file with actual file (copy file from source to the
                    | deleted file direction).
    
    -h | --{he}     Print this help message.

    [options]
    --{save_conf}   Save config to 'configs.json', this will automatic on
                    | when run script the first time or when configs.json not found.

    --{fo_}         Override current file on system if conflicts.

    --{resol}=      Use with `init` operation, strategy to resolve conflict files.
        {repla}     - Replace current file on system with a symlink,
                    | similar with the flag `--force`.
        {ado}       - (default) Copy current file on system to source folder and
                    | override file on source (this is like `--adopt` on stow),
                    | then user can use git to compare (or restore) them.

    --{copy_ba}     Use with `remove` operation, this will copy file on source to
                    | the link files on the system. Like replace symlink file
                    | with actual file.

    -v | --{verbo}  Vebose output
    
Examples:
    CMD = python3 `me-stow.py`
          or just `me-stow` if you put a link in PATH point to `me-stow.py`

    # init package
    CMD --init # this will init all the pakages in source dir
    CMD --init <or-you-can-put-pakages-name-here>
    # or you can omit the `--init` flag
    CMD <omit or you can put pakages name here>

    # this will replace current file on your system if confict happen
    CMD <pakages-name> --resolve=replace
    CMD <pakages-name> --force # equivalent to above

    # stow file to package
    # (can process multiple files but only 1 package at a time)
    CMD --stow <package-name> <path-to-files>
    # or you can omit the `--stow` flag
    CMD <package-name> <path-to-files>

    # remove package
    CMD --remove <packages-name>
    CMD --remove --copy-back <packages-name>
    CMD --remove # omit package will remove all the packages
    
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

    success = 0
    total = len(params.packages)
    match params.op:
        case Operation.HELP:
            print_help(exit=True)

        case Operation.INIT:
            for pkg_dir in params.packages:
                if not pkg_dir.exists():
                    print(f"[skipped] -- package not exist: '{pkg_dir.name}'")
                    continue
                process_init_package(params.root, pkg_dir, params.resolve)
                # TODO: check for failure
                success += 1

        case Operation.REMOVE:
            for pkg_dir in params.packages:
                remove_stow_package(params.root, pkg_dir, params.copy_back)
                # TODO: check for failure
                print(f"[ok] -- package '{pkg_dir.name}' removed")
                success += 1

        case Operation.STOW:
            total = len(params.stowers)
            success = process_stow_package(
                params.get_package_to_stow(), params.stowers, params.root
            )

    if params.save_config:
        params.save_configuration(CONFIG_FILE)

    print_result(params, total, success)
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


def print_result(param: Params, total: int, success: int) -> None:
    msg = f"-- [{success} / {total}] packages "
    match param.op:
        case Operation.INIT:
            msg += "init"
        case Operation.STOW:
            msg += f"stowed to package {param.get_package_to_stow().name}"
        case Operation.REMOVE:
            msg += "removed"

    print(msg)


if __name__ == "__main__":
    main()
