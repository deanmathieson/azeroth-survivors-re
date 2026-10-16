#!/usr/bin/env bash
# Bash twin of scripts/package-friends.ps1: packs an already-built warcraft-survivors.exe into
# dist/AzerothSurvivors-<git short hash>.zip with the friends kit (scripts/friends-kit/) and an
# empty benilla-config. Builds nothing; never packs Blizzard data.
#   scripts/package-friends.sh [path/to/warcraft-survivors.exe] [out-dir]
# Requires: git, zip.
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
KIT_SRC="$ROOT/scripts/friends-kit"
EXE="${1:-$ROOT/target/play/warcraft-survivors.exe}"
OUT_DIR="${2:-$ROOT/dist}"

[ -f "$EXE" ] || { echo "No exe at '$EXE'. Build it first: cargo build --profile play -p warcraft-survivors" >&2; exit 1; }
command -v zip >/dev/null || { echo "needs 'zip' (or use scripts/package-friends.ps1)" >&2; exit 1; }

HASH="$(git -C "$ROOT" rev-parse --short HEAD)"
if [ -n "$(git -C "$ROOT" status --porcelain --untracked-files=no)" ]; then
    echo "warning: uncommitted changes; the zip is named after HEAD ($HASH)" >&2
fi

NAME="AzerothSurvivors-$HASH"
mkdir -p "$OUT_DIR"
OUT_DIR="$(cd "$OUT_DIR" && pwd)"
ZIP="$OUT_DIR/$NAME.zip"
STAGE_ROOT="$OUT_DIR/_stage"
STAGE="$STAGE_ROOT/$NAME"
rm -rf "$STAGE_ROOT"
mkdir -p "$STAGE/benilla-config"

# CRLF for everything cmd.exe or Notepad reads.
crlf() { sed -e 's/\r$//' -e 's/$/\r/' "$1" > "$2"; }

cp "$EXE" "$STAGE/warcraft-survivors.exe"
for f in Play.bat Host.bat Join.bat _launch.cmd README-FRIENDS.txt; do
    crlf "$KIT_SRC/$f" "$STAGE/$f"
done
crlf "$KIT_SRC/benilla-config-README.txt" "$STAGE/benilla-config/README.txt"

if find "$STAGE" -iname '*.mpq' -o -type d \( -name Data -o -name WoW \) | grep -q .; then
    echo "Refusing to pack Blizzard data" >&2; exit 1
fi

rm -f "$ZIP"
(cd "$STAGE_ROOT" && zip -r -q -9 "$ZIP" "$NAME")
rm -rf "$STAGE_ROOT"

if unzip -Z1 "$ZIP" | grep -qi '\.mpq$'; then
    rm -f "$ZIP"; echo "An .MPQ ended up in the zip; deleted it." >&2; exit 1
fi

echo "Wrote $ZIP:"
unzip -Z1 "$ZIP" | sed 's/^/  /'
echo "Everyone in the party needs this exact zip."
