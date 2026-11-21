import sys

from PIL import Image

USAGE = "usage: uv run tools/mkimage.py <image> <size_px> <out.raw> [white_threshold]"
DEFAULT_WHITE_THRESHOLD = 128


def to_black_and_white(path, size, threshold):
    grey = Image.open(path).convert("L")
    grey.thumbnail((size, size), Image.Resampling.LANCZOS)
    return grey.point(lambda v: 255 if v >= threshold else 0).convert("1")


def main(path, size, out, threshold=DEFAULT_WHITE_THRESHOLD):
    image = to_black_and_white(path, int(size), int(threshold))
    with open(out, "wb") as f:
        f.write(image.tobytes())
    print(f"{out}: width {image.width}, height {image.height}")


if len(sys.argv) not in (4, 5):
    sys.exit(USAGE)
main(*sys.argv[1:])
