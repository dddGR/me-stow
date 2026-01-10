from pathlib import Path
import sys
import json
from enum import Enum
from dataclasses import dataclass
from typing import Dict, List, Any


# ============================================================= #
# ============================================================= #


class ResolveType(Enum):
    ADOPT = "adopt"
    REPLACE = "replace"


class Operation(Enum):
    NONE = "none"
    INIT = "init"
    UPDATE = "update"
    REMOVE = "remove"


@dataclass
class Key:
    SOURCE = "source_path"
    ROOT = "root_path"
    RESOLVE = "resolve"


class Params:
    def __init__(self, config_file: Path) -> None:
        # DEFAULT PARAMS
        self.op = Operation.NONE
        self.verbose = False
        self.save_config = False
        self.packages: List[Path] = []

        config = self.get_configurations(config_file)
        self.source_dir = (
            Path(config[Key.SOURCE])
            if Key.SOURCE in config and config[Key.SOURCE]
            else Path.cwd()
        )
        self.root = (
            Path(config[Key.ROOT])
            if Key.ROOT in config and config[Key.ROOT]
            else Path.home()
        )
        self.resolve = ResolveType(config[Key.RESOLVE])
        self.assign_user_arguments()

        # CHECK POINT
        if not self.packages:
            if self.op == Operation.NONE:
                raise ValueError("Invalid arguments!!")
            if self.op == Operation.INIT:
                # First run, get all the packages in sources folder to stow
                self.find_all_packages()

        if self.verbose:
            pass

    def get_configurations(self, config_file: Path) -> Dict[str, Any]:
        config: Dict[str] = {}
        try:
            with open(config_file, "r") as file:
                config = json.load(file)
        except FileNotFoundError:
            print("[warning] -- config file not found")
            self.save_config = True
            config[Key.SOURCE] = (
                input("Input source directory: ")
                if input("Use current directory as source [Y/n]: ")
                .lower()
                .startswith("n")
                else ""
            )
            config[Key.ROOT] = (
                input("Input root directory: ")
                if input("Use HOME as root directory [Y/n]: ").lower().startswith("n")
                else ""
            )
            config[Key.RESOLVE] = ResolveType.ADOPT.value

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
                    case "help":
                        raise Warning
                    case "verbose":
                        self.verbose = True
                    case "force":
                        self.resolve = ResolveType.REPLACE
                    case Key.RESOLVE:
                        self.resolve = ResolveType(val)
                    case "save-config":
                        self.save_config = True
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
            Key.SOURCE: str(self.source_dir.resolve()),
            Key.ROOT: str(self.root.resolve()),
            Key.RESOLVE: self.resolve,
        }

        with open(file_dir, "w") as file:
            json.dump(config, file, indent=4)

        if self.verbose:
            print(f"Configuration saved to: '{file_dir}'")
            print("Config output:\n" + json.dumps(config, indent=4))

    @property
    def resolve(self):
        return self._resolve

    @resolve.setter
    def resolve(self, value: ResolveType):
        if value not in ResolveType:
            raise ValueError("wrong value for 'resolve'")
        self._resolve: ResolveType = value
