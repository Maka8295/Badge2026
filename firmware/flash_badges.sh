#!/usr/bin/env bash
# Flash release/badge2026.bin onto badges, one after another. Nothing else from the repo is needed.
#
#   ./flash_badges.sh --full     new badges: whole image, wipes NVS (hunt progress, username, chat
#                                settings, transmit unlock)
#   ./flash_badges.sh --update   badges already in use: app only, keeps NVS
#
# Plug a badge in, wait for "done", unplug it, plug the next one in. Ctrl+C to stop.
# First run sets up esptool in ~/.cache/badge2026-flash (needs python3 and internet).
# Works on Linux and macOS. On Windows, follow "Flash the release build" in README.md.

set -euo pipefail

HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
IMAGE="${IMAGE:-$HERE/release/badge2026.bin}"
VENV="${BADGE_FLASH_VENV:-$HOME/.cache/badge2026-flash}"
ESPTOOL_VERSION="esptool>=5,<6"
CHIP=esp32s3
BAUD=921600
APP_OFFSET=0x10000
# The image starts with the bootloader at 0x0; the app partition is 64 KiB in.
APP_SKIP_BYTES=$((0x10000))
# A reset makes the badge drop off USB for a moment; only count it as unplugged after this long.
UNPLUG_SECONDS=3

red() { printf '\033[31m%s\033[0m\n' "$*"; }
green() { printf '\033[32m%s\033[0m\n' "$*"; }
bold() { printf '\033[1m%s\033[0m\n' "$*"; }

usage() {
    sed -n '2,10p' "$0" | sed 's/^# \{0,1\}//'
    exit "${1:-0}"
}

MODE=""
case "${1:-}" in
    --full) MODE=full ;;
    --update) MODE=update ;;
    -h | --help) usage 0 ;;
    "") ;;
    *) usage 1 ;;
esac

if [[ -z "$MODE" ]]; then
    bold "Which kind of flash?"
    echo "  1) full    new badges, wipes saved data (hunt progress, username, chat settings)"
    echo "  2) update  badges already in use, keeps saved data"
    read -rp "1 or 2: " choice
    case "$choice" in
        1) MODE=full ;;
        2) MODE=update ;;
        *) red "Pick 1 or 2." && exit 1 ;;
    esac
fi

# --- setup -------------------------------------------------------------------------------------

[[ -f "$IMAGE" ]] || { red "Image not found: $IMAGE"; exit 1; }

if ! command -v python3 >/dev/null; then
    red "python3 is missing. Install it first:"
    echo "  Arch/CachyOS:  sudo pacman -S python"
    echo "  Debian/Ubuntu: sudo apt install python3 python3-venv"
    echo "  Fedora:        sudo dnf install python3"
    echo "  macOS:         brew install python"
    exit 1
fi

if [[ ! -x "$VENV/bin/esptool" ]]; then
    bold "Setting up esptool in $VENV ..."
    if ! python3 -m venv "$VENV"; then
        red "Could not make a Python venv. On Debian/Ubuntu: sudo apt install python3-venv"
        exit 1
    fi
    "$VENV/bin/pip" install --quiet --upgrade pip
    "$VENV/bin/pip" install --quiet "$ESPTOOL_VERSION"
fi
ESPTOOL=("$VENV/bin/esptool" --chip "$CHIP" -b "$BAUD")

# Linux: the serial port belongs to the uucp (Arch) or dialout (Debian/Fedora) group.
if [[ "$(uname)" == Linux ]]; then
    group=""
    for g in uucp dialout; do getent group "$g" >/dev/null && group="$g" && break; done
    if [[ -n "$group" ]] && ! id -nG | tr ' ' '\n' | grep -qx "$group"; then
        red "You are not in the '$group' group, so you can't open the badge's serial port."
        echo "Run this, then log out and back in:"
        echo "  sudo usermod -aG $group $USER"
        exit 1
    fi
fi

if [[ "$MODE" == update ]]; then
    APP_IMAGE="$(mktemp)"
    trap 'rm -f "$APP_IMAGE"' EXIT
    tail -c +$((APP_SKIP_BYTES + 1)) "$IMAGE" >"$APP_IMAGE"
fi

# --- flashing loop -----------------------------------------------------------------------------

ports() {
    compgen -G '/dev/ttyACM*' || true
    compgen -G '/dev/cu.usbmodem*' || true
}

wait_for_badge() {
    local p
    while true; do
        p="$(ports | head -n1)"
        [[ -n "$p" ]] && { sleep 1; echo "$p"; return; }
        sleep 0.5
    done
}

wait_for_unplug() {
    local gone=0
    while ((gone < UNPLUG_SECONDS * 2)); do
        if [[ -z "$(ports)" ]]; then gone=$((gone + 1)); else gone=0; fi
        sleep 0.5
    done
}

flash() {
    local port="$1"
    if [[ "$MODE" == full ]]; then
        "${ESPTOOL[@]}" -p "$port" write-flash 0x0 "$IMAGE"
    else
        "${ESPTOOL[@]}" -p "$port" write-flash "$APP_OFFSET" "$APP_IMAGE"
    fi
}

bold "Mode: $MODE   Image: $IMAGE"
echo "sha256: $(sha256sum "$IMAGE" 2>/dev/null | cut -d' ' -f1 || shasum -a 256 "$IMAGE" | cut -d' ' -f1)"
count=0
while true; do
    echo
    bold "Plug in a badge (Ctrl+C to stop) ..."
    port="$(wait_for_badge)"
    echo "Found $port, flashing ..."
    log="$(mktemp)"
    if flash "$port" 2>&1 | tee "$log" | grep -E --line-buffered 'MAC:|Wrote|Hash of data verified|Hard resetting|rror'; then :; fi
    if grep -q 'Hash of data verified' "$log"; then
        count=$((count + 1))
        mac="$(grep -m1 -oE 'MAC: +[0-9a-f:]+' "$log" | awk '{print $2}')"
        green "done: badge #$count ($mac). Unplug it."
    else
        red "FAILED. Full output:"
        cat "$log"
        echo
        red "If it keeps failing: hold BOOT, tap RESET, release BOOT, and replug."
        [[ "$MODE" == update ]] && red "A badge that was never flashed needs --full."
    fi
    rm -f "$log"
    printf '\a'
    wait_for_unplug
done
