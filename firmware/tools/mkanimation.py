import sys

from PIL import Image, ImageChops, ImageSequence

USAGE = "usage: uv run tools/mkanimation.py <gif> <height_px> <out.raw>   frames are RGB565, big-endian, back to back"
# The screen background (ui::BG); the GIF's white background is replaced with it, since frames are drawn opaque.
BACKGROUND = (0x09, 0x09, 0x09)
WHITE_MIN = 235
# Anti-aliasing between the white background and the black outline leaves light greys; those become
# background too, or every frame gets a pale halo. Body colours are saturated, so they are kept.
HALO_MIN_LIGHTNESS = 110
HALO_MAX_CHROMA = 40


def rgb565(r, g, b):
    return (r >> 3) << 11 | (g >> 2) << 5 | b >> 3


def content_box(frames):
    box = None
    for frame in frames:
        not_white = ImageChops.invert(frame.convert("L")).point(lambda v: 255 if v > 255 - WHITE_MIN else 0)
        b = not_white.getbbox()
        box = b if box is None else (min(box[0], b[0]), min(box[1], b[1]), max(box[2], b[2]), max(box[3], b[3]))
    return box


def main(path, height, out):
    gif = Image.open(path)
    frames = [f.convert("RGB").copy() for f in ImageSequence.Iterator(gif)]
    box = content_box(frames)
    width = round((box[2] - box[0]) * int(height) / (box[3] - box[1]))
    with open(out, "wb") as f:
        for frame in frames:
            small = frame.crop(box).resize((width, int(height)), Image.Resampling.LANCZOS)
            raw = small.tobytes()
            for i in range(0, len(raw), 3):
                r, g, b = raw[i : i + 3]
                if min(r, g, b) >= HALO_MIN_LIGHTNESS and max(r, g, b) - min(r, g, b) <= HALO_MAX_CHROMA:
                    r, g, b = BACKGROUND
                f.write(rgb565(r, g, b).to_bytes(2, "big"))
    durations = sorted({f.info.get("duration") for f in ImageSequence.Iterator(gif)})
    print(f"{out}: width {width}, height {height}, frames {len(frames)}, frame ms {durations}")


if len(sys.argv) != 4:
    sys.exit(USAGE)
main(*sys.argv[1:])
