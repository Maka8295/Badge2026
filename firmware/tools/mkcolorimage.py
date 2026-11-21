import sys

from PIL import Image

USAGE = "usage: uv run tools/mkcolorimage.py <image> <max_width> <max_height> <out.raw> [--keep-dark]"
BACKGROUND = (0x09, 0x09, 0x09)
TRANSPARENT = 0xF81F
TRANSPARENT_NUDGED = 0xF83F
MIN_ALPHA = 32
GREY_MAX_CHROMA = 40
DARK_MAX_LIGHTNESS = 110


def lighten_dark_grey(r, g, b):
    if max(r, g, b) - min(r, g, b) <= GREY_MAX_CHROMA and max(r, g, b) <= DARK_MAX_LIGHTNESS:
        return 255 - r, 255 - g, 255 - b
    return r, g, b


def blend(color, alpha):
    return tuple((c * alpha + bg * (255 - alpha)) // 255 for c, bg in zip(color, BACKGROUND))


def rgb565(r, g, b):
    value = (r >> 3) << 11 | (g >> 2) << 5 | b >> 3
    return TRANSPARENT_NUDGED if value == TRANSPARENT else value


def pixel(r, g, b, a, keep_dark):
    if a < MIN_ALPHA:
        return TRANSPARENT
    color = (r, g, b) if keep_dark else lighten_dark_grey(r, g, b)
    return rgb565(*blend(color, a))


def main(path, max_width, max_height, out, keep_dark=False):
    image = Image.open(path).convert("RGBA")
    image.thumbnail((int(max_width), int(max_height)), Image.Resampling.LANCZOS)
    with open(out, "wb") as f:
        rgba = image.tobytes()
        for i in range(0, len(rgba), 4):
            f.write(pixel(*rgba[i : i + 4], keep_dark).to_bytes(2, "big"))
    print(f"{out}: width {image.width}, height {image.height}")


args = [a for a in sys.argv[1:] if a != "--keep-dark"]
if len(args) != 4:
    sys.exit(USAGE)
main(*args, keep_dark="--keep-dark" in sys.argv)
