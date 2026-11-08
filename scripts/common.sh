#!/usr/bin/env bash
# Shared helpers for the DFU scripts. Sourced, not executed.
#
# UNTESTED ON HARDWARE. Read docs/en/flashing.md before using any script.

set -euo pipefail

# USB IDs of the LEGO SPIKE Prime DFU bootloader.
readonly DFU_DEVICE="0694:0011"
# Flash layout (see docs/en/memory-map.md).
readonly BOOTLOADER_START=0x08000000
readonly FIRMWARE_START=0x08008000
readonly FLASH_SIZE=$((0x100000))          # 1 MiB
readonly BOOTLOADER_SIZE=$((0x8000))       # 32 KiB, never written
readonly FIRMWARE_MAX_SIZE=$((FLASH_SIZE - BOOTLOADER_SIZE))

die() {
    echo "error: $*" >&2
    exit 1
}

require_dfu_util() {
    command -v dfu-util >/dev/null 2>&1 || die "dfu-util not found (install it first, see docs/en/flashing.md)"
}

require_hub_in_dfu_mode() {
    if ! dfu-util -l 2>/dev/null | grep -qi "${DFU_DEVICE}"; then
        die "no hub in DFU mode found (${DFU_DEVICE}). See docs/en/flashing.md, section 'Enter DFU mode'."
    fi
}

confirm() {
    local prompt="$1"
    local answer
    read -r -p "${prompt} Type 'yes' to continue: " answer
    [[ "${answer}" == "yes" ]] || die "aborted by user"
}

file_size() {
    wc -c <"$1" | tr -d ' '
}
