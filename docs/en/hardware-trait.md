# `Hardware` trait implementation

`src/hub.rs` implements `ekips_hub_api::Hardware` (core rev
`81a4bd574b93502b0347d641d12bb7e7b2519890`) as `SpikeHub`.

> **Skeleton, untested on hardware.** The VM is not integrated yet;
> `main` only creates a `SpikeHub` and prints "hub ready".

| Method | Implementation | Notes |
|--------|----------------|-------|
| `now_ms` | `embassy_time::Instant` since `SpikeHub::new()` | |
| `check_device` | LUMP type ID → device class | motors: SPIKE S/M/L, Technic M/L angular |
| `motor_run_angle` | `motor_control::run_by` | P controller, duty limit from speed; negative speed reverses |
| `motor_run` | open-loop duty | speed % → duty ‰ |
| `motor_stop` | hold current angle | brakes if no encoder data |
| `motor_done` | move reached ±3° | |
| `motor_angle` | LUMP mode 2 (int32 °) minus offset | |
| `motor_speed` | estimated from the angle | 100 % = 1000 °/s (approximation) |
| `motor_reset_angle` | offset | running moves stay consistent |
| `drive_configure` | stores the geometry, checks both motors | |
| `drive_straight` / `drive_turn` | both wheels `run_by` | left wheel mirrored (`LEFT_WHEEL_SIGN = -1`) |
| `drive_done` / `drive_stop` | both wheels | |
| `heading` / `reset_heading` | IMU z integration | clockwise positive, sign unverified |
| `color` | color sensor mode 0 | LEGO color numbers → `Color` |
| `reflection` | color sensor mode 1 | requests a mode switch if needed |
| `distance_mm` | ultrasonic mode 0 | `-1` → `None` |
| `pressed` / `force_n` | force sensor mode 0 | scaling 1 unit = 0.1 N **unverified** |
| `show` | `matrix::show_text` | scrolls if longer than one character |
| `beep` | `speaker::beep` | non-blocking |
| `battery_pct` | `battery::state().percent` | voltage-based estimate |
| `print` | `usb::write` + CRLF | dropped if the host does not read |

## Sensor modes

A sensor reports one mode at a time. If a method needs another mode than
the current one, it asks the device to switch and returns a neutral value
(`Color::None`, `0.0`, `None`) until the new data arrives (typically
30–50 ms). Programs that poll in a loop therefore see the right value
after a short delay.
