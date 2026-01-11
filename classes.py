from pathlib import Path
import sys
import json
from enum import Enum
from dataclasses import dataclass
from typing import Dict, List, Generator


# ============================================================= #
# ============================================================= #


@dataclass
class Arguments:
    INIT = "init"
    REMOVE = "remove"
    STOW = "stow"
    HELP = "help"
    LIST = "list"
    VERBOSE = "verbose"
    FORCE = "force"
    RESOLVE = "resolve"
    SAVE_CONFIG = "save-config"
    COPY_BACK = "copy-back"
    PROCESS_ALL = "all"


class ResolveType(Enum):
    ADOPT = "adopt"
    REPLACE = "replace"


class Operation(Enum):
    NONE = "none"
    HELP = Arguments.HELP
    INIT = Arguments.INIT
    REMOVE = Arguments.REMOVE
    STOW = Arguments.STOW
    LIST = Arguments.LIST


@dataclass
class ConfigKey:
    SOURCE = "source_path"
    ROOT = "root_path"
    RESOLVE = "resolve"


class Params:
    def __init__(self, config_file: Path) -> None:
        # DEFAULT PARAMS
        self._op = Operation.NONE
        self.verbose = False
        self.save_config = False
        self.copy_back = False
        self.get_all = False
        self.packages: List[Path] = []
        self.stowers: List[Path] = []

        self.assign_configurations(config_file)
        self.assign_user_arguments()

        # CHECK POINT
        self.eval_operation()

        if self.verbose:
            print(f"-- Current source: '{self.source_dir}'")
            print(f"-- Current root: '{self.root}'")
            print(f"-- Running op: '{self.op.value}'")
            self.print_all_packages()
            if self.op == Operation.LIST:
                sys.exit(0)

    def assign_configurations(self, config_file: Path):
        """
        Assign configurations from config file (if exist) or assign default value.
        """
        config: Dict[str] = {}
        try:
            with open(config_file, "r") as file:
                config = json.load(file)
        except FileNotFoundError:
            print("[warning] -- config file not found")
            self.save_config = True

        self.source_dir = (
            Path(config[ConfigKey.SOURCE])
            if ConfigKey.SOURCE in config
            else (
                Path(input("\nInput source directory: "))
                if (
                    input("\n-- Use current directory as source [Y/n]: ")
                    .lower()
                    .startswith("n")
                )
                else Path.cwd()
            )
        )
        self.root = (
            Path(config[ConfigKey.ROOT])
            if ConfigKey.ROOT in config
            else (
                Path(input("\nInput root directory: "))
                if input("\n-- Use HOME as root directory [Y/n]: ")
                .lower()
                .startswith("n")
                else Path.home()
            )
        )

        self.resolve = (
            ResolveType(config[ConfigKey.RESOLVE])
            if ConfigKey.RESOLVE in config
            else ResolveType.ADOPT
        )

        return config

    def assign_user_arguments(self) -> None:
        if len(sys.argv) == 1:
            self.op = Operation.HELP
            return

        for arg in sys.argv[1:]:
            if arg.startswith("--"):
                key = arg[2:].lower()
                try:
                    key, val = key.split("=")
                except ValueError:
                    val = ""

                match key:
                    case _ if key in [op.value for op in Operation]:
                        self.op = Operation(key)
                    case Arguments.HELP:
                        self.op = Operation.HELP
                    case Arguments.LIST:
                        self.op = Operation.LIST
                    case Arguments.VERBOSE:
                        self.verbose = True
                    case Arguments.FORCE:
                        self.resolve = ResolveType.REPLACE
                    case Arguments.RESOLVE:
                        self.resolve = ResolveType(val)
                    case Arguments.SAVE_CONFIG:
                        self.save_config = True
                    case Arguments.COPY_BACK:
                        self.copy_back = True
                    case Arguments.PROCESS_ALL:
                        self.get_all = True
                continue

            if arg.startswith("-"):
                match arg[1:].lower():
                    case "v":
                        self.verbose = True
                    case "h":
                        self.op = Operation.HELP
                continue

            if (file := Path(arg).absolute()).is_file():
                # Assume that the pass in argument is a dir path for
                # the file that need to be stow
                self.stowers.append(file)
                continue

            if is_folder_name(arg):
                try:
                    # If the name of package pass in and it package already
                    # inside source direction.
                    pkg = self.source_dir / arg
                    self.packages.append(pkg.resolve(strict=True))
                except FileNotFoundError:
                    # Maybe this is new package that user want to add to source
                    self.packages.append(pkg)
                continue

            if self.op == Operation.STOW and (dir := Path(arg).absolute()).is_dir():
                # stow all the files in this directory
                self.stowers.extend([f for f in dir.rglob("*") if f.is_file()])

            else:
                raise ValueError(f"[warning] -- path not exist: '{arg}'")

    def eval_operation(self) -> None:
        match self.op:
            case Operation.HELP:
                return

            case Operation.LIST:
                self.get_all = True
                self.verbose = True

            case Operation.STOW:
                if (leng := len(self.packages)) != 1:
                    raise ValueError(
                        f"stow op accept only 1 package, current: [{leng}]"
                    )

            case Operation.INIT | Operation.REMOVE if self.stowers:
                raise ValueError(f"don't pass in file when running: '{self.op.name}'")

            case Operation.INIT:
                if not self.packages:
                    self.get_all = True

            case Operation.REMOVE:
                if not self.packages:
                    if input("Remove all packages [y/N]: ").lower().startswith("y"):
                        self.get_all = True

            case Operation.NONE:
                if not self.packages:
                    raise ValueError("Invalid arguments!!")
                elif self.stowers:
                    self.op = Operation.STOW
                else:
                    self.op = Operation.INIT

            case _:
                raise ValueError(f"wrong op: '{self.op.name}'")

        if self.get_all:
            if self.packages:
                raise ValueError("don't use `--all` with other package")
            if self.op == Operation.STOW:
                raise ValueError(f"don't use `--all` when running '{self.op.name}'")
            self.get_all_packages()

    def get_package_to_stow(self) -> Path:
        return self.packages[0]

    def get_all_packages(self) -> None:
        self.packages = [d for d in self.source_dir.iterdir() if d.is_dir()]

    def print_all_packages(self) -> None:
        print(f"\nPackages to stow : [{len(self.packages)}]")
        for pkg in self.packages:
            name = f"'{pkg.name}'"
            print(name, "-" * (40 - (len(name))))
            for line in tree(pkg):
                print(line)

    def save_configuration(self, file_dir: Path) -> None:
        config = {
            ConfigKey.SOURCE: str(self.source_dir),
            ConfigKey.ROOT: str(self.root),
            ConfigKey.RESOLVE: self.resolve,
        }

        with open(file_dir, "w") as file:
            json.dump(config, file, indent=4)

        if self.verbose:
            print(f"Configuration saved to: '{file_dir}'")
            print("Config output:\n" + json.dumps(config, indent=4))

    # ============================================================= #
    # MEM VARIABLES THAT NEED TO CHECK ============================ #
    @property
    def op(self):
        return self._op

    @op.setter
    def op(self, value: Operation):
        if value not in Operation:
            raise ValueError("wrong value for 'operation'")
        if self.op != Operation.NONE:
            raise ValueError("cannot process more than 1 operation")
        self._op = value

    @property
    def resolve(self):
        return self._resolve

    @resolve.setter
    def resolve(self, value: ResolveType):
        if value not in ResolveType:
            raise ValueError("wrong value for 'resolve'")
        self._resolve = value

    @property
    def source_dir(self):
        return self._source_dir

    @source_dir.setter
    def source_dir(self, path: Path):
        if not path.exists():
            raise ValueError(f"path not exist: '{path}'")
        self._source_dir = path

    @property
    def root(self):
        return self._root

    @root.setter
    def root(self, path: Path):
        if not path.exists():
            raise ValueError(f"path not exist: '{path}'")
        self._root = path


def is_folder_name(name: str) -> bool:
    """
    Simple check for invalid character in the name.
    """
    if sys.platform.lower().startswith("win"):
        invalid_chars = r"<>:\"/\\|?*" + "".join(chr(i) for i in range(0, 32))
    else:
        invalid_chars = r"/"

    return all(char not in invalid_chars for char in name)


# prefix components:
TREE_SPACE = "    "
TREE_BRANCH = "│   "
# pointers:
TREE_TEE = "├── "
TREE_LAST = "└── "


def tree(dir_path: Path, prefix: str = "") -> Generator[str]:
    """
    A recursive generator, given a directory Path object will yield
    a visual tree structure line by line with each line prefixed by
    the same characters

    Credit to: https://stackoverflow.com/a/59109706 with some modification
    """
    contents = list(dir_path.iterdir())
    files = []
    # contents each get pointers that are ├── with a final └── :
    pointers = [TREE_TEE] * (len(contents) - 1) + [TREE_LAST]
    for pointer, path in zip(pointers, contents):
        out = prefix + pointer + path.name
        if path.is_file():
            files.append(out)
            continue

        yield out
        extension = TREE_BRANCH if pointer == TREE_TEE else TREE_SPACE
        # i.e. space because last, └── , above so no more |
        yield from tree(path, prefix=prefix + extension)

    for file in files:
        yield file
