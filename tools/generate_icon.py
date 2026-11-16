"""Generate the black MOV/MP4 icon with separately rendered small sizes."""

from io import BytesIO
from pathlib import Path
from struct import pack

from PIL import Image, ImageDraw, ImageFont


ASSETS = Path(__file__).resolve().parent.parent / "assets"
FONT = "C:/Windows/Fonts/segoeuib.ttf"
SIZES = (16, 24, 32, 48, 64, 128, 256)


def word_mask(word: str, width: int, height: int) -> Image.Image:
    font = ImageFont.truetype(FONT, 300)
    bounds = font.getbbox(word)
    mask = Image.new("L", (bounds[2] - bounds[0], bounds[3] - bounds[1]), 0)
    ImageDraw.Draw(mask).text((-bounds[0], -bounds[1]), word, font=font, fill=255)
    return mask.resize((width, height), Image.Resampling.LANCZOS)


def make_icon(size: int) -> Image.Image:
    icon = Image.new("RGB", (size, size), (8, 8, 9))
    margin = max(2, round(size * 0.05))
    gap = max(1, round(size * 0.045))
    line_height = (size - 2 * margin - gap) // 2
    for word, top, color in (
        ("MOV", margin, (190, 190, 192)),
        ("MP4", size - margin - line_height, (255, 255, 255)),
    ):
        mask = word_mask(word, size - 2 * margin, line_height)
        if size <= 32:
            mask = mask.point(lambda pixel: 255 if pixel >= 128 else 0)
        icon.paste(color, (margin, top), mask)
    return icon


def build_ico() -> bytes:
    frames = []
    for size in SIZES:
        buffer = BytesIO()
        make_icon(size).convert("RGBA").save(buffer, format="PNG")
        frames.append((size, buffer.getvalue()))

    header = pack("<HHH", 0, 1, len(frames))
    offset = len(header) + 16 * len(frames)
    entries = bytearray()
    images = bytearray()
    for size, data in frames:
        dimension = 0 if size == 256 else size
        entries.extend(pack("<BBBBHHII", dimension, dimension, 0, 0, 1, 32, len(data), offset))
        images.extend(data)
        offset += len(data)
    return header + entries + images


def main() -> None:
    ASSETS.mkdir(exist_ok=True)
    make_icon(1024).save(ASSETS / "mov2mp4.png")
    (ASSETS / "mov2mp4.ico").write_bytes(build_ico())


if __name__ == "__main__":
    main()
