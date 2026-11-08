# Treiber

Alle Treiber sind **nicht auf Hardware getestet**. „Logik getestet“
bedeutet, dass der hardwareunabhängige Teil in `crates/hub-logic`
Host-Unit-Tests hat.

| # | Treiber | Modul | Status |
|--:|---------|-------|--------|
| 1 | Power-Latch, Port-VCC, Ausschalten | `drivers::power` | kompiliert |
| 2 | Analogeingänge (ADC1) | `drivers::analog` | kompiliert |
| 2 | Tasten + Ausschalten per langem Druck | `drivers::buttons` | kompiliert, Logik getestet |

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
