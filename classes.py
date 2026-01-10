from pathlib import Path
import sys
import json
from enum import Enum
from dataclasses import dataclass
from typing import Dict, List, Any


# ============================================================= #
# ============================================================= #


@dataclass
class Arguments:
    HELP = "help"
    VERBOSE = "verbose"
    FORCE = "force"
    RESOLVE = "resolve"
    INIT = "init"
    UPDATE = "update"
    REMOVE = "remove"
    SAVE_CONFIG = "save-config"
    COPY_BACK = "copy-back"
    PROCESS_ALL = "all"


class ResolveType(Enum):
    ADOPT = "adopt"
    REPLACE = "replace"


class Operation(Enum):
    NONE = "none"
    INIT = Arguments.INIT
    UPDATE = Arguments.UPDATE
    REMOVE = Arguments.REMOVE


@dataclass
class ConfigKey:
    SOURCE = "source_path"
    ROOT = "root_path"
    RESOLVE = "resolve"


class Params:
    def __init__(self, config_file: Path) -> None:
        # DEFAULT PARAMS
        self.op = Operation.NONE
        self.verbose = False
        self.save_config = False
        self.copy_back = False
        self.get_all = False
        self.packages: List[Path] = []

        _cfg = self.get_configurations(config_file)
        self.source_dir = Path(_cfg[ConfigKey.SOURCE])
        self.root = Path(_cfg[ConfigKey.ROOT])
        self.resolve = ResolveType(_cfg[ConfigKey.RESOLVE])

        self.assign_user_arguments()

        # CHECK POINT
        if not self.packages:
            if self.op == Operation.NONE:
                raise ValueError("Invalid arguments!!")
            if self.op == Operation.INIT or self.get_all:
                # First run, get all the packages in sources folder to stow
                self.find_all_packages()

        if self.verbose:
            pass

    def get_configurations(self, config_file: Path) -> Dict[str, Any]:
        try:
            with open(config_file, "r") as file:
                config = json.load(file)
        except FileNotFoundError:
            print("[warning] -- config file not found")
            config: Dict[str] = {}
            self.save_config = True

        # DEFAULT VALUE FOR CONFIG FILE
        if ConfigKey.SOURCE not in config:
            print(f"[warning] -- value '{ConfigKey.SOURCE}' not found")
            config[ConfigKey.SOURCE] = (
                input("Input source directory: ")
                if (
                    input("-- Use current directory as source [Y/n]: ")
                    .lower()
                    .startswith("n")
                )
                else str(Path.cwd())
            )

        if ConfigKey.ROOT not in config:
            print(f"[warning] -- value '{ConfigKey.ROOT}' not found")
            config[ConfigKey.ROOT] = (
                input("Input root directory: ")
                if input("-- Use HOME as root directory [Y/n]: ")
                .lower()
                .startswith("n")
                else str(Path.home())
            )

        if ConfigKey.RESOLVE not in config:
            print(f"[warning] -- value '{ConfigKey.RESOLVE}' not found")
            config[ConfigKey.RESOLVE] = ResolveType.ADOPT.value

        return config

    def assign_user_arguments(self) -> None:
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
                        raise Warning
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
                        raise Warning
                continue

            pkg = self.source_dir / arg
            if pkg.exists():
                self.packages.append(pkg)

    def find_all_packages(self) -> None:
        for entry in self.source_dir.iterdir():
            if entry.is_dir():
                self.packages.append(entry)

    def print_all_packages(self) -> None:
        print("Packages to stow:")
        for pkg in self.packages:
            print(f"-- {pkg.name}")

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
