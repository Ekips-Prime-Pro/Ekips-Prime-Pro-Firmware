# Bauen

> Bauen ist ungefährlich: Es spricht nie mit einem Hub. Das Flashen ist
> separat in [flashing.md](flashing.md) beschrieben.

## Voraussetzungen

- Rust (stable, mindestens 1.89 – nötig für `--target host-tuple`).
  `rust-toolchain.toml` installiert das Target `thumbv7em-none-eabihf`,
  `rustfmt` und `clippy` automatisch.
- Sonst nichts. Die `.bin` erzeugt unser eigener `xtask`, daher werden
  weder `cargo-binutils` noch `arm-none-eabi-objcopy` benötigt.

## Befehle

| Befehl | Ergebnis |
|--------|----------|
| `cargo build --release` | ELF unter `target/thumbv7em-none-eabihf/release/ekips-prime-pro-firmware` |
| `cargo xtask bin` | Release-Build + `target/firmware.bin` |
| `cargo xtask bin --debug` | Dasselbe mit dem Dev-Profil |
| `cargo xtask bin --features swd-debug` | Image, das die SWD-Pins frei lässt (siehe [debugging.md](debugging.md)) |
| `cargo xtask check-elf <datei>` | Führt nur die Sicherheitsprüfungen auf einer ELF aus |
| `cargo test -p ekips-hub-logic --target host-tuple` | Unit-Tests der hardwareunabhängigen Logik |
| `cargo test --manifest-path xtask/Cargo.toml --target host-tuple` | Unit-Tests des Image-Builders |

## Was `cargo xtask bin` prüft

1. Jedes ladbare ELF-Segment liegt innerhalb von `0x08008000..0x08100000`.
   Schon ein einziges Byte unterhalb von `0x08008000` (LEGO-Bootloader)
   bricht den Build ab.
2. Das Image beginnt exakt bei `0x08008000` (Vektortabelle).
3. Der initiale Stackpointer zeigt ins RAM, und der Reset-Vektor ist eine
   Thumb-Adresse innerhalb der Firmware.

Lücken zwischen Segmenten werden mit `0xFF` (gelöschter Flash) gefüllt.

## Aufbau des Workspaces

- `.` – die Firmware (`no_std`, `no_main`, baut nur für
  `thumbv7em-none-eabihf`; `.cargo/config.toml` setzt dieses Target).
- `crates/hub-logic` – reine Logik (kein Embassy, keine Register), auf dem
  Host testbar.
- `xtask/` – Host-Werkzeug, absichtlich **kein** Workspace-Mitglied, weil es
  für den Host gebaut wird. Der Alias `cargo xtask` startet es mit
  `--target host-tuple`.

## CI

`.github/workflows/ci.yml` läuft bei jedem Push und Pull Request:
`cargo fmt`-Prüfung, `clippy -D warnings` (mit und ohne `swd-debug` sowie für `hub-logic` inkl. Tests auf dem Host),
Host-Unit-Tests, `cargo xtask bin` und eine Syntaxprüfung der
Shell-Skripte. Das Image wird als Build-Artefakt hochgeladen.
