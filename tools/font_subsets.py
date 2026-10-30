#!/usr/bin/env python3
"""The fonts of the languages whose characters the program's own fonts
lack (Languages/<Language>.ttf, named by `font =` in the language file):
subsets of the Noto fonts with the characters the language file uses, so
the download stays small. Run after a language file gained new text:

    python3 tools/font_subsets.py [--src-cjk FILE.ttc] [--src-deva FILE.ttf]

needs fontTools (pip install fonttools). A character of the old subset is
always kept, so a subset only grows. The CJK subsets are named "MB3D Sans
JP/KR/SC/TC" (they are not the whole Noto fonts); the Noto license is
Languages/OFL-NotoSans*.txt.
"""
import os, sys
from fontTools import subset
from fontTools.ttLib import TTFont, TTCollection

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
LANG = os.path.join(ROOT, "Languages")
args = sys.argv[1:]
def opt(name, default):
    return args[args.index(name) + 1] if name in args else default
CJK = opt("--src-cjk", "/usr/share/fonts/noto-cjk/NotoSansCJK-Regular.ttc")
DEVA = opt("--src-deva", "/usr/share/fonts/noto/NotoSansDevanagari-Regular.ttf")

# language file, face in the CJK collection (None: Devanagari), new family name
TARGETS = [
    ("Japanese", "Noto Sans CJK JP", "MB3D Sans JP"),
    ("Korean", "Noto Sans CJK KR", "MB3D Sans KR"),
    ("ChineseSimplified", "Noto Sans CJK SC", "MB3D Sans SC"),
    ("ChineseTaiwan", "Noto Sans CJK TC", "MB3D Sans TC"),
    ("Hindi", None, None),
]


def chars_of_language(name):
    text = open(os.path.join(LANG, name + ".lng"), encoding="utf-8").read()
    return {c for c in text if ord(c) > 0x7F}


def cmap_chars(font):
    return {chr(u) for u in font.getBestCmap()}


def source(face):
    if face is None:
        return TTFont(DEVA)
    for f in TTCollection(CJK).fonts:
        if f["name"].getDebugName(1) == face:
            return f
    sys.exit(f"{face} is not in {CJK}")


def rename(font, family):
    for rec in font["name"].names:
        if rec.nameID in (1, 16):
            rec.string = family
        elif rec.nameID == 4:
            rec.string = family + " Regular"
        elif rec.nameID == 6:
            rec.string = family.replace(" ", "") + "-Regular"
        elif rec.nameID == 3:
            rec.string = family + " subset"


for name, face, family in TARGETS:
    out = os.path.join(LANG, name + ".ttf")
    keep = chars_of_language(name)
    old = set()
    if os.path.exists(out):
        old = cmap_chars(TTFont(out))
    # (Latin letters come from the program's own fonts)
    want = keep | old
    font = source(face)
    have = cmap_chars(font)
    if old and (want & have) <= old:
        print(f"{name:18} {len(old):5} characters, nothing new")
        continue
    missing = sorted(c for c in keep if c not in have and not c.isspace() and ord(c) not in (0xFEFF,))
    o = subset.Options()
    if face is None:
        # Devanagari needs all its shaping rules (conjuncts, matras)
        o.layout_features = ["*"]
    else:
        # (CJK: one glyph per character, no layout tables: the program
        # draws them one by one; features would double the file)
        o.layout_features = []
        o.hinting = False
    o.name_IDs = ["*"]
    o.name_languages = ["*"]
    o.notdef_outline = True
    o.glyph_names = False
    s = subset.Subsetter(o)
    s.populate(text="".join(c for c in want if c in have))
    s.subset(font)
    if family:
        rename(font, family)
    font.save(out)
    new = cmap_chars(TTFont(out))
    print(f"{name:18} {len(old):5} -> {len(new):5} characters, {os.path.getsize(out) // 1024} kB"
          + (f"; not in the source font: {''.join(missing[:20])}" if missing else ""))
