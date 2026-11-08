# Memory map

Source of truth: [`memory.x`](../../memory.x). Device: STM32F413VGT6
(1 MiB flash, 320 KiB SRAM).

## Flash

| Sector | Address range | Size | Content |
|-------:|---------------|-----:|---------|
| 0 | `0x08000000–0x08003FFF` | 16 KiB | **LEGO DFU bootloader – never write or erase** |
| 1 | `0x08004000–0x08007FFF` | 16 KiB | **LEGO DFU bootloader – never write or erase** |
| 2 | `0x08008000–0x0800BFFF` | 16 KiB | Firmware: vector table, code |
| 3 | `0x0800C000–0x0800FFFF` | 16 KiB | Firmware |
| 4 | `0x08010000–0x0801FFFF` | 64 KiB | Firmware |
| 5–11 | `0x08020000–0x080FFFFF` | 7 × 128 KiB | Firmware |

- The firmware may use up to **992 KiB** (`0x08008000–0x080FFFFF`).
- Because the bootloader ends exactly at a sector boundary, writing the
  firmware with DFU or SWD only erases sectors 2 and up. Neither
  `dfu-util -s 0x08008000` nor `probe-rs` (which only erases sectors the
  ELF occupies) touches sectors 0 and 1.
- Pybricks reads the hub variant (SPIKE Prime vs. Robot Inventor) from
  `0x08007D80`, i.e. **inside** the bootloader area (\[PB-cfg\]
  `PBDRV_CONFIG_USB_STM32F4_HUB_VARIANT_ADDR`). Reading is fine, writing is
  not.
- The vector table is at `0x08008000`. `cortex-m-rt` (feature `set-vtor`)
  writes this address into `SCB->VTOR` at reset, in case the bootloader
  does not.

## RAM

| Address range | Size | Content |
|---------------|-----:|---------|
| `0x20000000–0x2003FFFF` | 256 KiB | SRAM1 |
| `0x20040000–0x2004FFFF` | 64 KiB | SRAM2 |

SRAM1 and SRAM2 are contiguous and used as one 320 KiB region (`.data`,
`.bss`, heap-free; the stack grows down from `0x20050000`).

spike-rtic reports that writing `0xDEADB007` to `0x2004FFF0` before a
reset makes the LEGO bootloader enter DFU mode. This firmware does not
use that yet; it is **unverified** and must be checked before relying on
it.

## Peripherals reserved by hardware functions

| Timer | Use |
|-------|-----|
| TIM1 | Motor PWM ports A, B (12 kHz) |
| TIM3 | Motor PWM ports E, F |
| TIM4 | Motor PWM ports C, D |
| TIM5 | Charger ISET PWM (96 kHz) |
| TIM6 | DAC trigger for the speaker |
| TIM8 | Bluetooth 32.768 kHz clock (unused, Bluetooth stays off) |
| TIM9 | Embassy time driver |
| TIM12 | TLC5955 GSCLK (9.6 MHz) |
