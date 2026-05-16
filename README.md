# ME STOW

- [Description](#description)
- [Usage](#usage)
  - [Examples](#examples)
- [Installation](#installation)
  - [Compile Yourself](#compile-yourself)
  - [Download Executable Binary](#download-executable-binary)
  - [Note](#note)

## Description

Configuration file manager (similar to [gnu stow](https://www.gnu.org/software/stow)).  
That stores dot files and replaces them with symlinks on your system.

## Usage

```bash
$ ❯ me-stow help
# Simple configuration files management that use symlink (like gnu stow) but with some changes

me-stow <COMMAND>

Commands:
  sync    # Sync packages in source with system
  stow    # Stow files into package
  remove  # Remove selected packages
  list    # List all current packages
  help    # Print this message or the help of the given subcommand(s)

Options:
  -h, --help     Print help
  -V, --version  Print version

# Run --help with each command to see more options.
$ ❯ me-stow sync --help
```

### Examples

```bash
# sync everything (like when install a fresh system)
me-stow sync
# this will sync `pakage-name` if that currently in your src dir.
me-stow sync <package-name>
# display status of the current src and your system.
me-stow sync (-d | --diff)

# stow files to package
me-stow stow <pakages-name> [files-need-to-stow]
# or you can put directory that need to stow
me-stow stow <pakages-name> [files, dirs,...]
# NOTE: this will stow every files in that directory (not whole dir like GNU Stow)

# remove package
me-stow remove [packages-need-to-remove]
# to remove everything, include file on the system.
me-stow remove [packages-need-to-remove] (-p | --purge)

# to see what packages that currently in your src dir.
me-stow list # this will list all packages (name only)
me-stow list (-f | --full) # this will list every files (in tree format)
me-stow list <pakages-name> # only one package
```

## Installation

### Compile Yourself

```bash
git clone https://github.com/dddGR/me-stow.git
cd me-stow && cargo build --release
```

> Note: you will need `rustc` and `cargo` to build.

The final bin will be in `./target/release`.  
Copy/Link `me-stow` executable to your `$PATH` to use.  
After that you can `me-stow` directory to save some space.

### Download Executable Binary

The pre-build binary is provided in [release](https://github.com/dddGR/me-stow/releases) section.

### Note

When first run program (or when cannot find config file). It will ask for:

- `src_dir`: the directory that your stow files will go.
- `root_dir`: the base directory that will use to locate the config file on your system.  
  (usually your $HOME)

After that, the configuration file will be saved at `$HOME/.config/me-stow.toml`.
