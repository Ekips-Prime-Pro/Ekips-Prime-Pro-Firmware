# Building

> Building is safe: it never talks to a hub. Flashing is described
> separately in [flashing.md](flashing.md).

## Requirements

- Rust (stable, at least 1.89 – needed for `--target host-tuple`).
  `rust-toolchain.toml` installs the target `thumbv7em-none-eabihf`,
  `rustfmt` and `clippy` automatically.
- Nothing else. The `.bin` is produced by our own `xtask`, so neither
  `cargo-binutils` nor `arm-none-eabi-objcopy` is required.

## Commands

| Command | Result |
|---------|--------|
| `cargo build --release` | ELF at `target/thumbv7em-none-eabihf/release/ekips-prime-pro-firmware` |
| `cargo xtask bin` | Release build + `target/firmware.bin` |
| `cargo xtask bin --debug` | Same with the dev profile |
| `cargo xtask bin --features swd-debug` | Image that keeps the SWD pins (see [debugging.md](debugging.md)) |
| `cargo xtask check-elf <file>` | Only runs the safety checks on an ELF |
| `cargo test -p ekips-hub-logic --target host-tuple` | Unit tests of the hardware-independent logic |
| `cargo test --manifest-path xtask/Cargo.toml --target host-tuple` | Unit tests of the image builder |

## What `cargo xtask bin` checks

1. Every loadable ELF segment lies inside `0x08008000..0x08100000`.
   A single byte below `0x08008000` (the LEGO bootloader) aborts the build.
2. The image starts exactly at `0x08008000` (the vector table).
3. The initial stack pointer points into RAM and the reset vector is a
   Thumb address inside the firmware.

Gaps between segments are filled with `0xFF` (erased flash).

## Workspace layout

- `.` – the firmware (`no_std`, `no_main`, only builds for
  `thumbv7em-none-eabihf`; `.cargo/config.toml` sets this target).
- `crates/hub-logic` – pure logic (no Embassy, no registers), testable on
  the host.
- `xtask/` – host tool, deliberately **not** a workspace member because it
  is built for the host. The alias `cargo xtask` runs it with
  `--target host-tuple`.

## CI

`.github/workflows/ci.yml` runs on every push and pull request: `cargo fmt`
check, `clippy -D warnings` (with and without `swd-debug`, and for `hub-logic` incl. tests on the host), host unit
tests, `cargo xtask bin` and a syntax check of the shell scripts. The
image is uploaded as a build artifact.
