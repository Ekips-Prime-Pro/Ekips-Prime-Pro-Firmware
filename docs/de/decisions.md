# Entscheidungen

Architektur- und Designentscheidungen, neueste zuletzt. Jeder Eintrag:
Kontext, Entscheidung, Konsequenzen.

## D-001 – Dokumentation in zwei Sprachen

- **Kontext:** Das Projekt wird von deutsch- und englischsprachigen Menschen
  genutzt.
- **Entscheidung:** Jedes Dokument existiert als `docs/en/<name>.md` und
  `docs/de/<name>.md`. Code, Kommentare und Commit-Messages sind
  ausschließlich Englisch.
- **Konsequenzen:** Jede Doku-Änderung betrifft beide Dateien.

## D-002 – Firmware beginnt bei 0x08008000, Bootloader wird nie angefasst

- **Kontext:** Der LEGO-DFU-Bootloader liegt in den Flash-Sektoren 0 und 1
  (`0x08000000–0x08007FFF`). Er ist der einzige Weg, den Hub ohne Öffnen
  wiederherzustellen. Die offizielle LEGO-Firmware und spike-rtic beginnen
  beide bei `0x08008000`. (Aktuelles Pybricks installiert einen eigenen
  Second-Stage-Bootloader bei `0x08008000` und beginnt bei `0x08010000`;
  das brauchen wir nicht.)
- **Entscheidung:** `memory.x` legt die Vektortabelle auf `0x08008000`.
  Die `.bin` wird von `cargo xtask bin` erzeugt, das Images ablehnt, die
  eine ladbare Sektion unterhalb von `0x08008000` oder hinter dem Flash-Ende
  enthalten.
- **Konsequenzen:** Geflasht wird immer mit
  `dfu-util -d 0694:0011 -a 0 -s 0x08008000:leave -D firmware.bin`.
