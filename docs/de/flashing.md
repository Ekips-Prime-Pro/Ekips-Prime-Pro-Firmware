# Backup, Flashen und Wiederherstellen der LEGO-Firmware

> ⚠️ **Diese Firmware lief noch nie auf einem echten Hub.** Sie ist nicht
> auf Hardware getestet. Flashen auf eigene Gefahr. Nichts in diesem
> Projekt wurde bisher an einem Gerät ausprobiert – auch nicht die
> Skripte auf dieser Seite.

> ⚠️ **Niemals nach `0x08000000–0x08007FFF` schreiben.** Dort liegt der
> LEGO-DFU-Bootloader. Solange er intakt ist, lässt sich der Hub immer über
> USB wiederherstellen. Alle Befehle hier schreiben nur ab `0x08008000`.
> Niemals „Erase chip“, `dfu-util -s 0x08000000` mit `-D`, Mass Erase oder
> ein SWD-„Erase all“ verwenden.

## Warum das (normalerweise) sicher ist

Der Hub hat in den Flash-Sektoren 0–1 einen LEGO-DFU-Bootloader, den
unsere Firmware nie anfasst (siehe [memory-map.md](memory-map.md)). Egal
was bei `0x08008000` geflasht ist: Der Bootloader startet weiterhin, wenn
der Hub in den DFU-Modus versetzt wird, sodass eine defekte Firmware
wieder ersetzt werden kann. Restrisiken:

- Ein Firmware-Fehler, der Hardware beschädigt (z. B. Motoren gegen einen
  Anschlag, falsche Ladeeinstellung). Unsere Firmware begrenzt den
  Ladestrom auf 500 mA und schaltet bei einer Panic alle Motoren auf
  Auslaufen – ungetestet.
- Tiefentladung des Akkus: Die Firmware schaltet unter 6,0 V ab, ebenfalls
  ungetestet.
- Abgebrochenes Flashen: einfach erneut in den DFU-Modus wechseln und
  wiederholen.

## Voraussetzungen

- `dfu-util` 0.9 oder neuer (Linux: Paket `dfu-util`; macOS:
  `brew install dfu-util`; Windows: von dfu-util.sourceforge.net).
- Linux: die udev-Regel [`scripts/99-lego-spike-dfu.rules`](../../scripts/99-lego-spike-dfu.rules)
  für Zugriff ohne root.
- Windows: Das DFU-Gerät braucht ggf. den WinUSB-Treiber (z. B. über
  Zadig, Gerät `0694:0011`). Keine Treiber anderer Geräte ändern.
- Ein USB-Kabel mit Datenleitungen. Ein geladener Akku.

## In den DFU-Modus wechseln

(Ablauf wie vom Pybricks-Projekt dokumentiert; noch zu bestätigen.)

1. USB-Kabel abziehen und den Hub ausschalten (Mitteltaste halten, bis er
   aus ist).
2. Die **Bluetooth-Taste** (oben rechts) drücken und **halten**.
3. Währenddessen das USB-Kabel einstecken.
4. Weiter halten, bis die Bluetooth-LED in wechselnden Farben blinkt,
   dann loslassen.
5. Prüfen: `dfu-util -l` muss `[0694:0011]` anzeigen.

Der Hub verlässt den DFU-Modus beim Abstecken oder nach dem Flashen mit
`:leave`.

## 1. LEGO-Firmware sichern

```bash
scripts/backup-lego-firmware.sh backup
```

Das **liest** nur den Flash und schreibt nach `backup/`:

- `lego-firmware-<datum>.bin` – 992 KiB ab `0x08008000` (der Teil, der
  ersetzt wird; nötig für die Wiederherstellung),
- `lego-full-<datum>.bin` – das komplette 1 MiB inklusive Bootloader (nur
  zur Referenz; niemals zurückschreiben),
- `SHA256SUMS-<datum>`.

Das Backup an einem sicheren Ort aufbewahren. Hinweis: Die offizielle
Firmware gibt es auch über die LEGO-Apps; das Backup ist eine zweite
Absicherung.

## 2. Ekips Prime Pro bauen und flashen

```bash
cargo xtask bin
```

```bash
scripts/flash-firmware.sh target/firmware.bin
```

Das Skript prüft Größe und Vektortabelle des Images, warnt bei fehlendem
Backup, fragt nach einer Bestätigung und führt dann genau Folgendes aus:

```bash
dfu-util -d 0694:0011 -a 0 -s 0x08008000:leave -D target/firmware.bin
```

`cargo xtask bin` hat zuvor bereits jedes Image abgelehnt, das den
Bootloader-Bereich berühren würde.

## 3. LEGO-Firmware wiederherstellen

Variante A (empfohlen): Den Hub mit der offiziellen LEGO-Education-SPIKE-
App verbinden; sie bietet ein Firmware-Update bzw. eine Wiederherstellung
an, wenn sie einen Hub ohne LEGO-Firmware findet (ggf. im DFU-Modus).

Variante B: Das eigene Backup zurückschreiben (nur der Firmware-Bereich):

```bash
scripts/restore-lego-firmware.sh backup/lego-firmware-<datum>.bin
```

Das Skript lehnt die Datei `lego-full-*` ab, weil sie den Bootloader
enthält.

## Wenn etwas schiefgeht

| Symptom | Was tun |
|---------|---------|
| Hub geht nach dem Flashen nicht an / bleibt dunkel | DFU-Modus aktivieren und erneut flashen oder wiederherstellen. Ohne Akku und mit USB wird der Hub über USB versorgt. |
| `dfu-util` findet kein Gerät | Kabel, DFU-Modus, udev-Regel / WinUSB-Treiber prüfen. |
| Flashen abgebrochen | Erneut in den DFU-Modus wechseln und wiederholen. |
| Hub schaltet im Akkubetrieb sofort ab | Der Power-Latch funktioniert evtl. nicht – bitte melden; USB-Versorgung nutzen. |
| Hub wird beim Laden warm | USB abziehen und melden. |
