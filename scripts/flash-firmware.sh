#!/usr/bin/env bash
# Writes the Ekips Prime Pro firmware to 0x08008000 via the LEGO DFU bootloader.
# The bootloader itself (0x08000000-0x08007FFF) is never written.
#
# UNTESTED ON HARDWARE. Read docs/en/flashing.md first and make a backup
# with scripts/backup-lego-firmware.sh.
#
# Usage: scripts/flash-firmware.sh [path/to/firmware.bin]

source "$(dirname "$0")/common.sh"

bin="${1:-target/firmware.bin}"
[[ -f "${bin}" ]] || die "${bin} not found. Build it with: cargo xtask bin"

size="$(file_size "${bin}")"
[[ "${size}" -gt 8 ]] || die "${bin} is too small to be a firmware image"
[[ "${size}" -le "${FIRMWARE_MAX_SIZE}" ]] || die "${bin} is larger than the firmware area (${FIRMWARE_MAX_SIZE} bytes)"

# The first word of the image is the initial stack pointer, which must be in RAM.
sp_hi="$(od -An -tx1 -j3 -N1 "${bin}" | tr -d ' ')"
[[ "${sp_hi}" == "20" ]] || die "${bin} does not start with a vector table (stack pointer not in RAM)"

if ! ls backup/lego-firmware-*.bin >/dev/null 2>&1; then
    echo "warning: no backup found in ./backup/. Run scripts/backup-lego-firmware.sh first." >&2
    confirm "Flash WITHOUT a backup?"
fi

require_dfu_util
require_hub_in_dfu_mode

echo "About to write ${bin} (${size} bytes) to ${FIRMWARE_START}."
confirm "This replaces the LEGO firmware (the bootloader stays)."

dfu-util -d "${DFU_DEVICE}" -a 0 -s "${FIRMWARE_START}:leave" -D "${bin}"
