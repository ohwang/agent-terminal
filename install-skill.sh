#!/usr/bin/env bash
set -euo pipefail

SKILL_NAME="agent-terminal"
SOURCE_DIR="$(cd "$(dirname "$0")" && pwd)/skills/$SKILL_NAME"
TARGET_DIR="${HOME}/.claude/skills/$SKILL_NAME"

if [ ! -d "$SOURCE_DIR" ]; then
  echo "Error: skill source not found at $SOURCE_DIR" >&2
  exit 1
fi

mkdir -p "$(dirname "$TARGET_DIR")"

if [ -L "$TARGET_DIR" ]; then
  existing="$(readlink "$TARGET_DIR")"
  if [ "$existing" = "$SOURCE_DIR" ]; then
    echo "Skill already installed (symlink exists)."
    exit 0
  fi
  echo "Updating existing symlink: $existing -> $SOURCE_DIR"
  rm "$TARGET_DIR"
elif [ -e "$TARGET_DIR" ]; then
  echo "Error: $TARGET_DIR already exists and is not a symlink. Remove it first." >&2
  exit 1
fi

ln -s "$SOURCE_DIR" "$TARGET_DIR"
echo "Installed: $TARGET_DIR -> $SOURCE_DIR"
