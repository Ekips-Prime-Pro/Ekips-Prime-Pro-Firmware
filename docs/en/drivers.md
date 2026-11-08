# Drivers

All drivers are **untested on hardware**. "Logic tested" means the
hardware-independent part in `crates/hub-logic` has host unit tests.

| # | Driver | Module | Status |
|--:|--------|--------|--------|
| 1 | Power latch, port VCC, power-off | `drivers::power` | compiles |
| 2 | Analog inputs (ADC1) | `drivers::analog` | compiles |
| 2 | Buttons + power-off by long press | `drivers::buttons` | compiles, logic tested |

## 1. Power latch

- PA13 is driven HIGH in `__pre_init` (assembly), before RAM init and
  before Embassy starts (decision D-003). `power::hold()` repeats it.
- `power::set_port_vcc(on)` switches PA14 (3.3 V on all I/O ports).
- `power::power_off()` puts all actuators into the safe state, switches
  port VCC off and releases PA13. With USB plugged in the hub stays
  powered; the function then waits forever.
- With the `swd-debug` feature PA13/PA14 are never touched.

## 2. Analog inputs

One task owns ADC1 and reads, every 10 ms, the six channels Pybricks uses:
battery current (PC0/ch10), battery voltage (PC1/ch11), battery NTC
(PB0/ch8), USB charge current (PA3/ch3), resistor ladder 0 (PC4/ch14) and
resistor ladder 1 (PA1/ch1). Software-triggered blocking reads replace
Pybricks' TIM2 + DMA trigger (simpler, keeps TIM2 free).

## 2. Buttons

- Ladder decoding ported from Pybricks `resistor_ladder.c`; thresholds
  from `platform.c` L660-671.
- Mapping: ladder 0 CH1 = center; ladder 1 CH0 = left, CH1 = right,
  CH2 = Bluetooth. Ladder 0 CH2 is the charger `/CHG` line.
- Debouncing: 3 identical samples (30 ms).
- Power-off: center held for 2 s. The press that switched the hub on does
  not count – the button must have been released once since boot.
- Readings below the lowest threshold (ADC not settled, hardware fault)
  are ignored.
