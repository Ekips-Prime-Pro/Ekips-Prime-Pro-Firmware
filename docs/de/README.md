# Ekips Prime Pro Firmware – Überblick

Eigene Firmware für den LEGO® Education SPIKE™ Prime Hub (STM32F413VGT6),
geschrieben in `no_std`-Rust mit Embassy. Sie wird Ekips-Prime-Pro-Programme
über das `Hardware`-Trait aus `ekips-hub-api` ausführen.

> **Nicht auf Hardware getestet.** Während der Entwicklung stand kein Hub zur
> Verfügung. Die Firmware kompiliert, wurde aber noch nie geflasht oder
> ausgeführt.

## Inhalt

- [Entscheidungen](decisions.md) – warum die Dinge so sind, wie sie sind.
- [Bauen](building.md)
- [Speicherbelegung](memory-map.md)
- [Pinbelegung](pinout.md)
- [Treiber](drivers.md)
- [Debugging](debugging.md)

## Grundregeln

- Der LEGO-DFU-Bootloader (`0x08000000–0x08007FFF`) wird nie angefasst.
  Die Firmware beginnt bei `0x08008000`.
- Nur USB-Seriell (CDC-ACM), Bluetooth bleibt aus.
