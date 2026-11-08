# RF463 evidence pixel analysis — run from repo root:
#   uv run --no-project --with pillow python evidence/rf463-spectrogram-intensity/analyze.py <png>
# Prints the dominant colours inside the spectrogram canvas and a
# horizontal scanline through its middle, for before.png / after.png.
import sys

from PIL import Image


def is_canvas_blue(p: tuple[int, int, int]) -> bool:
    r, g, b = p
    return b > r + 10 and 40 < b < 240


def main() -> None:
    path = sys.argv[1]
    im = Image.open(path).convert("RGB")
    w, h = im.size
    px = im.load()
    xs, ys = [], []
    for y in range(0, h, 4):
        for x in range(0, w, 4):
            if is_canvas_blue(px[x, y]):
                xs.append(x)
                ys.append(y)
    x0, x1, y0, y1 = min(xs), max(xs), min(ys), max(ys)
    colors: dict[tuple[int, int, int], int] = {}
    for y in range(y0 + 10, y1 - 10, 3):
        for x in range(x0 + 10, x1 - 10, 3):
            c = px[x, y]
            colors[c] = colors.get(c, 0) + 1
    print(f"{path}: canvas bbox=({x0},{y0})-({x1},{y1}), distinct={len(colors)}")
    for c, n in sorted(colors.items(), key=lambda kv: -kv[1])[:10]:
        print(f"  #{c[0]:02X}{c[1]:02X}{c[2]:02X} {n}")
    y = (y0 + y1) // 2
    print(
        f"scanline y={y}:",
        " ".join(f"{px[x, y][0]:02X}{px[x, y][1]:02X}{px[x, y][2]:02X}"
                 for x in range(x0 + 10, x1 - 10, 24)),
    )


if __name__ == "__main__":
    main()
