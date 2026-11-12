#!/usr/bin/env bash
# Install Lumen for the current user: binary, hicolor icons (PNG + scalable SVG)
# and the .desktop launcher, then refresh icon/desktop caches when the tools exist.
#
# Usage: scripts/install.sh [--binary PATH] [--prefix DIR] [--uninstall] [--dry-run] [-h|--help]
#   --binary PATH  built binary (default: target/release/lumen-rs under $CARGO_TARGET_DIR or the repo)
#   --prefix DIR   binary goes to DIR/bin (default: ~/.local)
#   --uninstall    remove everything this script installs
#   --dry-run      print actions, change nothing
#
# Exit codes: 0 ok, 1 install failure, 2 bad usage, 3 missing input file.
set -euo pipefail

APP=lumen-rs
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
PREFIX="${HOME}/.local"
BIN=""
UNINSTALL=0
DRY=0

usage() { sed -n '2,12p' "${BASH_SOURCE[0]}" | sed 's/^# \{0,1\}//'; }
die() { echo "install.sh: $*" >&2; exit "${2:-1}"; }

while [ $# -gt 0 ]; do
  case "$1" in
    --binary) [ $# -ge 2 ] || die "--binary needs a path" 2; BIN="$2"; shift 2 ;;
    --prefix) [ $# -ge 2 ] || die "--prefix needs a directory" 2; PREFIX="$2"; shift 2 ;;
    --uninstall) UNINSTALL=1; shift ;;
    --dry-run) DRY=1; shift ;;
    -h|--help) usage; exit 0 ;;
    *) echo "install.sh: unknown option: $1" >&2; usage >&2; exit 2 ;;
  esac
done

DATA="${XDG_DATA_HOME:-$HOME/.local/share}"
ICONS="$DATA/icons/hicolor"
APPS="$DATA/applications"
SRC_ICONS="$ROOT/packaging/icons/linux/hicolor"
DESKTOP_SRC="$ROOT/packaging/linux/$APP.desktop"
[ -n "$BIN" ] || BIN="${CARGO_TARGET_DIR:-$ROOT/target}/release/$APP"

run() { if [ "$DRY" = 1 ]; then echo "+ $*"; else "$@"; fi; }

refresh_caches() {
  if command -v gtk-update-icon-cache >/dev/null 2>&1 && [ -f "$ICONS/index.theme" ]; then
    run gtk-update-icon-cache -f -t "$ICONS" || echo "install.sh: gtk-update-icon-cache failed (ignored)" >&2
  fi
  if command -v update-desktop-database >/dev/null 2>&1; then
    run update-desktop-database "$APPS" || echo "install.sh: update-desktop-database failed (ignored)" >&2
  fi
  for k in kbuildsycoca6 kbuildsycoca5; do
    if command -v "$k" >/dev/null 2>&1; then
      run "$k" >/dev/null 2>&1 || true
      break
    fi
  done
}

if [ "$UNINSTALL" = 1 ]; then
  run rm -f "$PREFIX/bin/$APP" "$APPS/$APP.desktop"
  for f in "$ICONS"/*/apps/$APP.png "$ICONS"/scalable/apps/$APP.svg; do
    [ -e "$f" ] && run rm -f "$f"
  done
  refresh_caches
  echo "Removed Lumen from $PREFIX and $DATA"
  exit 0
fi

[ -f "$BIN" ] || die "binary not found: $BIN (run cargo build --release, or pass --binary)" 3
[ -f "$DESKTOP_SRC" ] || die "missing $DESKTOP_SRC" 3
[ -d "$SRC_ICONS" ] || die "missing $SRC_ICONS" 3

run install -Dm755 "$BIN" "$PREFIX/bin/$APP"
n=0
for f in "$SRC_ICONS"/*/apps/$APP.png "$SRC_ICONS"/scalable/apps/$APP.svg; do
  [ -f "$f" ] || die "missing icon $f" 3
  rel="${f#"$SRC_ICONS"/}"
  run install -Dm644 "$f" "$ICONS/$rel"
  n=$((n + 1))
done
# Icon= and StartupWMClass= already match the app id the binary reports ($APP).
run install -Dm644 "$DESKTOP_SRC" "$APPS/$APP.desktop"
# Make Exec= absolute when the prefix is not on PATH so launchers always find the binary.
if [ "$DRY" = 0 ]; then
  sed -i "s#^Exec=$APP #Exec=$PREFIX/bin/$APP #" "$APPS/$APP.desktop"
fi
refresh_caches
echo "Installed Lumen: binary, $n icons, desktop entry (data dir $DATA)"
case ":$PATH:" in *":$PREFIX/bin:"*) ;; *) echo "note: $PREFIX/bin is not on PATH" >&2 ;; esac
