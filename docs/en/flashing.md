# Backup, flashing and restoring the LEGO firmware

> ⚠️ **This firmware has never run on a real hub.** It is untested on
> hardware. Flashing it is at your own risk. Nothing in this project has
> been tried on a device yet – including the scripts on this page.

> ⚠️ **Never write to `0x08000000–0x08007FFF`.** That is the LEGO DFU
> bootloader. As long as it is intact, the hub can always be recovered
> over USB. Every command here writes to `0x08008000` only. Never use
> "erase chip", `dfu-util -s 0x08000000` with `-D`, mass erase, or an
> SWD "erase all".

## Why this is (normally) safe

The hub has a LEGO DFU bootloader in flash sectors 0–1 that our firmware
never touches (see [memory-map.md](memory-map.md)). Whatever is flashed
at `0x08008000`, the bootloader still starts when the hub is put into DFU
mode, so a broken firmware can be replaced again. The risks that remain:

- A firmware bug that damages hardware (e.g. motors driven against a
  stop, a wrong charger setting). Our firmware limits the charge current
  to 500 mA and coasts all motors on panic, but this is untested.
- Running the battery flat: the firmware powers off below 6.0 V, also
  untested.
- Interrupting a flash: just enter DFU mode again and repeat.

## Requirements

- `dfu-util` 0.9 or newer (Linux: package `dfu-util`; macOS:
  `brew install dfu-util`; Windows: from dfu-util.sourceforge.net).
- Linux: the udev rule [`scripts/99-lego-spike-dfu.rules`](../../scripts/99-lego-spike-dfu.rules)
  for access without root.
- Windows: the DFU device may need the WinUSB driver (e.g. via Zadig,
  device `0694:0011`). Do not change drivers of other devices.
- A USB cable that carries data. A charged battery.

## Enter DFU mode

(Procedure as documented by the Pybricks project; to be confirmed.)

1. Unplug the USB cable and switch the hub off (hold the center button
   until it turns off).
2. Press and **hold the Bluetooth button** (top right).
3. Plug in the USB cable while holding it.
4. Keep holding until the Bluetooth light flashes in changing colors,
   then release.
5. Check: `dfu-util -l` must list `[0694:0011]`.

The hub leaves DFU mode when unplugged or after flashing with `:leave`.

## 1. Back up the LEGO firmware

```bash
scripts/backup-lego-firmware.sh backup
```

This only **reads** the flash and writes into `backup/`:

- `lego-firmware-<date>.bin` – 992 KiB from `0x08008000` (the part that
  will be replaced; needed for a restore),
- `lego-full-<date>.bin` – the complete 1 MiB including the bootloader
  (for reference only; never write it back),
- `SHA256SUMS-<date>`.

Keep the backup somewhere safe. Note: the official firmware is also
available through the LEGO apps, so the backup is a second line of
defense.

## 2. Build and flash Ekips Prime Pro

```bash
cargo xtask bin
```

```bash
scripts/flash-firmware.sh target/firmware.bin
```

The script checks the image size and vector table, warns if there is no
backup, asks for confirmation and then runs exactly:

```bash
dfu-util -d 0694:0011 -a 0 -s 0x08008000:leave -D target/firmware.bin
```

`cargo xtask bin` has already refused any image that would touch the
bootloader range.

## 3. Restore the LEGO firmware

Option A (recommended): connect the hub to the official LEGO Education
SPIKE app; it offers a firmware update/restore when it finds a hub
without LEGO firmware (in DFU mode if needed).

Option B: write your backup back (firmware area only):

```bash
scripts/restore-lego-firmware.sh backup/lego-firmware-<date>.bin
```

The script refuses the `lego-full-*` file because it contains the
bootloader.

## If something goes wrong

| Symptom | What to do |
|---------|------------|
| Hub does not turn on / stays dark after flashing | Enter DFU mode and flash again or restore. With the battery removed and USB plugged in the hub is powered by USB. |
| `dfu-util` finds no device | Check cable, DFU mode, udev rule / WinUSB driver. |
| Flash interrupted | Enter DFU mode again and repeat. |
| Hub turns off immediately on battery | The power latch may not work – please report; use USB power. |
| Hub gets warm while charging | Unplug USB and report. |
