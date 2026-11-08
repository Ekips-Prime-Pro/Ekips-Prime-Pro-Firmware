# Ekips Prime Pro Firmware – Overview

Custom firmware for the LEGO® Education SPIKE™ Prime hub (STM32F413VGT6),
written in `no_std` Rust with Embassy. It will run Ekips Prime Pro programs
through the `Hardware` trait from `ekips-hub-api`.

> **Untested on hardware.** No hub has been available during development.
> The firmware compiles, but nothing has been flashed or run yet.

## Contents

- [Decisions](decisions.md) – why things are the way they are.
- [Building](building.md)
- [Memory map](memory-map.md)
- [Pinout](pinout.md)
- [Drivers](drivers.md)
- [`Hardware` trait](hardware-trait.md)
- [Debugging](debugging.md)

## Ground rules

- The LEGO DFU bootloader (`0x08000000–0x08007FFF`) is never touched.
  The firmware starts at `0x08008000`.
- USB serial (CDC-ACM) only, Bluetooth stays off.
