# Debugging (geplant)

> **Nicht auf Hardware getestet.** Nichts auf dieser Seite wurde bisher
> ausprobiert.

## Plan

- Debug-Probe: ein **Raspberry Pi Pico 2 W** mit debugprobe-Firmware
  (CMSIS-DAP), angeschlossen an die SWD-Testpunkte des Hubs (der Hub muss
  dafür geöffnet werden; die Lage der Testpunkte ist noch zu
  dokumentieren).
- Host-Software: [probe-rs](https://probe.rs). `.cargo/config.toml` setzt
  bereits `runner = "probe-rs run --chip STM32F413VGTx"`, sodass
  `cargo run --release --features swd-debug` per SWD flashen und
  `defmt`-Logs anzeigen würde.
- Logging: `defmt` über RTT (`defmt-rtt`). Ohne Probe wird der Log-Puffer
  einfach nie ausgelesen; das kostet etwas RAM, sonst nichts.

## Der Konflikt PA13 / PA14 (Entscheidung D-003)

Beim STM32F413 sind PA13 nach dem Reset **SWDIO** und PA14 **SWCLK**. Auf
dem SPIKE-Prime-Hub sind dieselben Pins der **Power-Latch** (PA13) und die
**VCC-Freigabe der I/O-Ports** (PA14). Die normale Firmware schaltet beide
schon beim allerersten Befehl auf GPIO-Ausgang, was jede Debug-Probe
trennt.

Das Cargo-Feature `swd-debug` ändert das:

| | normaler Build | `--features swd-debug` |
|-|----------------|------------------------|
| PA13 | GPIO, HIGH (Hub bleibt an) | bleibt SWDIO |
| PA14 | GPIO, Port-VCC bei Bedarf | bleibt SWCLK |
| Versorgung | Akku oder USB | **USB muss eingesteckt sein** |
| Sensoren / Motor-Encoder | funktionieren | **kein Port-VCC → funktionieren nicht** |
| Ausschalten | gibt PA13 frei | stoppt nur alles; Hub bleibt über USB an |

Auf Hardware zu prüfen: ob der interne SWDIO-Pull-up allein den
Power-Latch hält und ob Motoren ohne Port-VCC laufen (die Motorleistung
kommt vom Akku, die Encoder brauchen VCC).

## Flashen per SWD

probe-rs löscht nur die Flash-Sektoren, die die ELF belegt. Weil die
Firmware in Sektor 2 (`0x08008000`) beginnt, wird der Bootloader in den
Sektoren 0–1 nicht gelöscht. **Niemals** „Erase chip“ / `probe-rs erase`
verwenden – das würde den LEGO-Bootloader löschen.
