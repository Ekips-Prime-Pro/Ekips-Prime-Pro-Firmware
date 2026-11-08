# Ekips Prime Pro – Firmware

Custom firmware for the LEGO® Education SPIKE™ Prime hub (STM32F413), written
in `no_std` Rust with [Embassy](https://embassy.dev). It runs Ekips Prime Pro
programs in the bytecode VM from
[Ekips-Prime-Pro-Core](https://github.com/Ekips-Prime-Pro/Ekips-Prime-Pro-Core).

> **Status:** early development, **untested on hardware**. See [docs/](docs/README.md).

## Plan

- Keeps the original LEGO DFU bootloader (never writes `0x08000000–0x08007FFF`);
  the firmware lives at `0x08008000`.
- USB serial (CDC-ACM) only; Bluetooth stays off.
- Drivers: power latch, LED matrix, buttons, speaker, battery/charging,
  IMU (gyro), motors with PID control, LPF2/LUMP sensors
  (color, distance, force).
- Implements the `Hardware` trait from `ekips-hub-api`.

Build and flash (hub in DFU mode) – **read [docs/en/flashing.md](docs/en/flashing.md)
([deutsch](docs/de/flashing.md)) first, back up the original firmware, and note
that the firmware is untested on hardware**:

```sh
cargo xtask bin
dfu-util -d 0694:0011 -a 0 -s 0x08008000:leave -D target/firmware.bin
```

The official LEGO firmware can always be restored via the DFU bootloader.

## Credits

The hardware knowledge in this firmware is derived from these MIT-licensed
projects (copyright notices in [NOTICE](NOTICE)):

- [Pybricks](https://github.com/pybricks/pybricks-micropython) – hardware
  details (pin mapping, LUMP protocol, motor control).
  © The Pybricks Authors.
- [spike-rtic](https://github.com/springyworks/spike-rtic) – bare-metal Rust
  reference for the same hub. © spike-rtic contributors.

LEGO and SPIKE are trademarks of the LEGO Group, which does not sponsor or
endorse this project.

## License

[MIT](LICENSE)
