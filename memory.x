/* Linker memory layout for the LEGO SPIKE Prime hub (STM32F413VGT6).
 *
 * Flash (1 MiB, 0x08000000 - 0x080FFFFF):
 *   0x08000000 - 0x08007FFF   LEGO DFU bootloader, sectors 0-1 (32 KiB).
 *                             NEVER write or erase this range.
 *   0x08008000 - 0x080FFFFF   This firmware, sectors 2-11 (992 KiB).
 *
 * RAM (320 KiB): SRAM1 (256 KiB) and SRAM2 (64 KiB) are contiguous and are
 * used as one block.
 *
 * See docs/en/memory-map.md for details.
 */
MEMORY
{
  FLASH : ORIGIN = 0x08008000, LENGTH = 992K
  RAM   : ORIGIN = 0x20000000, LENGTH = 320K
}

/* Fail the link if the firmware ever moves into the bootloader range. */
ASSERT(ORIGIN(FLASH) >= 0x08008000, "FLASH must not start inside the LEGO bootloader");
