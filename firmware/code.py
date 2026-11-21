# code.py - CircuitPython IR "TV power on" sender for ESP32-S3, with on-screen status
#
# Sends the Flipper Zero code  NECext  A: 0x4040  C: 0xF50A
# out of IR-OUT on GPIO44 and shows what it sent on the ILI9341 screen.
#
# NECext on the wire (32 bits, each byte sent LSB-first):
#   addr_lo 0x40, addr_hi 0x40, cmd_lo 0x0A, cmd_hi 0xF5
# Timing (microseconds): 9000 mark / 4500 space header,
#   bit 0 = 560 mark + 560 space, bit 1 = 560 mark + 1690 space,
#   then a final 560 us stop mark.  Carrier: 38 kHz, ~33% duty.

import array
import time

import board
import busio
import digitalio
import displayio
import pulseio

# ---- Pins (pinouts.txt) ---------------------------------------------------
PIN_SPI_CLK = board.IO41
PIN_SPI_MOSI = board.IO39
PIN_LCD_DC = board.IO1          # WRX
PIN_LCD_CS = board.IO2          # CS2
PIN_LORA_CS = board.IO38        # CS1, held high so the LoRa chip stays off the shared bus
IR_OUT_PIN = board.IO44         # IR-OUT

# ---- IR config ------------------------------------------------------------
ADDRESS = 0x4040        # Flipper "A:"
COMMAND = 0xF50A        # Flipper "C:"

CARRIER_HZ = 38000
DUTY = int(65535 * 0.33)   # same duty cycle the Flipper uses

REPEAT_FRAMES = 2       # NEC "button held" frames after the main frame (0 = none)

# ---- NEC timings (us) -----------------------------------------------------
HDR_MARK = 9000
HDR_SPACE = 4500
BIT_MARK = 560
ONE_SPACE = 1690
ZERO_SPACE = 560
REPEAT_SPACE = 2250
FRAME_PERIOD = 108000   # NEC frames start every ~108 ms

# ===========================================================================
# DISPLAY (ILI9341 driver taken from your script)
# ===========================================================================
WIDTH = 320
HEIGHT = 240
MADCTL = 0xE8
SPI_BAUDRATE = 20_000_000

FONT = {
    " ": (0x00, 0x00, 0x00, 0x00, 0x00),
    "0": (0x3E, 0x51, 0x49, 0x45, 0x3E),
    "1": (0x00, 0x42, 0x7F, 0x40, 0x00),
    "2": (0x42, 0x61, 0x51, 0x49, 0x46),
    "3": (0x21, 0x41, 0x45, 0x4B, 0x31),
    "4": (0x18, 0x14, 0x12, 0x7F, 0x10),
    "5": (0x27, 0x45, 0x45, 0x45, 0x39),
    "6": (0x3C, 0x4A, 0x49, 0x49, 0x30),
    "7": (0x01, 0x71, 0x09, 0x05, 0x03),
    "8": (0x36, 0x49, 0x49, 0x49, 0x36),
    "9": (0x06, 0x49, 0x49, 0x29, 0x1E),
    "A": (0x7E, 0x11, 0x11, 0x11, 0x7E),
    "B": (0x7F, 0x49, 0x49, 0x49, 0x36),
    "C": (0x3E, 0x41, 0x41, 0x41, 0x22),
    "D": (0x7F, 0x41, 0x41, 0x22, 0x1C),
    "E": (0x7F, 0x49, 0x49, 0x49, 0x41),
    "F": (0x7F, 0x09, 0x09, 0x09, 0x01),
    "G": (0x3E, 0x41, 0x49, 0x49, 0x7A),
    "H": (0x7F, 0x08, 0x08, 0x08, 0x7F),
    "I": (0x00, 0x41, 0x7F, 0x41, 0x00),
    "J": (0x20, 0x40, 0x41, 0x3F, 0x01),
    "K": (0x7F, 0x08, 0x14, 0x22, 0x41),
    "L": (0x7F, 0x40, 0x40, 0x40, 0x40),
    "M": (0x7F, 0x02, 0x0C, 0x02, 0x7F),
    "N": (0x7F, 0x04, 0x08, 0x10, 0x7F),
    "O": (0x3E, 0x41, 0x41, 0x41, 0x3E),
    "P": (0x7F, 0x09, 0x09, 0x09, 0x06),
    "Q": (0x3E, 0x41, 0x51, 0x21, 0x5E),
    "R": (0x7F, 0x09, 0x19, 0x29, 0x46),
    "S": (0x46, 0x49, 0x49, 0x49, 0x31),
    "T": (0x01, 0x01, 0x7F, 0x01, 0x01),
    "U": (0x3F, 0x40, 0x40, 0x40, 0x3F),
    "V": (0x1F, 0x20, 0x40, 0x20, 0x1F),
    "W": (0x3F, 0x40, 0x38, 0x40, 0x3F),
    "X": (0x63, 0x14, 0x08, 0x14, 0x63),
    "Y": (0x07, 0x08, 0x70, 0x08, 0x07),
    "Z": (0x61, 0x51, 0x49, 0x45, 0x43),
    ":": (0x00, 0x36, 0x36, 0x00, 0x00),
    "-": (0x08, 0x08, 0x08, 0x08, 0x08),
    ".": (0x00, 0x60, 0x60, 0x00, 0x00),
    "_": (0x40, 0x40, 0x40, 0x40, 0x40),
    "=": (0x14, 0x14, 0x14, 0x14, 0x14),
    "/": (0x20, 0x10, 0x08, 0x04, 0x02),
    "(": (0x00, 0x1C, 0x22, 0x41, 0x00),
    ")": (0x00, 0x41, 0x22, 0x1C, 0x00),
    ",": (0x00, 0x50, 0x30, 0x00, 0x00),
    "?": (0x02, 0x01, 0x51, 0x09, 0x06),
}

cs = dc = spi = lora_cs = None


def setup_display():
    global cs, dc, spi, lora_cs
    displayio.release_displays()
    lora_cs = digitalio.DigitalInOut(PIN_LORA_CS)
    lora_cs.direction = digitalio.Direction.OUTPUT
    lora_cs.value = True
    cs = digitalio.DigitalInOut(PIN_LCD_CS)
    cs.direction = digitalio.Direction.OUTPUT
    cs.value = True
    dc = digitalio.DigitalInOut(PIN_LCD_DC)
    dc.direction = digitalio.Direction.OUTPUT
    dc.value = True
    spi = busio.SPI(clock=PIN_SPI_CLK, MOSI=PIN_SPI_MOSI)
    while not spi.try_lock():
        pass
    spi.configure(baudrate=SPI_BAUDRATE, polarity=0, phase=0)


def write_cmd(c, data=b""):
    cs.value = False
    dc.value = False
    spi.write(bytes([c]))
    if data:
        dc.value = True
        spi.write(bytes(data))
    cs.value = True


def set_window(x0, y0, x1, y1):
    write_cmd(0x2A, bytes([x0 >> 8, x0 & 0xFF, x1 >> 8, x1 & 0xFF]))
    write_cmd(0x2B, bytes([y0 >> 8, y0 & 0xFF, y1 >> 8, y1 & 0xFF]))


def rgb565(r, g, b):
    c = ((r & 0xF8) << 8) | ((g & 0xFC) << 3) | (b >> 3)
    return c >> 8, c & 0xFF


WHITE = bytes(rgb565(255, 255, 255))
YELLOW = bytes(rgb565(255, 255, 0))
GREEN = bytes(rgb565(0, 255, 0))
RED = bytes(rgb565(255, 60, 60))
GREY = bytes(rgb565(150, 150, 150))
CYAN = bytes(rgb565(0, 220, 255))

MAX_H = 30
line_buf = bytearray(WIDTH * MAX_H * 2)
line_mv = memoryview(line_buf)
ZERO_ROW = bytes(WIDTH * 2)
_line_cache = {}


def clear_screen():
    _line_cache.clear()
    set_window(0, 0, WIDTH - 1, HEIGHT - 1)
    cs.value = False
    dc.value = False
    spi.write(b"\x2C")
    dc.value = True
    for _ in range(HEIGHT):
        spi.write(ZERO_ROW)
    cs.value = True


def init_display():
    write_cmd(0x01)
    time.sleep(0.15)
    write_cmd(0x28)
    write_cmd(0xCF, b"\x00\xC1\x30")
    write_cmd(0xED, b"\x64\x03\x12\x81")
    write_cmd(0xE8, b"\x85\x00\x78")
    write_cmd(0xCB, b"\x39\x2C\x00\x34\x02")
    write_cmd(0xF7, b"\x20")
    write_cmd(0xEA, b"\x00\x00")
    write_cmd(0xC0, b"\x23")
    write_cmd(0xC1, b"\x10")
    write_cmd(0xC5, b"\x3E\x28")
    write_cmd(0xC7, b"\x86")
    write_cmd(0x36, bytes([MADCTL]))
    write_cmd(0x3A, b"\x55")
    write_cmd(0xB1, b"\x00\x18")
    write_cmd(0xB6, b"\x08\x82\x27")
    write_cmd(0xF2, b"\x00")
    write_cmd(0x26, b"\x01")
    write_cmd(0xE0, b"\x0F\x31\x2B\x0C\x0E\x08\x4E\xF1\x37\x07\x10\x03\x0E\x09\x00")
    write_cmd(0xE1, b"\x00\x0E\x14\x03\x11\x07\x31\xC1\x48\x08\x0F\x0C\x31\x36\x0F")
    write_cmd(0x11)
    time.sleep(0.15)
    clear_screen()
    write_cmd(0x29)
    time.sleep(0.05)


def draw_text(x, y, text, color, scale, h):
    block = color * scale
    for ch in text:
        glyph = FONT.get(ch.upper(), FONT["?"])
        for col in range(5):
            bits = glyph[col]
            if not bits:
                continue
            px = x + col * scale
            if px + scale > WIDTH:
                continue
            for row in range(7):
                if (bits >> row) & 1:
                    for dy in range(scale):
                        yy = y + row * scale + dy
                        if 0 <= yy < h:
                            off = (yy * WIDTH + px) * 2
                            line_buf[off:off + 2 * scale] = block
        x += 6 * scale


def put_line(y, text, color=WHITE, scale=2):
    """Draw one line of text; skips the redraw if nothing changed."""
    if _line_cache.get(y) == (text, color, scale):
        return
    _line_cache[y] = (text, color, scale)
    h = 7 * scale + 4
    stride = WIDTH * 2
    for i in range(h):
        line_buf[i * stride:(i + 1) * stride] = ZERO_ROW
    draw_text(6, 2, text, color, scale, h)
    set_window(0, y, WIDTH - 1, y + h - 1)
    cs.value = False
    dc.value = False
    spi.write(b"\x2C")
    dc.value = True
    for i in range(h):
        spi.write(line_mv[i * stride:(i + 1) * stride])
    cs.value = True


def show_error(e):
    print("ERROR:", type(e).__name__, e)
    try:
        clear_screen()
        put_line(4, "ERROR:", RED, 3)
        put_line(36, type(e).__name__, YELLOW)
        msg = str(e)
        y = 60
        for i in range(0, min(len(msg), 25 * 7), 25):
            put_line(y, msg[i:i + 25], WHITE)
            y += 22
    except Exception:
        pass


# ===========================================================================
# IR
# ===========================================================================
def necext_pulses(address, command):
    """Build mark/space durations for one NECext frame (starts and ends on a mark)."""
    data = (address & 0xFFFF) | ((command & 0xFFFF) << 16)
    pulses = [HDR_MARK, HDR_SPACE]
    for i in range(32):                      # LSB first, same as the Flipper encoder
        pulses.append(BIT_MARK)
        pulses.append(ONE_SPACE if (data >> i) & 1 else ZERO_SPACE)
    pulses.append(BIT_MARK)                  # stop bit
    return array.array("H", pulses)


FRAME = necext_pulses(ADDRESS, COMMAND)
REPEAT = array.array("H", [HDR_MARK, REPEAT_SPACE, BIT_MARK])


def wait_rest_of_period(pulses):
    """Sleep out the remainder of the 108 ms NEC frame period."""
    used = sum(pulses)
    time.sleep(max(0, FRAME_PERIOD - used) / 1_000_000)


def send_power(ir):
    ir.send(FRAME)
    wait_rest_of_period(FRAME)
    for _ in range(REPEAT_FRAMES):
        ir.send(REPEAT)
        wait_rest_of_period(REPEAT)


# ===========================================================================
# MAIN
# ===========================================================================
def main():
    setup_display()
    init_display()

    put_line(4, "TV IR SENDER", YELLOW, 3)
    put_line(40, "NECEXT A:{:04X} C:{:04X}".format(ADDRESS, COMMAND), CYAN)
    put_line(64, "BYTES {:02X} {:02X} {:02X} {:02X}".format(
        ADDRESS & 0xFF, ADDRESS >> 8, COMMAND & 0xFF, COMMAND >> 8), GREY)
    put_line(88, "{} PULSES  {} KHZ".format(len(FRAME), CARRIER_HZ // 1000), GREY)

    ir = pulseio.PulseOut(IR_OUT_PIN, frequency=CARRIER_HZ, duty_cycle=DUTY)
    put_line(120, "IR OUT GPIO44 OK", GREEN)

    put_line(150, "SENDING...", YELLOW)
    print("Sending NECext A=0x%04X C=0x%04X (%d pulses)" % (ADDRESS, COMMAND, len(FRAME)))
    send_power(ir)
    put_line(150, "SENT: 1 FRAME, {} REPEAT".format(REPEAT_FRAMES), GREEN)
    print("Done")

    ir.deinit()

    while True:              # keep the program running so the screen stays as-is
        time.sleep(1)


try:
    main()
except Exception as e:
    show_error(e)
    raise
