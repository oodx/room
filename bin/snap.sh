#!/usr/bin/env bash
set -euo pipefail

# snapshot target/criterion reports into meta/snaps
ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")"/.. && pwd)"
TARGET_DIR="$ROOT_DIR/target/criterion"
SNAP_DIR="$ROOT_DIR/meta/snaps"

if [ ! -d "$TARGET_DIR" ]; then
  echo "No criterion data found in $TARGET_DIR" >&2
  exit 0
fi

mkdir -p "$SNAP_DIR"

# copy each benchmark directory preserving baseline structure
find "$TARGET_DIR" -maxdepth 1 -mindepth 1 -type d | while read -r bench_dir; do
  bench_name="$(basename "$bench_dir")"
  rsync -a --delete "$bench_dir/" "$SNAP_DIR/$bench_name/"
  echo "Saved $bench_name to meta/snaps/$bench_name"
done
