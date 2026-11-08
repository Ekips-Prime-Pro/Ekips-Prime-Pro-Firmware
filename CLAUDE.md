# CLAUDE.md – Ekips Prime Pro Firmware

Guidance for AI assistants (and humans) working in this repository.

## Project context

- Custom firmware for the **LEGO® Education SPIKE™ Prime hub**
  (STM32F413VGT6, Cortex-M4F, 1 MiB flash, 320 KiB SRAM).
- Written in `no_std` Rust with [Embassy](https://embassy.dev)
  (`embassy-executor`, `embassy-stm32` with feature `stm32f413vg`,
  `embassy-time`, `embassy-usb`).
- Part of **Ekips Prime Pro**. The language, compiler, bytecode VM and the
  `ekips-hub-api` crate live in
  [ekips-prime-pro-core](https://github.com/Ekips-Prime-Pro/ekips-prime-pro-core).
  The firmware implements the `Hardware` trait from `ekips-hub-api`
  (git dependency pinned to rev `81a4bd574b93502b0347d641d12bb7e7b2519890`;
  switch to tags once the core publishes them).
- Hardware knowledge comes from
  [Pybricks](https://github.com/pybricks/pybricks-micropython)
  (`lib/pbio/platform/prime_hub_f4/`) and
  [spike-rtic](https://github.com/springyworks/spike-rtic). Both are MIT
  licensed; see `NOTICE`.

## Hard rules

1. **English only** for code, comments, identifiers, commit messages.
2. **Document every feature and every decision immediately** under `docs/`,
   **always in both languages**: `docs/en/...` and `docs/de/...` with the
   same file names. Keep `docs/README.md` (index) up to date.
3. `cargo fmt --all -- --check` and
   `cargo clippy --all-targets -- -D warnings` must be green. CI builds for
   `thumbv7em-none-eabihf`.
4. **There is no hub to test on yet.** Only compile. Never flash, never run
   `scripts/*.sh` against a device. Everything that depends on hardware is
   marked **"untested on hardware"** (in rustdoc and in the docs).
5. **Brick protection:** the LEGO DFU bootloader occupies
   `0x08000000–0x08007FFF` (flash sectors 0 and 1) and must **never** be
   overwritten or erased. The firmware starts at `0x08008000` (`memory.x`).
   `cargo xtask bin` refuses to produce an image that touches the bootloader.
6. Every pin constant in `src/bsp/` cites its source (Pybricks and/or
   spike-rtic, file and commit).
7. Small commits, push regularly.

## Layout

| Path | Content |
|------|---------|
| `src/main.rs` | Entry point, early power latch, task spawning |
| `src/bsp/` | Board support: pin map, clocks, raw GPIO helpers |
| `src/drivers/` | Hardware drivers (Embassy based, untested on hardware) |
| `src/hub.rs` | `ekips_hub_api::Hardware` implementation (skeleton) |
| `crates/hub-logic/` | Hardware-independent logic, unit tested on the host |
| `xtask/` | Host tool: ELF → `.bin` with bootloader overlap check |
| `scripts/` | Backup / flash / restore scripts (**do not run without a hub and reading the docs**) |
| `docs/en`, `docs/de` | Bilingual documentation |

## Commands

```sh
cargo build --release                       # firmware ELF (thumbv7em-none-eabihf)
cargo xtask bin                             # target/firmware.bin, checked against the bootloader range
cargo test -p ekips-hub-logic --target host-tuple
cargo fmt --all -- --check
cargo clippy --all-targets -- -D warnings
```

## Notes

- PA13 is both the **power latch** and **SWDIO**; PA14 is both **port VCC
  enable** and **SWCLK**. See `docs/en/decisions.md` (D-003) before touching
  them. The `swd-debug` feature leaves them in SWD mode.
- Embassy's time driver uses **TIM9**; TIM1/3/4 (motors), TIM5 (charger
  ISET), TIM6 (DAC trigger), TIM8 (Bluetooth clock) and TIM12 (TLC5955
  GSCLK) are reserved for hardware functions.
