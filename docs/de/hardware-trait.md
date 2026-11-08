# Implementierung des `Hardware`-Traits

`src/hub.rs` implementiert `ekips_hub_api::Hardware` (Core-Rev
`81a4bd574b93502b0347d641d12bb7e7b2519890`) als `SpikeHub`.

> **Skelett, nicht auf Hardware getestet.** Die VM ist noch nicht
> eingebunden; `main` erzeugt nur einen `SpikeHub` und gibt „hub ready“
> aus.

| Methode | Umsetzung | Hinweise |
|---------|-----------|----------|
| `now_ms` | `embassy_time::Instant` seit `SpikeHub::new()` | |
| `check_device` | LUMP-Typ-ID → Geräteklasse | Motoren: SPIKE S/M/L, Technic M/L Winkelmotor |
| `motor_run_angle` | `motor_control::run_by` | P-Regler, Duty-Limit aus der Geschwindigkeit; negative Geschwindigkeit kehrt um |
| `motor_run` | ungeregelter Duty | Geschwindigkeit % → Duty ‰ |
| `motor_stop` | aktuellen Winkel halten | bremst ohne Encoder-Daten |
| `motor_done` | Ziel auf ±3° erreicht | |
| `motor_angle` | LUMP-Modus 2 (int32 °) minus Offset | |
| `motor_speed` | aus dem Winkel geschätzt | 100 % = 1000 °/s (Näherung) |
| `motor_reset_angle` | Offset | laufende Bewegungen bleiben stimmig |
| `drive_configure` | speichert die Geometrie, prüft beide Motoren | |
| `drive_straight` / `drive_turn` | beide Räder `run_by` | linkes Rad gespiegelt (`LEFT_WHEEL_SIGN = -1`) |
| `drive_done` / `drive_stop` | beide Räder | |
| `heading` / `reset_heading` | Integration der IMU-z-Achse | im Uhrzeigersinn positiv, Vorzeichen ungeprüft |
| `color` | Farbsensor Modus 0 | LEGO-Farbnummern → `Color` |
| `reflection` | Farbsensor Modus 1 | fordert bei Bedarf einen Moduswechsel an |
| `distance_mm` | Ultraschall Modus 0 | `-1` → `None` |
| `pressed` / `force_n` | Kraftsensor Modus 0 | Skalierung 1 Einheit = 0,1 N **ungeprüft** |
| `show` | `matrix::show_text` | läuft durch, wenn länger als ein Zeichen |
| `beep` | `speaker::beep` | blockiert nicht |
| `battery_pct` | `battery::state().percent` | Schätzung aus der Spannung |
| `print` | `usb::write` + CRLF | wird verworfen, wenn der Host nicht liest |

## Sensormodi

Ein Sensor liefert immer nur einen Modus. Braucht eine Methode einen
anderen Modus als den aktuellen, fordert sie den Wechsel an und gibt bis
zum Eintreffen der neuen Daten (typisch 30–50 ms) einen neutralen Wert
zurück (`Color::None`, `0.0`, `None`). Programme, die in einer Schleife
abfragen, sehen den richtigen Wert daher nach kurzer Verzögerung.
