# Debugging (planned)

> **Untested on hardware.** Nothing on this page has been tried yet.

## Plan

- Debug probe: a **Raspberry Pi Pico 2 W** running debugprobe firmware
  (CMSIS-DAP), connected to the hub's SWD test pads (the hub has to be
  opened; the pad positions still need to be documented).
- Host software: [probe-rs](https://probe.rs). `.cargo/config.toml`
  already sets `runner = "probe-rs run --chip STM32F413VGTx"`, so
  `cargo run --release --features swd-debug` would flash over SWD and show
  `defmt` logs.
- Logging: `defmt` over RTT (`defmt-rtt`). Without a probe the log buffer
  is simply never read; this costs a little RAM and nothing else.

## The PA13 / PA14 conflict (decision D-003)

On the STM32F413, PA13 is **SWDIO** and PA14 is **SWCLK** after reset. On
the SPIKE Prime hub the same pins are the **power latch** (PA13) and the
**I/O port VCC enable** (PA14). The normal firmware switches both to GPIO
outputs at the very first instruction, which disconnects any debug probe.

The cargo feature `swd-debug` changes this:

| | normal build | `--features swd-debug` |
|-|--------------|------------------------|
| PA13 | GPIO, HIGH (hub stays on) | left as SWDIO |
| PA14 | GPIO, port VCC on demand | left as SWCLK |
| Power | battery or USB | **USB must be plugged in** |
| Sensors / motor encoders | work | **no port VCC → do not work** |
| Power-off | releases PA13 | only stops everything; hub stays on via USB |

Unknowns to check on hardware: whether the internal SWDIO pull-up alone
holds the power latch, and whether motors run without port VCC (the motor
power comes from the battery, the encoders need VCC).

## Flashing over SWD

probe-rs only erases the flash sectors the ELF occupies. Because the
firmware starts at sector 2 (`0x08008000`), the bootloader in sectors 0–1
is not erased. **Never** use "erase chip" / `probe-rs erase` – that would
delete the LEGO bootloader.
