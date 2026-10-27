"""Subsets the instrument font (Geist Mono, variable) into a small woff2.

Usage: python scripts/subset-font.py <GeistMono[wght].ttf>

Download the source from https://github.com/vercel/geist-font/releases (GeistMono/variable).
Needs `pip install fonttools brotli`. Writes app/src/lib/fonts/GeistMono-subset.woff2; the
license (OFL) sits next to it. Budget: <= 25 KB (design-language.md section 3).
"""

import sys
from pathlib import Path

from fontTools import subset
from fontTools.ttLib import TTFont

ROOT = Path(__file__).resolve().parent.parent
OUT = ROOT / "app" / "src" / "lib" / "fonts" / "GeistMono-subset.woff2"
BUDGET = 25 * 1024

# Printable ASCII, Turkish letters and the instrument symbols. The face is used for values
# only, but a full Latin set keeps unit and clock strings from falling back mid-word.
TEXT = (
    "".join(chr(c) for c in range(0x20, 0x7F))
    + "çğıöşüÇĞİÖŞÜ"
    + "\u00a0"  # no-break space
    + "≈▲▼·−…"
)


def main() -> int:
    if len(sys.argv) != 2:
        print(__doc__)
        return 2
    font = TTFont(sys.argv[1])
    if "fvar" not in font:
        print("expected the variable font (GeistMono[wght].ttf)")
        return 1
    options = subset.Options()
    options.flavor = "woff2"
    # Monospace already gives tabular digits; `tnum` is kept for fonts that ever need it.
    options.layout_features = ["kern", "tnum", "zero", "case"]
    options.name_IDs = [0, 1, 2, 3, 4, 5, 6, 13, 14]  # names plus copyright and license
    options.notdef_outline = True
    options.drop_tables += ["meta"]
    subsetter = subset.Subsetter(options)
    subsetter.populate(text=TEXT)
    subsetter.subset(font)
    cmap = font.getBestCmap()
    missing = [c for c in TEXT if ord(c) not in cmap]
    OUT.parent.mkdir(parents=True, exist_ok=True)
    font.flavor = "woff2"
    font.save(OUT)
    size = OUT.stat().st_size
    print(f"{OUT.relative_to(ROOT)}: {size} bytes, missing: {[f'U+{ord(c):04X}' for c in missing] or 'none'}")
    if size > BUDGET:
        print(f"over the {BUDGET} byte budget")
        return 1
    return 0


if __name__ == "__main__":
    sys.exit(main())
