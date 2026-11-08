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
