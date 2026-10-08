#!/usr/bin/env python3
"""Regenerate UI icon caches from SVG sources (optional resvg-py dependency)."""
import argparse
from pathlib import Path

ASSETS = Path(__file__).resolve().parent.parent / "assets" / "icons"
ICONS = ("clear", "previous", "next", "empty", "show", "hide", "add")
# Keep colors in sync with secondary/disabled_text in rust/theme.rs.
VARIANTS = {
    "light": "#637584",
    "dark": "#a0b3be",
    "light-disabled": "#7d8d99",
    "dark-disabled": "#6e8793",
}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("icons", nargs="*", help="Icon names; defaults to all icons")
    parser.add_argument("--output", type=Path, default=ASSETS)
    args = parser.parse_args()
    names = args.icons or ICONS
    if any(name not in ICONS for name in names):
        parser.error("Icon names must be: " + ", ".join(ICONS))
    try:
        import resvg_py
    except ImportError:
        parser.exit(1, "Install resvg-py in a Python environment to regenerate icons.\n")
    args.output.mkdir(parents=True, exist_ok=True)
    for name in names:
        source = (ASSETS / f"{name}.svg").read_text(encoding="utf-8")
        for variant, color in VARIANTS.items():
            png = resvg_py.svg_to_bytes(
                svg_string=source.replace("currentColor", color),
                width=96,
                height=96,
                skip_system_fonts=True,
            )
            (args.output / f"{name}-{variant}.png").write_bytes(png)


if __name__ == "__main__":
    main()
