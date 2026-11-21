# Badge 2026 firmware

Rust firmware (esp-idf-svc, ESP-IDF v6.0.3) for the TENGUCON 3.0 badge: LoRa chat, IR blaster,
SD card file explorer, ELF app loader, LED blink styles, nametag (name, company, where you are from), sponsors, about.

Hardware map: [HARDWARE.md](HARDWARE.md). App SDK: [sdk/README.md](sdk/README.md).

## Flash the release build

Many badges, Linux or macOS: `./flash_badges.sh --full` (new badges, wipes saved data) or
`./flash_badges.sh --update` (badges in use, keeps saved data). It installs esptool on first run,
then flashes every badge you plug in, one after another. It only needs this script and
`release/badge2026.bin`.

By hand, you only need `esptool` (`pip install esptool`, or `uv run esptool` from this folder).

1. Plug the badge in over USB-C. It shows up as `/dev/ttyACM0` (Linux) or a COM port (Windows).
2. If it does not show up: hold BOOT, tap RESET, release BOOT.
3. Flash:

   ```
   esptool --chip esp32s3 -p /dev/ttyACM0 write-flash 0x0 release/badge2026.bin
   ```

4. Tap RESET.

The full image also wipes NVS (0x9000-0xEFFF): hunt progress, saved hunt unlocks, chat settings
and the transmit unlock. To update a badge that is already in use, write only the app. From this
folder, `make flash-app` does it, or by hand:

```
tail -c +65537 release/badge2026.bin > app.bin
esptool --chip esp32s3 -p /dev/ttyACM0 write-flash 0x10000 app.bin
```

This only works while the bootloader and partition table stay the same. If they change, flash the
full image.

## LoRa transmit

Receiving works out of the box. Sending is locked: the first time you press OK in **Chat**, the
badge shows a disclaimer and asks for the transmit password. You transmit at your own risk.

- Default password: `tengu2026`
- Change it in **Chat > TAB (settings) > TX password > OK** (transmit must be unlocked first,
  4 characters minimum).
- The badge remembers the unlock after a reboot.

The radio is set up for Japan's 920 MHz band (920.6-923.4 MHz, max 13 dBm, listen-before-talk,
under 4 s per packet, 6 minutes of transmit per hour, counted in RAM so a reboot resets it). See [HARDWARE.md](HARDWARE.md#lora-u7-ra-01sh-sx1262-rated-903-927-mhz).

## Scavenger hunt (organizers only)

Players type `tengu` on the main menu to open the hidden Hunt app. It only says "Enter code:".
Each code decrypts one prompt. The serial line, the Morse LED, the NVS flag and the crackme ELF in
NVS are on the badge from boot. Typing the Wi-Fi code unlocks the Badge swap code, and typing the
swap code unlocks the Myojin remote. The badge saves both codes in NVS `hunt/unlocked`, so a reboot
doesn't undo them. Desk hints: `hunt/HINTS.md`.

- Codes and prompts: `hunt/hunt.toml`. Use strong codes (random strings or 3-4 random words);
  anything short can be brute-forced from a flash dump. The crackme code is the WPA2 password
  of the hunt network (8-63 chars, lowercase).
- `make hunt` builds the crackme with its secret, then encrypts everything into `rs/hunt.bin`.
  `cargo build` runs it by itself when `rs/hunt.bin` is missing or older than its inputs.
  `make rs-build` and `make release` run it for you.
- Wi-Fi stage: run a hidden access point named TENGU-HUNT with the crackme code as password,
  no internet. A device on it sends the broadcast from `hunt/HINTS.md` to UDP port 31337 every
  few seconds; badges show it in the Wi-Fi app.
  `hunt/ap` is that box for any plain ESP32: `cd hunt/ap && idf.py set-target esp32 && idf.py flash`.
  Tune `TX_POWER_QUARTER_DBM` in `hunt/ap/main/main.c` on the con floor so the dBm hunt has a slope.
- The last answer (Manseibashi) opens a finish screen with the board name and a code. This is the fixed
  badge ID from the MAC, not the name set in Nametag. Check it with
  `python3 hunt/verify_finish.py <board name> <code>`, look at the physical badge, and ask a question
  about the Manseibashi visit, and ask them how they solved every challenge: the code alone is
  not proof. Write down the time; the first 3 finished badges get a black badge.
- Keep 5 badges at the desk on the Badge swap screen so the first teams are not stuck.
- Print the sticker for the broadcast box (text in `hunt/HINTS.md`).

## SD card

- The card must be FAT32. If it is not (most cards of 64 GB and more come as exFAT), open
  **Files** on the badge and accept the format prompt. This erases the card.
- Copy Flipper Zero `.ir` files anywhere on the card (up to 3 folders deep), then use
  **IR blaster > Load from SD**. Example files: `tools/flipper/`.
- Copy `release/hello.elf` to the card and open it from **Files** to run it.
- **Phrack** always lists the built-in issue (`rs/phrack/*.tar.gz`, one file only; swap it to ship a
  newer issue). For other issues, copy archives from https://phrack.org/archives/tgz/ to the card root
  or `/phrack`. The first open extracts the archive to `/phrack/.cache/` (a few
  seconds). Reader keys: UP/DOWN scroll, SHIFT (caps lock) then UP/DOWN pages, LEFT/RIGHT pan, B back.

## Build from source

Needs: asdf (`.tool-versions`), uv, ESP-IDF v6.0.3 in `~/esp/esp-idf`, the Rust Xtensa toolchain
(`cargo install --locked espup ldproxy espflash@4.3.0`, then `espup install --targets esp32s3 --std`;
newer espflash needs a newer rustc than the one pinned in `.tool-versions`).

```
make rs-build      # firmware
make rs-flash      # flash it and open the serial log
make rs-monitor    # serial log with decoded backtraces
make app           # build apps/hello (APP=<name> for others)
make release       # firmware + hello app into release/
make flash-app     # write only the app from release/, keep NVS
make dump          # back up the whole flash to dumps/
```
