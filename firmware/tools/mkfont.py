import sys

from PIL import Image, ImageDraw, ImageFont

USAGE = "usage: uv run tools/mkfont.py <ttf> <size_px> <out.raw>   prints the Rust MonoFont params"
FIRST_CHAR = 32
LAST_CHAR = 126
GLYPHS_PER_ROW = 16
WHITE_THRESHOLD = 110


def glyph_size(font):
    ascent, descent = font.getmetrics()
    return round(font.getlength("M")), ascent + descent


def draw_atlas(font, width, height):
    chars = [chr(c) for c in range(FIRST_CHAR, LAST_CHAR + 1)]
    rows = -(-len(chars) // GLYPHS_PER_ROW)
    atlas = Image.new("L", (GLYPHS_PER_ROW * width, rows * height), 0)
    draw = ImageDraw.Draw(atlas)
    for i, ch in enumerate(chars):
        draw.text(((i % GLYPHS_PER_ROW) * width, (i // GLYPHS_PER_ROW) * height), ch, font=font, fill=255)
    return atlas


def to_black_and_white(atlas):
    return atlas.point(lambda v: 255 if v >= WHITE_THRESHOLD else 0).convert("1")


def main(ttf, size, out):
    size = int(size)
    font = ImageFont.truetype(ttf, size)
    width, height = glyph_size(font)
    with open(out, "wb") as f:
        f.write(to_black_and_white(draw_atlas(font, width, height)).tobytes())
    baseline = font.getmetrics()[0] - 1
    print(f"// {ttf.split('/')[-1]} {size}px: image width {GLYPHS_PER_ROW * width}, size ({width}, {height}), baseline {baseline}")


if len(sys.argv) != 4:
    sys.exit(USAGE)
main(*sys.argv[1:])
