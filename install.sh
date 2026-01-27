#!/bin/bash
set -e
available() { command -v "${1:?}" >/dev/null; }

# ============================================================= #
# PRE FLIGHT ================================================== #
P_BASE=$(dirname "$(readlink -f "$0")")
F_EXEC="$P_BASE/me-stow.py"
# CHECK ARGUMENTS
for arg in "$@"; do
    case $arg in
    -s* | --src*)
        P_SRC=${arg##*=}
        ;;
    -r* | --root*)
        P_ROOT=${arg##*=}
        ;;
    -o* | --output*)
        P_OUTPUT=${arg##*=}
        ;;
    *)
        echo "Unknown option: $arg"
        exit 1
        ;;
    esac
done

# CHECK SOURCE
if [[ -z "$P_SRC" ]]; then
    read -p "Input source directory: " -r P_SRC
fi

if [[ ! -d "$P_SRC" ]]; then
    echo "'$P_SRC' is not valid directory"
    exit 1
fi

# CHECK ROOT
if [[ -z "$STRING" ]]; then
    read -p "Use HOME directory as root? [Y/n]: " -n 1 -r
    if [[ $REPLY =~ ^[Nn]$ ]]; then
        echo
        read -p "Input your desire root: " -r P_ROOT
    else
        P_ROOT="$HOME"
    fi
fi

if [[ ! -d "$P_ROOT" ]]; then
    echo "'$P_ROOT' is not valid directory"
    exit 1
fi

# ============================================================= #
# MAIN SCRIPT ================================================= #

cat <<EOF >"$P_BASE/config.json"
{
    "source_path": "$P_SRC",
    "root_path": "$P_ROOT",
    "resolve": "adopt"
}
EOF
chmod u+x "$F_EXEC"

if [[ -n "$P_OUTPUT" ]]; then
    # make symlink to me-stow.py
    mkdir -p "$P_OUTPUT"
    ln -sf "$F_EXEC" "$P_OUTPUT/me-stow"
fi
