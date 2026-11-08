# Decisions

Architecture and design decisions, newest last. Each entry: context,
decision, consequences.

## D-001 – Documentation in two languages

- **Context:** The project is used by German- and English-speaking people.
- **Decision:** Every document exists as `docs/en/<name>.md` and
  `docs/de/<name>.md`. Code, comments and commit messages are English only.
- **Consequences:** Every change to the docs touches both files.

## D-002 – Firmware starts at 0x08008000, bootloader is never touched

- **Context:** The LEGO DFU bootloader lives in flash sectors 0 and 1
  (`0x08000000–0x08007FFF`). It is the only way to recover the hub without
  opening it. The official LEGO firmware and spike-rtic both start at
  `0x08008000`. (Current Pybricks installs its own second-stage bootloader
  at `0x08008000` and starts at `0x08010000`; we do not need that.)
- **Decision:** `memory.x` places the vector table at `0x08008000`.
  The `.bin` is produced by `cargo xtask bin`, which refuses images that
  have any loadable section below `0x08008000` or above the end of flash.
- **Consequences:** Flashing always uses
  `dfu-util -d 0694:0011 -a 0 -s 0x08008000:leave -D firmware.bin`.

## D-004 – Embassy time driver on TIM9

- **Context:** Embassy needs one hardware timer. TIM1/3/4 drive the motors,
  TIM5 the charger, TIM6 the DAC, TIM8 the Bluetooth clock and TIM12 the
  TLC5955 (all from Pybricks' `platform.c`). `time-driver-any` might pick
  one of them.
- **Decision:** Feature `time-driver-tim9`, tick rate 1 MHz (TIM9 runs at
  96 MHz on APB2, prescaler 96).
- **Consequences:** TIM2, TIM7, TIM10, TIM11, TIM13, TIM14 remain free.

## D-005 – Own `xtask` instead of cargo-binutils

- **Context:** `dfu-util` needs a raw binary. `cargo objcopy` needs extra
  installs and does not check anything.
- **Decision:** `xtask` reads the ELF program headers itself (no
  dependencies) and refuses images that touch the bootloader.
- **Consequences:** One command (`cargo xtask bin`) for all platforms;
  the brick-protection check is unit tested.

## D-006 – Hardware-independent logic in its own crate

- **Context:** Without a hub, host unit tests are the only way to verify
  anything. The firmware crate only builds for the embedded target.
- **Decision:** Protocol parsing, conversions and state machines live in
  `crates/hub-logic` (`no_std`, no Embassy). Drivers in the firmware are
  thin wrappers around it.
- **Consequences:** `cargo test -p ekips-hub-logic --target host-tuple`
  covers the logic; only register access remains untested.
