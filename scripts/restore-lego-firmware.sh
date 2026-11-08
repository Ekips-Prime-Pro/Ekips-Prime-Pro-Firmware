#!/usr/bin/env bash
# Restores a firmware backup made with backup-lego-firmware.sh.
# Only the firmware area (0x08008000 and up) is written, never the bootloader.
#
# UNTESTED ON HARDWARE. Read docs/en/flashing.md first. The easiest way to
# get the official firmware back is the LEGO SPIKE app's firmware update.
#
# Usage: scripts/restore-lego-firmware.sh backup/lego-firmware-<date>.bin

source "$(dirname "$0")/common.sh"

bin="${1:-}"
[[ -n "${bin}" ]] || die "usage: $0 backup/lego-firmware-<date>.bin"
[[ -f "${bin}" ]] || die "${bin} not found"
[[ "${bin}" != *lego-full-* ]] || die "use the lego-firmware-*.bin file, not the full flash dump (it contains the bootloader)"
[[ "$(file_size "${bin}")" -le "${FIRMWARE_MAX_SIZE}" ]] || die "${bin} is larger than the firmware area"

require_dfu_util
require_hub_in_dfu_mode

echo "About to restore ${bin} to ${FIRMWARE_START}."
confirm "This replaces the current firmware (the bootloader stays)."

dfu-util -d "${DFU_DEVICE}" -a 0 -s "${FIRMWARE_START}:leave" -D "${bin}"
