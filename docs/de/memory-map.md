# Speicherbelegung

Maßgeblich ist [`memory.x`](../../memory.x). Chip: STM32F413VGT6
(1 MiB Flash, 320 KiB SRAM).

## Flash

| Sektor | Adressbereich | Größe | Inhalt |
|-------:|---------------|------:|--------|
| 0 | `0x08000000–0x08003FFF` | 16 KiB | **LEGO-DFU-Bootloader – nie schreiben oder löschen** |
| 1 | `0x08004000–0x08007FFF` | 16 KiB | **LEGO-DFU-Bootloader – nie schreiben oder löschen** |
| 2 | `0x08008000–0x0800BFFF` | 16 KiB | Firmware: Vektortabelle, Code |
| 3 | `0x0800C000–0x0800FFFF` | 16 KiB | Firmware |
| 4 | `0x08010000–0x0801FFFF` | 64 KiB | Firmware |
| 5–11 | `0x08020000–0x080FFFFF` | 7 × 128 KiB | Firmware |

- Die Firmware darf bis zu **992 KiB** belegen (`0x08008000–0x080FFFFF`).
- Weil der Bootloader genau an einer Sektorgrenze endet, löscht das
  Schreiben der Firmware per DFU oder SWD nur Sektor 2 und höher. Weder
  `dfu-util -s 0x08008000` noch `probe-rs` (löscht nur Sektoren, die die ELF
  belegt) berühren Sektor 0 und 1.
- Pybricks liest die Hub-Variante (SPIKE Prime vs. Robot Inventor) aus
  `0x08007D80`, also **innerhalb** des Bootloader-Bereichs (\[PB-cfg\]
  `PBDRV_CONFIG_USB_STM32F4_HUB_VARIANT_ADDR`). Lesen ist in Ordnung,
  Schreiben nicht.
- Die Vektortabelle liegt bei `0x08008000`. `cortex-m-rt` (Feature
  `set-vtor`) schreibt diese Adresse beim Reset in `SCB->VTOR`, falls der
  Bootloader es nicht tut.

## RAM

| Adressbereich | Größe | Inhalt |
|---------------|------:|--------|
| `0x20000000–0x2003FFFF` | 256 KiB | SRAM1 |
| `0x20040000–0x2004FFFF` | 64 KiB | SRAM2 |

SRAM1 und SRAM2 liegen direkt hintereinander und werden als ein
320-KiB-Bereich genutzt (`.data`, `.bss`, kein Heap; der Stack wächst von
`0x20050000` nach unten).

spike-rtic berichtet, dass `0xDEADB007` an `0x2004FFF0` vor einem Reset den
LEGO-Bootloader in den DFU-Modus schickt. Diese Firmware nutzt das noch
nicht; es ist **nicht verifiziert** und muss vor einer Verwendung geprüft
werden.

## Durch Hardwarefunktionen belegte Peripherie

| Timer | Verwendung |
|-------|-----------|
| TIM1 | Motor-PWM Ports A, B (12 kHz) |
| TIM3 | Motor-PWM Ports E, F |
| TIM4 | Motor-PWM Ports C, D |
| TIM5 | Lade-ISET-PWM (96 kHz) |
| TIM6 | DAC-Trigger für den Lautsprecher |
| TIM8 | Bluetooth-32,768-kHz-Takt (ungenutzt, Bluetooth bleibt aus) |
| TIM9 | Embassy-Zeittreiber |
| TIM12 | TLC5955-GSCLK (9,6 MHz) |
