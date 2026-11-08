# Pinout

Source of truth in code: [`src/bsp/pins.rs`](../../src/bsp/pins.rs). Keep both in sync.

> **Untested on hardware.** Values are taken from the sources below, not
> measured by this project.

## Sources

- **PB** – Pybricks, [`lib/pbio/platform/prime_hub_f4/platform.c`](https://github.com/pybricks/pybricks-micropython/blob/c860b6a00301fb7c6ecd429da11dcb5a6c557c7a/lib/pbio/platform/prime_hub_f4/platform.c)
  at commit `c860b6a` (MIT). Line numbers refer to that commit. Older
  versions call the directory `prime_hub`.
- **SR** – spike-rtic, [`src/pins.rs`](https://github.com/springyworks/spike-rtic/blob/e6060b0beef16d3635b9d63f2ac829f5519be7d8/src/pins.rs)
  at commit `e6060b0` (MIT).

`src/bsp/pins.rs` contains a compile-time check that no GPIO and no
TLC5955 channel is assigned twice.

## Hub functions

| Pin | Function | Mode / AF | Source |
|-----|-----|-----|-----|
| PA13 | Power latch (HIGH = stay on) / SWDIO | GPIO out | PB L651-656, L1086-1092; SR L29-31 |
| PA14 | Port VCC enable (all 6 ports) / SWCLK | GPIO out | PB L228-231; SR L33-34 |
| PA11 | USB D- | AF10 | PB L968-974; SR L37-38 |
| PA12 | USB D+ | AF10 | PB L968-974; SR L39-40 |
| PA9 | USB VBUS sense | GPIO in | PB L976-980; SR L41-42 |
| PA5 | TLC5955 SPI1_SCK | AF5 | PB L874-885; SR L45-46 |
| PA6 | TLC5955 SPI1_MISO | AF5 | PB L874-885; SR L47-48 |
| PA7 | TLC5955 SPI1_MOSI | AF5 | PB L874-885; SR L49-50 |
| PA15 | TLC5955 LAT | GPIO out | PB L645-646; SR L51-52 |
| PB15 | TLC5955 GSCLK, TIM12_CH2 9.6 MHz | AF9 | PB L524-536, L605-613; SR L53-54 |
| PB12 | Ext. flash /CS | GPIO out | PB L890-896; SR L57-58 |
| PB13 | Ext. flash SPI2_SCK | AF5 | PB L898-904; SR L59-60 |
| PC2 | Ext. flash SPI2_MISO | AF5 | PB L906-908; SR L61-62 |
| PC3 | Ext. flash SPI2_MOSI | AF5 | PB L906-908; SR L63-64 |
| PB10 | IMU I2C2_SCL | AF4 | PB L173-190; SR L69-70 |
| PB3 | IMU I2C2_SDA | AF9 | PB L192-195; SR L67-68 |
| PB4 | IMU INT1 (data ready) | EXTI4 | PB L202-210; SR L71-72 |
| PA4 | Speaker DAC1_OUT1 | analog | PB L675-698; SR L75-76 |
| PC10 | Speaker amplifier enable | GPIO out | PB L676-677; SR L77-78 |
| PA2 | Bluetooth enable (kept LOW) | GPIO out | PB L91-95; SR L81-82 |
| PD5/PD6 | Bluetooth USART2 TX/RX | AF7 | PB L106-123; SR L83-86 |
| PD3/PD4 | Bluetooth USART2 CTS/RTS | AF7 | PB L114-117; SR L87-90 |
| PC9 | Bluetooth 32.768 kHz, TIM8_CH4 | AF3 | PB L538-550, L614-622 |
| PC0 | Battery current, ADC1 ch10 | analog | PB L783-794; SR L93-94 |
| PC1 | Battery voltage, ADC1 ch11 | analog | PB L796-807; SR L95-96 |
| PB0 | Battery NTC, ADC1 ch8 | analog | PB L809-820; SR L97-98 |
| PA3 | USB charge current, ADC1 ch3 | analog | PB L822-833; SR L99-100 |
| PC4 | Ladder 0: center button + charger /CHG, ADC1 ch14 | analog | PB L835-846, L662-666; SR L103-104 |
| PA1 | Ladder 1: left/right/BT buttons, ADC1 ch1 | analog | PB L848-859, L667-670; SR L105-106 |
| PA0 | Charger ISET, TIM5_CH1 96 kHz | AF2 | PB L552-561, L623-631 |

## Motor H-bridges

| Port | Pin 1 (forward PWM) | Pin 2 (reverse PWM) | AF | Source |
|---|---|---|---|---|
| A | PE9 TIM1_CH1 | PE11 TIM1_CH2 | AF1 | PB L433-445; SR L108-164 |
| B | PE13 TIM1_CH3 | PE14 TIM1_CH4 | AF1 | PB L446-458; SR L108-164 |
| C | PB6 TIM4_CH1 | PB7 TIM4_CH2 | AF2 | PB L459-471; SR L108-164 |
| D | PB8 TIM4_CH3 | PB9 TIM4_CH4 | AF2 | PB L472-484; SR L108-164 |
| E | PC6 TIM3_CH1 | PC7 TIM3_CH2 | AF2 | PB L485-497; SR L108-164 |
| F | PC8 TIM3_CH3 | PB1 TIM3_CH4 | AF2 | PB L498-510; SR L108-164 |

PWM: 12 kHz (prescaler 8, period 1000 at 96 MHz), channels inverted
(PB L563-604). Coast = both LOW, brake = both HIGH, forward = pin 1 PWM and
pin 2 HIGH (Pybricks `motor_driver_hbridge_pwm.c`).

## I/O ports (LPF2)

| Port | P5 (ID1) | P6 (ID2) | UART buffer (active LOW) | TX | RX | UART | AF | Source |
|---|---|---|---|---|---|---|---|---|
| A | PD7 | PD8 | PA10 | PE8 | PE7 | UART7 | AF8 | PB L234-255; SR L173-178 |
| B | PD9 | PD10 | PA8 | PD1 | PD0 | UART4 | AF11 | PB L256-273; SR L173-178 |
| C | PD11 | PE4 | PE5 | PE1 | PE0 | UART8 | AF8 | PB L274-291; SR L173-178 |
| D | PC15 | PC14 | PB2 | PC12 | PD2 | UART5 | AF8 | PB L292-309; SR L173-178 |
| E | PC13 | PE12 | PB5 | PE3 | PE2 | UART10 | AF11 | PB L310-327; SR L173-178 |
| F | PC11 | PE6 | PC5 | PD15 | PD14 | UART9 | AF11 | PB L328-345; SR L173-178 |

## TLC5955 channels

| Function | Channels | Source |
|----------|----------|--------|
| Center button LED top (R, G, B) | 5, 4, 3 | PB L367-378; SR L192 |
| Center button LED bottom (R, G, B) | 8, 7, 6 | PB L379-389; SR L193 |
| Battery LED (R, G, B) | 2, 1, 0 | PB L390-400; SR L194 |
| Bluetooth LED (R, G, B) | 20, 19, 18 | PB L401-411; SR L195 |
| Charger MODE pin (0 = charge, 0xFFFF = off) | 14 | PB L149-151; SR L217 |

5×5 matrix, row by row (PB L414-427; SR L183-189):

```text
38 36 41 46 33
37 28 39 47 21
24 29 31 45 23
26 27 32 34 22
25 40 30 35  9
```

Which corner is "top left" relative to the hub housing is still to be
verified on hardware.
