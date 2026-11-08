# Drivers

All drivers are **untested on hardware**. "Logic tested" means the
hardware-independent part in `crates/hub-logic` has host unit tests.

| # | Driver | Module | Status |
|--:|--------|--------|--------|
| 1 | Power latch, port VCC, power-off | `drivers::power` | compiles |
| 2 | Analog inputs (ADC1) | `drivers::analog` | compiles |
| 2 | Buttons + power-off by long press | `drivers::buttons` | compiles, logic tested |
| 3 | TLC5955 LED driver | `drivers::tlc5955` | compiles, frames tested |
| 3 | Status light (center button) | `drivers::status_light` | compiles, color logic tested |
| 4 | USB CDC-ACM console (echo) | `drivers::usb` | compiles |
| 5 | 5x5 light matrix | `drivers::matrix` | compiles, font/scrolling tested |
| 6 | Battery + charger | `drivers::battery` | compiles, conversions/state machine tested |
| 7 | IMU LSM6DS3TR-C | `drivers::imu` | compiles, conversion/heading tested |
| 8 | Speaker | `drivers::speaker` | compiles, waveform tested |

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

## 3. TLC5955 and status light

The status light sits on the TLC5955, so the TLC5955 driver comes first
(it is reused by the 5x5 matrix and the charger MODE pin).

- SPI1 at 24 MHz, mode 0, MSB first, TX DMA2 stream 3 / channel 3. Only
  SCK and MOSI are used (MISO would read back the shift register).
- Frame: 97 bytes (769 bits). Control frame byte-for-byte as Pybricks'
  `TLC5955_CONTROL_DATA` macro with DC 127, MC 3.2 mA, BC 127, auto
  repeat, ES-PWM. It is sent twice at start-up (the max current setting
  only takes effect the second time).
- Grayscale: channel *n* at bytes `2n+1, 2n+2`, big endian (channels are
  mapped to the GS registers in reverse order).
- LAT (PA15) pulses HIGH for about 1 µs after each frame.
- GSCLK: TIM12_CH2 on PB15, 9.6 MHz, 50 %.
- Colors: Pybricks' chromaticity and brightness correction
  (`led_pwm.c`), taking RGB instead of HSV.
- The status light is the two RGB LEDs of the center button
  (top 5/4/3, bottom 8/7/6). The firmware sets it to green once running.

## 4. USB CDC-ACM console

- USB OTG FS, internal PHY, PA11/PA12, VBUS sense on PA9 (the hub is
  self-powered, so VBUS detection is on).
- VID:PID `1209:0001` (decision D-008), manufacturer "Ekips Prime Pro",
  product "Ekips Prime Pro Hub".
- Everything the host sends is echoed back. `usb::write()` queues text for
  the host (512-byte pipe; data that does not fit is dropped so a missing
  host never blocks a program).
- On Linux the hub should appear as `/dev/ttyACM0`; test with e.g.
  `picocom /dev/ttyACM0` (once hardware is available).

## 5. 5x5 light matrix

- Pixels are TLC5955 channels (`bsp::pins::TLC_MATRIX`, from Pybricks).
- `matrix::show_image(Image)` – brightness 0–100 % per pixel, squared to a
  duty cycle for perceived linearity.
- `matrix::show_text(&str)` – one character is shown centered and
  static; longer text scrolls right to left, one column per 80 ms, at
  60 % brightness. A new request interrupts running text.
- Font: our own 3x5 pixel font (0–9, A–Z, `- + . , : !`, space; unknown
  characters become `?`; lower case is shown as upper case).
- `matrix::set_orientation()` rotates the output. The default
  orientation is not verified on hardware.
- At start-up the firmware scrolls "EKIPS".

## 6. Battery and charging

- Every 250 ms: battery current `raw * 7300 / 4096` mA, voltage
  `raw * 9900 / 4096 + I * 3 / 16` mV (Pybricks), low-pass filtered.
- Charge estimate: piecewise linear between 6000 mV (0 %) and 8190 mV
  (100 %). **Approximation**, not calibrated.
- Temperature: Pybricks' 103AT NTC formula.
- Charger MP2639A: MODE via TLC5955 channel 14 (0 = on), ISET via
  TIM5_CH1 / PA0 at 96 kHz, `/CHG` from resistor ladder 0. Port of the
  Pybricks state machine: fault detection by counting `/CHG` transitions
  in 7 samples; after 1 h of continuous charging a 30 s pause so the
  chip's safety timer does not expire.
- Charge current: 500 mA (ISET 15 %) whenever USB is present, see D-009.
- Battery LED: orange charging, green full, red fault, off otherwise.
- On battery below 6000 mV for 5 s the hub powers off.

## 7. IMU (LSM6DS3TR-C)

- I2C2 at 400 kHz (SCL PB10 AF4, SDA PB3 AF9), DMA1 streams 7 (TX) and 2
  (RX), address 0x6A. Before the bus starts, SCL is pulsed 10 times as a
  GPIO to free a stuck chip (Pybricks does the same).
- Setup as Pybricks: software reset, BDU, 833 Hz, ±8 g, ±2000 dps, gyro
  data-ready on INT1 (pulsed), rounding gyro + accel. `WHO_AM_I` must be
  0x6A.
- INT1 (PB4, EXTI4) wakes the task for every sample; after 50 ms without
  an interrupt it reads anyway.
- Axis signs from Pybricks (`SIGN_X = -1, SIGN_Y = 1, SIGN_Z = -1`).
- Heading: integration of the z rate, clockwise positive (API
  convention). The gyro bias is learned whenever the hub is still for
  about 1 s (all rates < 1.5 °/s); while still, the heading is frozen.
  The sign is **not verified on hardware**.
- On I2C errors the chip is re-initialized every 500 ms.

## 8. Speaker

- DAC1 channel 1 on PA4, amplifier enable PC10 (only HIGH while a beep
  plays).
- One period (32 samples) of a square wave around mid level is played in
  a loop by circular DMA (DMA1 stream 5 / channel 7); TIM6 TRGO paces the
  DAC at `32 × f`. Frequencies are clamped to 20–10 000 Hz.
- `speaker::beep(freq_hz, duration_ms)` never blocks; a new beep
  replaces the running one. Volume 50 % by default (amplitude grows with
  the square of the volume).
- At start-up the hub beeps once (1 kHz, 100 ms).
