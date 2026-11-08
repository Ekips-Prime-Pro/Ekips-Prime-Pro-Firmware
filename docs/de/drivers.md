# Treiber

Alle Treiber sind **nicht auf Hardware getestet**. „Logik getestet“
bedeutet, dass der hardwareunabhängige Teil in `crates/hub-logic`
Host-Unit-Tests hat.

| # | Treiber | Modul | Status |
|--:|---------|-------|--------|
| 1 | Power-Latch, Port-VCC, Ausschalten | `drivers::power` | kompiliert |
| 2 | Analogeingänge (ADC1) | `drivers::analog` | kompiliert |
| 2 | Tasten + Ausschalten per langem Druck | `drivers::buttons` | kompiliert, Logik getestet |
| 3 | TLC5955-LED-Treiber | `drivers::tlc5955` | kompiliert, Frames getestet |
| 3 | Status-LED (Mitteltaste) | `drivers::status_light` | kompiliert, Farblogik getestet |
| 4 | USB-CDC-ACM-Konsole (Echo) | `drivers::usb` | kompiliert |
| 5 | 5×5-LED-Matrix | `drivers::matrix` | kompiliert, Schrift/Lauftext getestet |
| 6 | Akku + Ladegerät | `drivers::battery` | kompiliert, Umrechnung/Zustandsautomat getestet |

## 1. Power-Latch

- PA13 wird in `__pre_init` (Assembler) auf HIGH gesetzt, noch vor der
  RAM-Initialisierung und vor dem Start von Embassy (Entscheidung D-003).
  `power::hold()` wiederholt das.
- `power::set_port_vcc(on)` schaltet PA14 (3,3 V an allen I/O-Ports).
- `power::power_off()` bringt alle Aktoren in den sicheren Zustand,
  schaltet Port-VCC ab und gibt PA13 frei. Mit eingestecktem USB bleibt der
  Hub versorgt; die Funktion wartet dann für immer.
- Mit dem Feature `swd-debug` werden PA13/PA14 nie angefasst.

## 2. Analogeingänge

Ein Task besitzt ADC1 und liest alle 10 ms die sechs Kanäle, die Pybricks
nutzt: Akkustrom (PC0/Kanal 10), Akkuspannung (PC1/Kanal 11), Akku-NTC
(PB0/Kanal 8), USB-Ladestrom (PA3/Kanal 3), Widerstandsleiter 0
(PC4/Kanal 14) und Widerstandsleiter 1 (PA1/Kanal 1). Software-getriggerte
blockierende Messungen ersetzen den TIM2-+-DMA-Trigger von Pybricks
(einfacher, TIM2 bleibt frei).

## 2. Tasten

- Leiter-Dekodierung aus Pybricks `resistor_ladder.c` portiert;
  Schwellwerte aus `platform.c` L660-671.
- Zuordnung: Leiter 0 CH1 = Mitte; Leiter 1 CH0 = links, CH1 = rechts,
  CH2 = Bluetooth. Leiter 0 CH2 ist die `/CHG`-Leitung des Laders.
- Entprellung: 3 gleiche Messungen (30 ms).
- Ausschalten: Mitteltaste 2 s halten. Der Druck, mit dem der Hub
  eingeschaltet wurde, zählt nicht – die Taste muss seit dem Start einmal
  losgelassen worden sein.
- Messwerte unterhalb der niedrigsten Schwelle (ADC noch nicht
  eingeschwungen, Hardwarefehler) werden ignoriert.

## 3. TLC5955 und Status-LED

Die Status-LED hängt am TLC5955, daher kommt der TLC5955-Treiber zuerst
(er wird von der 5×5-Matrix und dem MODE-Pin des Laders mitbenutzt).

- SPI1 mit 24 MHz, Modus 0, MSB zuerst, TX über DMA2 Stream 3 / Kanal 3.
  Nur SCK und MOSI werden genutzt (MISO würde das Schieberegister
  zurücklesen).
- Frame: 97 Bytes (769 Bit). Control-Frame Byte für Byte wie Pybricks'
  Makro `TLC5955_CONTROL_DATA` mit DC 127, MC 3,2 mA, BC 127, Auto-Repeat,
  ES-PWM. Er wird beim Start zweimal gesendet (die Maximalstrom-Einstellung
  greift erst beim zweiten Mal).
- Graustufen: Kanal *n* in den Bytes `2n+1, 2n+2`, Big Endian (die Kanäle
  liegen in umgekehrter Reihenfolge in den GS-Registern).
- LAT (PA15) geht nach jedem Frame für ca. 1 µs auf HIGH.
- GSCLK: TIM12_CH2 an PB15, 9,6 MHz, 50 %.
- Farben: Farb- und Helligkeitskorrektur von Pybricks (`led_pwm.c`), mit
  RGB statt HSV als Eingabe.
- Die Status-LED sind die zwei RGB-LEDs der Mitteltaste (oben 5/4/3, unten
  8/7/6). Die Firmware setzt sie nach dem Start auf Grün.

## 4. USB-CDC-ACM-Konsole

- USB OTG FS, interner PHY, PA11/PA12, VBUS-Erkennung an PA9 (der Hub hat
  eine eigene Versorgung, daher ist die VBUS-Erkennung an).
- VID:PID `1209:0001` (Entscheidung D-008), Hersteller „Ekips Prime Pro“,
  Produkt „Ekips Prime Pro Hub“.
- Alles, was der Host sendet, wird zurückgeschickt. `usb::write()` stellt
  Text für den Host in eine Warteschlange (512-Byte-Pipe; was nicht passt,
  wird verworfen, damit ein fehlender Host nie ein Programm blockiert).
- Unter Linux sollte der Hub als `/dev/ttyACM0` erscheinen; testen z. B.
  mit `picocom /dev/ttyACM0` (sobald Hardware da ist).

## 5. 5×5-LED-Matrix

- Die Pixel sind TLC5955-Kanäle (`bsp::pins::TLC_MATRIX`, aus Pybricks).
- `matrix::show_image(Image)` – Helligkeit 0–100 % pro Pixel, quadriert
  in einen Duty-Cycle umgerechnet (wirkt linearer).
- `matrix::show_text(&str)` – ein Zeichen wird zentriert und statisch
  angezeigt; längerer Text läuft von rechts nach links, eine Spalte pro
  80 ms, mit 60 % Helligkeit. Eine neue Anfrage unterbricht laufenden
  Text.
- Schrift: eigene 3×5-Pixel-Schrift (0–9, A–Z, `- + . , : !`, Leerzeichen;
  unbekannte Zeichen werden zu `?`; Kleinbuchstaben werden groß
  dargestellt).
- `matrix::set_orientation()` dreht die Ausgabe. Die Standardausrichtung
  ist nicht auf Hardware geprüft.
- Beim Start läuft „EKIPS“ durch.

## 6. Akku und Laden

- Alle 250 ms: Akkustrom `raw * 7300 / 4096` mA, Spannung
  `raw * 9900 / 4096 + I * 3 / 16` mV (Pybricks), tiefpassgefiltert.
- Ladezustand: stückweise linear zwischen 6000 mV (0 %) und 8190 mV
  (100 %). **Näherung**, nicht kalibriert.
- Temperatur: NTC-Formel (103AT) aus Pybricks.
- Ladegerät MP2639A: MODE über TLC5955-Kanal 14 (0 = an), ISET über
  TIM5_CH1 / PA0 mit 96 kHz, `/CHG` über Widerstandsleiter 0. Portierung
  des Pybricks-Zustandsautomaten: Fehlererkennung über die Anzahl der
  `/CHG`-Wechsel in 7 Messungen; nach 1 h Dauerladen 30 s Pause, damit der
  Sicherheitstimer des Chips nicht abläuft.
- Ladestrom: 500 mA (ISET 15 %), sobald USB anliegt, siehe D-009.
- Akku-LED: Orange = lädt, Grün = voll, Rot = Fehler, sonst aus.
- Im Akkubetrieb unter 6000 mV für 5 s schaltet sich der Hub ab.
