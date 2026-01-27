# ME STOW

- [Description](#description)
- [Usage](#usage)
- [Examples](#examples)
- [Installation](#installation)
  - [Git clone](#git-clone)
  - [Manual installation](#manual-installation)
  - [Optional Enhancements](#optional-enhancements)

## Description

Configuration file manager (similar to [gnu stow](https://www.gnu.org/software/stow)).  
That stores dot files and replaces them with symlinks on your system.

## Usage

```text
me-stow [operation] [options] <packages>
    <packages>      Can pass in one or multiple packages. If non specify
                    | this script will process all the packages that in source dir.

    [operation]     NOTE: only one op or omit
    --init          Run this when you want to put stowed file to system.
                    | Make symlink to the file that in source folder.
                    | This is similar to `gnu stow`.
    --stow          Run this when you want to stow file on system to source.
                    | Add new file that currently in system into package.
                    | Make new one if package is not currently exist.
    --remove        Use this when you want to remove link file in system.
                    | Delete the symlink file on the system (not on source).
                    | can be used with `--copy-back` flag to replace symlink
                    | file with actual file (copy file from source to the
                    | deleted file direction).
    --list          List all the package currently on source directions.

    -h | --help     Print this help message.

    [options]
    --saveconfig    Save config to 'configs.json', this will automatic on
                    | when run script the first time or when configs.json not found.

    --force         Override current file on system if conflicts.

    --resolve=      Use with `init` operation, strategy to resolve conflict files.
        replace     - Replace current file on system with a symlink,
                    | similar with the flag `--force`.
        adopt       - (default) Copy current file on system to source folder and
                    | override file on source (this is like `--adopt` on stow),
                    | then user can use git to compare (or restore) them.

    --copyback      Use with `remove` operation, this will copy file on source to
                    | the link files on the system. Like replace symlink file
                    | with actual file.

    -v | --verbose  Vebose output
```

## Examples

```bash
CMD = python3 `me-stow.py`
      # or just `me-stow` if you put a link in PATH point to `me-stow.py`

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
```

## Installation

### Git clone

```bash
git clone https://github.com/dddGR/me-stow.git

cd me-stow
chmod u+x install.sh && ./install.sh
```

### Manual installation

Download or copy the following files to your local machine:  
No additional dependencies are needed beyond those included with Python.

- `me-stow.py`
- `classes.py`

Create a `config.json` file in the same directory. This file holds configurations unique to your system.

> NOTE: creating a `config.json` is optional, as it will be generated automatically when you run the script.

```json
{
    "source_path": "path-to-store-your-config",
    "root_path": "your-home-path",
    "resolve": "adopt"
}
```

After that, execute `me-stow.py` to view all available commands. Or see [Usage](#usage) or [Example](#examples).

```bash
# Display help when no arguments are provided
python3 me-stow.py

# Equivalent to the command below
python3 me-stow.py -h
```

### Optional Enhancements

While not necessary, the following steps can improve usability:

- Utilize [`uv`](https://github.com/astral-sh/uv) to execute the script and manage dependencies effortlessly.
- Create a symlink to your executable location for direct access.

```bash
ln -s "full-path-to-me-stow.py" "path-to-your-PATH-folder/me-stow"
```
