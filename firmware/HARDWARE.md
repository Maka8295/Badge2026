# Badge 2026 hardware map

Source of truth: pad nets in `../badge.kicad_pcb` (not `pinouts.txt`, which has errors, see below).
Pin map in code: `rs/src/board.rs`.

Code tagged `// HARDWARE:` works around board-level limits described below (see "LEDs" and "IR").
Don't remove, reorder or "simplify" it without reading the matching section.

## MCU

BOM says ESP32-S3-WROOM-2 (octal flash), but the fitted part is not. Read from a badge with esptool/espefuse:

- Chip: **ESP32-S3R8**, QFN56, rev v0.2, 240 MHz dual core, 40 MHz crystal.
- PSRAM: **8 MB octal**, inside the chip package (eFuse PSRAM_CAP = 8M, PSRAM_VENDOR = AP_3v3, 85 C).
  Enabled in `rs/sdkconfig.defaults` (`CONFIG_SPIRAM_MODE_OCT`, 80 MHz) and added to the heap, so `malloc`/`Vec`
  can use it. Allocations over 16 KB go to PSRAM first. `CONFIG_SPIRAM_IGNORE_NOTFOUND` lets the firmware boot
  without it. Octal PSRAM uses GPIO33-37, so those pins are not free.
- Flash: **16 MB quad** SPI, external (JEDEC manufacturer 0x46, device 0x4018). FLASH_TYPE eFuse = 4 data lines.
  The firmware is built for 8 MB (`CONFIG_ESPTOOLPY_FLASHSIZE_8MB`, `-s 8mb` in the Makefile), which works on 16 MB.
- VDD_SPI is fixed at 3.3 V by eFuse (VDD_SPI_FORCE = 1, VDD_SPI_TIEH = 1), which matches the 3.3 V PSRAM.
  FLASH_CAP/FLASH_VENDOR eFuses are empty.

No security eFuses burned (no flash encryption, no secure boot, JTAG on).
USB: native USB on GPIO19/20 through USBLC6 ESD. The badge shows up as USB Serial/JTAG (`303a:1001`, `/dev/ttyACM0`).
There is no USB-UART chip. UART0 pins (43/44) are used by IR, so the console must be USB Serial/JTAG.

Buttons: BOOT = SW61 (GPIO0, 10k pull-up), RESET = SW56 (EN). To force download mode: hold BOOT, tap RESET.

Free GPIOs: 45. It is a strapping pin, but VDD_SPI_FORCE is burned, so the chip ignores it for flash voltage. GPIO46 is used by the RXEN bodge. Nothing else is free.

## GPIO table

| GPIO | Net | Goes to |
|------|-----|---------|
| 0  | BOOT    | SW61, 10k pull-up |
| 1  | WRX     | LCD D/C (J6.36) |
| 2  | CS2     | LCD CS (J6.38) |
| 3  | LED     | SK6803 chain DIN (strapping pin, JTAG select) |
| 4  | DAT1    | SD DAT1 |
| 5  | DAT0    | SD DAT0 |
| 6  | SDCLK   | SD CLK (via R19 47R series) |
| 7  | CMD     | SD CMD |
| 8  | S2      | row mux U3 S2 |
| 9-11  | S3-S5 | col mux U5 S0-S2 |
| 12-14 | S6-S8 | col mux U6 S0-S2 |
| 15 | DAT3    | SD DAT3/CD |
| 16 | DAT2    | SD DAT2 |
| 17 | S0      | row mux U3 S0 |
| 18 | S1      | row mux U3 S1 |
| 19/20 | D-/D+ | USB-C |
| 21 | ROWOUT  | row mux U3 common |
| 38 | CS1     | LoRa NSS |
| 39 | MOSI    | LCD SDA (J6.34) + LoRa MOSI |
| 40 | MISO    | LoRa MISO |
| 41 | SPICLK  | LCD SCL (J6.37) + LoRa SCK |
| 42 | IRO     | LoRa module pin 5 (TXEN, tied to SX1262 DIO2), input only |
| 43 | IR-IN   | TSOP38238 OUT (J4.3) |
| 44 | IR-OUT  | R23 1k -> Q1 SS8050 base -> D67+D68 940nm IR LEDs |
| 33-37 | - | octal PSRAM inside the chip, not usable |
| 45 | -   | not connected |
| 46 | -   | bodge wire, U8 pad 16 to LoRa RXEN (U7 pin 11), not on the PCB |

EN (chip reset) also drives LCD RESET (J6.10) and LoRa RESET. Neither can be reset alone.

## Keyboard

3x 74HC4051, enable tied low. Mux address 0 (A0) is not connected, so address N (1..7) picks ROWn / COLn.
COL8..14 are U6 address 1..7. Diodes: COL -> switch -> diode anode, cathode -> ROW.

Scan: set ROWOUT output low, COLOUT1 and COLOUT2 input with pull-up. For each row address 1..7 and col address 1..7,
read both COLOUT pins (one read covers COLn and COLn+7). Pressed = 0. One key at a time, so no ghosting.

Main block: 4 rows x 14 cols, top of the board. SW numbers below (x grows left to right):

| | C1 | C2 | C3 | C4 | C5 | C6 | C7 | C8 | C9 | C10 | C11 | C12 | C13 | C14 |
|--|--|--|--|--|--|--|--|--|--|--|--|--|--|--|
| R1 | SW1 | SW8 | SW15 | SW19 | SW23 | SW27 | SW31 | SW35 | SW40 | SW44 | SW48 | SW52 | SW57 | SW62 |
| R2 | SW2 | SW9 | SW16 | SW20 | SW24 | SW28 | SW32 | SW36 | SW41 | SW45 | SW49 | SW53 | SW58 | SW63 |
| R3 | SW3 | SW10 | SW17 | SW21 | SW25 | SW29 | SW33 | SW37 | SW42 | SW46 | SW50 | SW54 | SW59 | SW64 |
| R4 | SW4 | SW11 | SW18 | SW22 | SW26 | SW30 | SW34 | SW38 | SW43 | SW47 | SW51 | SW55 | SW60 | SW65 |

Big 12x12 buttons (by position, game-pad layout; R5-R7 x C1-C2 confirmed on hardware):

| Button | SW | Row | Col |
|--|--|--|--|
| Up    | SW6  | R6 | C1 |
| Down  | SW5  | R5 | C1 |
| Left  | SW12 | R5 | C2 |
| Right | SW7  | R7 | C1 |
| A (right, upper) | SW13 | R6 | C2 |
| B (right, lower) | SW14 | R7 | C2 |

SHIFT (R3 C2) works as caps lock in the firmware: each press toggles shifted legends (letters and symbols),
and the footer shows "CAPS" while it is on. ELF programs still see the raw key state through `badge.h`.

62 matrix keys total. SW39 = power slide switch, SW56 = RESET, SW61 = BOOT.

## LEDs

10x SK6803MINI-E on the "5V" rail, data from GPIO3. Chain order (index: part):
0 D69, 1 D72, 2 D75, 3 D70, 4 D73, 5 D76, 6 D71, 7 D74, 8 D77 (left edge, bottom to top), 9 D78 (top right, last in the chain).
D78 is blue for LoRa and red for IR: four rapid pulses on transmit, two 1 s breaths on receive. The LED thread keeps these running while the main loop is busy. Each LED passes data to the next, so one bad joint darkens every LED after it.

Roles in firmware (`rs/src/leds/`):
- Index 0-7 (left edge strip, bottom to top): LED theme from the LEDs app (default "Cyberpunk"). Rendered in the LED
  thread at 50 frames/s with gamma 2.2 on each colour's brightness (not per channel, which shifts hues). No temporal
  dithering: at this frame rate it flickers on dim channels.
- Index 8 (D77, top of the left edge): hunt Morse LED. White, scaled by the brightness level only. Themes, effects and
  the current limiter never touch it.
- Index 9 (D78): LoRa/IR activity, as above.

Current: SK6803MINI-E is 5 mA per colour channel plus about 1 mA static, so the chain is about 160 mA at full white.
`leds/power.rs` estimates every frame and scales the strip down to keep all 10 LEDs under 90 mA. The LEDs app
shows the worst case and a red warning above brightness 128/255.

Other LEDs, not on GPIO: D15 power (red), D44 charger PG, D45 STAT1, D46 STAT2.

**Data level is out of spec.** D69's DIN gets a 3.3 V high from GPIO3. The SK6803MINI-E datasheet (Rev B/0)
only guarantees a "1" at VIH >= 0.7 x VDD: that is 3.47 V on the 4.95 V rail, and about 3.6 V on USB (see Power).
It works on the boards we have. A misread bit shows up as wrong colours, so `leds.rs` re-sends the frame every
500 ms (`REFRESH_INTERVAL`). Keep that refresh. The hardware fix for a later revision is a 5 V buffer with
TTL-level inputs (e.g. 74AHCT1G125) on DIN.

## IR (J4 TSOP38238 socket, Q1 + D67/D68)

- J4 pinout: 1 (square pad) = 3V3, 2 = GND, 3 = IR-IN (GPIO43). The TSOP38238 is 1 = OUT, 2 = GND, 3 = VS,
  so it goes in **with the lens facing the top edge of the board**. Reversed, its output sits on 3V3:
  receive does not work and the part can be damaged.
- GPIO43/44 are UART0 TX/RX at boot. The ROM drives GPIO43 as TX (boot log), so it fights the TSOP output
  (5 mA abs max) if IR arrives before the firmware takes the pin. GPIO44 has a weak pull-up at reset, which
  half-turns-on Q1, so D67/D68 glow.
- Firmware workaround, keep it:
  - `ir::Ir::new()` is the first thing `main()` sets up.
  - It forces GPIO43 to input-only, because the RMT RX driver does not turn off the output the ROM left on.
  - The IR TX channel holds GPIO44 low from then on.

  Don't move IR setup later in `main()`.
- The ROM/bootloader stage before the app starts is still exposed. The hardware fix for a later revision is
  1k in series with IR-IN at the ESP32, and 4.7k from Q1 base to GND.

## Display (J6, 50-pin FPC)

BuyDisplay 2.4" 240x320 SPI TFT with touch (link in schematic). ILI9341, confirmed on hardware
(mipidsi `ILI9341Rgb565`, BGR). The panel is mounted mirrored: use a horizontal flip.
4-wire SPI write-only: SDA=MOSI, SCL=GPIO41, CS=GPIO2, D/C=GPIO1, RESET=EN.
Backlight: J6.1 to "5V", J6.2-5 to GND through 91R each. Always on, no PWM.
Touch I2C (J6.44 SCL, J6.45 SDA, J6.46 INT) is **not connected** to the ESP32. Touch cannot work.

## LoRa (U7, Ra-01SH, SX1262, rated 903-927 MHz)

Fitted with an Ra-01SH for the Japan 920 MHz band (earlier boards had the 433 MHz Ra-01S, same pinout).
The PCB uses the Ra-01 (SX1278) footprint and pin names. The Ra-01SH pinout is different
(Ra-01SH Specification V1.1):

| Pin | Ra-01 name (PCB) | Ra-01SH real | Connected to |
|--|--|--|--|
| 4  | RESET | RESET | EN |
| 5  | DIO0  | **TXEN** | GPIO42 (input only, see below) |
| 6  | DIO1  | **DIO1** | nothing |
| 10 | DIO4  | **BUSY** | nothing |
| 11 | DIO5  | **RXEN** | GPIO46 (bodge wire) |
| 12-15 | SCK/MISO/MOSI/NSS | same | 41/40/39/38 |
| 1  | ANT   | ANT | nothing on the PCB; antenna soldered to the pin |

Module internals, from the schematic in the Ra-01SH spec:
- 32 MHz crystal, no TCXO, so no `SetDIO3AsTCXOCtrl`. 15 uH inductor on DCC_SW, so DC-DC regulator mode is correct.
- **Pin 5 (TXEN) is tied to the SX1262's DIO2 through R8 (0R).** DIO2 is an active output in every awake mode
  (SX1262 table 8-3). Never drive GPIO42 as an output: it fights the chip. The firmware keeps it as an input
  and uses `SetDIO2AsRfSwitchCtrl`, so the radio switches the antenna itself. Each packet logs the line's
  level; "LOW" means R8 is not fitted on that module.
- R7 (1M) pulls the switch to receive by default and Q1 pulls it to transmit, so receive does not depend on
  RXEN. RXEN only reaches the switch through R1 ("NC/10K", may not be fitted). The GPIO46 bodge is harmless:
  firmware drives it high to receive and low to transmit. Module pin 5/DIO2 can be checked with a meter.
- GPIO46 is a strapping pin. If download mode stops working (GPIO0 low needs GPIO46 low), check this wire.

Effects of the PCB wiring:
- No BUSY: fixed 1 ms wait after each command (longest mode change is 126 us, table 8-2), 50 ms after calibration.
- No DIO1: no IRQ, poll `GetIrqStatus` over SPI.

Firmware radio limits (ARIB STD-T108 v1.5, Part 2, 20 mW), enforced in `rs/src/sx126x`:
- Channels 24-38 only: 920.6-923.4 MHz on the 200 kHz grid. Above that, packets are capped below 400 ms.
- 62.5 or 125 kHz bandwidth, -9 to +13 dBm set on the chip.
- Carrier sense for 6 ms before every packet, measured with a 250 kHz filter to cover the whole channel.
  Busy at -83 dBm or above (limit is -80 dBm at the antenna), random backoff, give up after 10 tries.
- Under 3.9 s on air per packet, with the radio's own timer as a backstop at 3.95 s. 50 ms rest between packets.
- 360 s of transmit in any hour, tracked in RAM, so a reboot resets it. Part 2 does not require this on
  channels 24-38 with 5 ms carrier sense, Part 1 does on 33-38; it is applied to all as a margin.
- Saved settings outside these limits are replaced by defaults: 923.2 MHz, SF9, 125 kHz.

Not covered by firmware: actual antenna power and EIRP are unmeasured, the Ra-01SH spec lists no certification,
and antenna gain must stay at or below 3 dBi.

## SD card (J3)

SDMMC 4-bit, 10k pull-ups on CMD/DAT0-3, 47R on CLK. No card-detect GPIO (DAT3/CD only).

Supported in firmware (`src/sd.rs`): FAT mounted at `/sdcard`, long file names on. With no card-detect pin, the firmware tries to mount when an app needs the card and retries on request.

## Power

- USB-C: 5.1k on CC1/CC2 (sink), VBUS through 600R ferrite.
- MCP73871 charger/power-path (U4): PROG1 4.7k = ~210 mA charge, PROG2 tied to VBUS (500 mA USB mode), PROG3 10k = ~100 mA termination, safety timer disabled (TE high).
  **THERM (pin 5) is floating.** The datasheet says to tie it to a 10k resistor (or 10k NTC) to GND when not used. Floating THERM can stop charging. Check: plug in a battery and USB and look at STAT LEDs D45/D46.
- SW39 switches charger OUT -> VSWITCH.
- TPS63020 buck-boost -> 3V3 (R3 1M / R2 180k, ~3.28 V).
- TPS61023 boost -> "5V" (R10 732k / R29 100k with VREF 0.595 V = ~4.95 V). On USB the input is above the
  setpoint, so the boost goes into pass-through and the rail follows VBUS (about 5.0-5.15 V).
  Feeds LEDs, LCD backlight, power LED, IR LEDs.
- Battery voltage is not routed to any ADC. Charger status LEDs are not routed to GPIO. Firmware cannot see battery state.

## pinouts.txt errors

- WRX (GPIO1) goes to the LCD only, not to LoRa.
- MISO (GPIO40) is missing; it goes to LoRa.
- GPIO42 is LoRa TXEN, not a general IO pin of the module.
- Row/col mux address 0 is unused (rows/cols start at address 1).

## Firmware gotchas

- ESP-IDF v6 uses picolibc. Rust must be built with `--cfg espidf_picolibc` (set in `rs/.cargo/config.toml`),
  otherwise the `libc` crate uses newlib flag values and `File::create` opens files in append mode.
- Writing to the SD card straight from data in flash (`const` / `include_bytes!`) can write zeros when the
  buffer is 4-byte aligned: the SDMMC DMA reads the flash address directly. Copy such data to RAM first.
- The FAT layer has no exFAT support. Cards must be FAT32; the Files app can format them.
