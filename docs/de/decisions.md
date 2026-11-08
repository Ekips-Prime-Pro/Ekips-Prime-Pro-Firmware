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

## D-004 – Embassy-Zeittreiber auf TIM9

- **Kontext:** Embassy braucht einen Hardware-Timer. TIM1/3/4 treiben die
  Motoren, TIM5 das Ladegerät, TIM6 den DAC, TIM8 den Bluetooth-Takt und
  TIM12 den TLC5955 (alles aus Pybricks' `platform.c`). `time-driver-any`
  könnte einen davon wählen.
- **Entscheidung:** Feature `time-driver-tim9`, Tickrate 1 MHz (TIM9 läuft
  mit 96 MHz an APB2, Prescaler 96).
- **Konsequenzen:** TIM2, TIM7, TIM10, TIM11, TIM13, TIM14 bleiben frei.

## D-005 – Eigener `xtask` statt cargo-binutils

- **Kontext:** `dfu-util` braucht eine Rohbinärdatei. `cargo objcopy`
  erfordert zusätzliche Installationen und prüft nichts.
- **Entscheidung:** `xtask` liest die ELF-Program-Header selbst (ohne
  Abhängigkeiten) und lehnt Images ab, die den Bootloader berühren.
- **Konsequenzen:** Ein Befehl (`cargo xtask bin`) für alle Plattformen;
  die Brick-Schutz-Prüfung ist unit-getestet.

## D-006 – Hardwareunabhängige Logik in eigenem Crate

- **Kontext:** Ohne Hub sind Host-Unit-Tests die einzige Möglichkeit,
  irgendetwas zu verifizieren. Das Firmware-Crate baut nur für das
  Embedded-Target.
- **Entscheidung:** Protokoll-Parsing, Umrechnungen und Zustandsautomaten
  liegen in `crates/hub-logic` (`no_std`, kein Embassy). Die Treiber in der
  Firmware sind dünne Hüllen darum.
- **Konsequenzen:** `cargo test -p ekips-hub-logic --target host-tuple`
  deckt die Logik ab; ungetestet bleibt nur der Registerzugriff.

## D-003 – Power-Latch in `__pre_init`, SWD nur mit `swd-debug`

- **Kontext:** PA13 muss früh auf HIGH gesetzt werden, sonst schaltet sich
  der Hub im Akkubetrieb selbst ab (Pybricks macht das in `SystemInit`,
  spike-rtic in `#[pre_init]`). PA13/PA14 sind außerdem die SWD-Pins.
- **Entscheidung:** Der Latch wird in einem in Assembler geschriebenen
  `__pre_init` gesetzt (cortex-m-rt ruft ihn vor der RAM-Initialisierung
  auf; Rust-Code ist dort nicht erlaubt). Das Feature `swd-debug` lässt ihn
  weg und PA13/PA14 unverändert. Siehe [debugging.md](debugging.md).
- **Konsequenzen:** Normale Builds sind nicht per SWD debugbar.
  `swd-debug`-Builds brauchen USB-Versorgung und haben kein Port-VCC.

## D-007 – Panic-Handler: sicherer Zustand, dann ausschalten

- **Kontext:** Eine Panic darf keine Motoren laufen lassen und den Akku
  nicht endlos leeren.
- **Entscheidung:** Der Panic-Handler sperrt Interrupts, setzt alle
  H-Brücken-Eingänge auf LOW (Auslaufen), schaltet Lautsprecherverstärker
  und Bluetooth-Chip ab, loggt die Meldung über defmt, wartet ca. 10 s und
  gibt den Power-Latch frei.
- **Konsequenzen:** Bei USB-Versorgung bleibt der Hub an, aber untätig, bis
  zum Reset oder Abstecken.
