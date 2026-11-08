#!/usr/bin/env bash
# Reads (uploads) the complete flash of the hub into backup files.
# Reading never modifies the hub.
#
# UNTESTED ON HARDWARE. Read docs/en/flashing.md first.
#
# Usage: scripts/backup-lego-firmware.sh [output-directory]
#
# Produces:
#   lego-full-<date>.bin      0x08000000, 1 MiB (bootloader + firmware), for reference only
#   lego-firmware-<date>.bin  0x08008000, 992 KiB (firmware only), used by restore-lego-firmware.sh

source "$(dirname "$0")/common.sh"

out_dir="${1:-backup}"
stamp="$(date +%Y%m%d-%H%M%S)"
full="${out_dir}/lego-full-${stamp}.bin"
fw="${out_dir}/lego-firmware-${stamp}.bin"

require_dfu_util
require_hub_in_dfu_mode
mkdir -p "${out_dir}"

echo "Reading firmware area (${FIRMWARE_START}, ${FIRMWARE_MAX_SIZE} bytes) ..."
dfu-util -d "${DFU_DEVICE}" -a 0 -s "${FIRMWARE_START}:${FIRMWARE_MAX_SIZE}" -U "${fw}"

echo "Reading complete flash (${BOOTLOADER_START}, ${FLASH_SIZE} bytes) ..."
if ! dfu-util -d "${DFU_DEVICE}" -a 0 -s "${BOOTLOADER_START}:${FLASH_SIZE}" -U "${full}"; then
    echo "warning: full read failed (the bootloader may refuse to read itself)." >&2
    echo "         The firmware-only backup ${fw} is what matters for a restore." >&2
fi

[[ "$(file_size "${fw}")" -eq "${FIRMWARE_MAX_SIZE}" ]] || die "firmware backup has an unexpected size"

if command -v sha256sum >/dev/null 2>&1; then
    sha256sum "${out_dir}"/lego-*-"${stamp}".bin >"${out_dir}/SHA256SUMS-${stamp}"
fi

echo "Backup written to ${out_dir}/. Keep it somewhere safe (not only on this computer)."
