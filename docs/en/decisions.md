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

## D-003 – Power latch in `__pre_init`, SWD only with `swd-debug`

- **Context:** PA13 must be driven HIGH early or the hub turns itself off
  when running from battery (Pybricks does it in `SystemInit`, spike-rtic
  in `#[pre_init]`). PA13/PA14 are also the SWD pins.
- **Decision:** The latch is set in a `__pre_init` written in assembly
  (cortex-m-rt calls it before RAM initialization; Rust code is not
  allowed there). The feature `swd-debug` omits it and leaves PA13/PA14
  alone. See [debugging.md](debugging.md).
- **Consequences:** Normal builds cannot be debugged over SWD.
  `swd-debug` builds need USB power and have no port VCC.

## D-007 – Panic handler: safe state, then power off

- **Context:** A panic must not leave motors running or the hub draining
  the battery forever.
- **Decision:** The panic handler disables interrupts, sets all motor
  H-bridge inputs LOW (coast), switches off the speaker amplifier and the
  Bluetooth chip, logs the message via defmt, waits about 10 s and
  releases the power latch.
- **Consequences:** On USB power the hub stays on but inert until reset or
  unplugged.

## D-008 – USB IDs: pid.codes test ID, not LEGO's

- **Context:** The LEGO firmware uses VID `0x0694` (LEGO). Pretending to
  be a LEGO device would confuse the official apps and is not ours to use.
- **Decision:** Use the pid.codes test ID `1209:0001` during development.
- **Consequences:** Before a release an own PID has to be requested from
  pid.codes (free for open source hardware/firmware). The DFU bootloader
  keeps its own ID `0694:0011`; this does not change.
