#!/usr/bin/env python3
"""The README's and the wiki's translations (docs/i18n/README.<code>.md, the
wiki's <Page>-<code>.md), one per language of the program (Languages/*.lng).

`python3 tools/i18n_docs.py` writes the language bar (between
<!-- languages:start --> and <!-- languages:end -->, or in place of a bare
<!-- languages --> line) into README.md and every translation, and checks
that each translation still has the English README's sections: the
fixed anchors `<a id="...">` of README.md's `##` and `###` headings, in the
same order. `--wiki DIR` also writes the language list into the wiki's
_Sidebar.md and checks that every page has its translations. It exits
with 1 when a translation is missing or behind; the English files are the
reference, and a section added there is to be translated into each file.
"""
import os
import re
import sys

# file code, interface file, the name in the language (as the .lng says)
LANGS = [
    ("en", "English.lng"), ("de", "Deutsch.lng"), ("fr", "Francais.lng"),
    ("it", "Italiano.lng"), ("nl", "Nederlands.lng"), ("pl", "Polski.lng"),
    ("csb", "Kaszebsczi.lng"), ("hi", "Hindi.lng"), ("ja", "Japanese.lng"),
    ("ko", "Korean.lng"), ("zh-CN", "ChineseSimplified.lng"), ("zh-TW", "ChineseTaiwan.lng"),
]
WIKI_PAGES = ["Home", "Installing", "Lighting-workflow", "Swarm-setup", "Troubleshooting-and-FAQ"]
START, END = "<!-- languages:start -->", "<!-- languages:end -->"


def native_name(lng):
    for line in open(os.path.join("Languages", lng), encoding="utf-8"):
        m = re.match(r"\s*language\s*=\s*(.+)", line)
        if m:
            return m.group(1).strip()
    return lng[:-4]


def slug(title):
    title = re.sub(r"\[([^]]*)\]\([^)]*\)", r"\1", title)
    s = re.sub(r"[^\w\- ]", "", title.strip().lower())
    return s.replace(" ", "-")


def english_anchors():
    """The anchors of README.md's `##` and `###` sections, as the
    translations carry them in `<a id>` lines; the Updates section is one
    pointer there (its releases are not translated)."""
    out, in_updates = [], False
    for line in open("README.md", encoding="utf-8"):
        m = re.match(r"^(##|###) (.*)", line)
        if not m:
            continue
        if m.group(1) == "##":
            in_updates = m.group(2).startswith("Updates")
        elif in_updates:
            continue
        out.append(slug(m.group(2)))
    # (a heading's second use is <slug>-1, as GitHub names it)
    seen = {}
    for i, a in enumerate(out):
        n = seen.get(a, 0)
        seen[a] = n + 1
        if n:
            out[i] = f"{a}-{n}"
    return out


def bar(current, names, from_root):
    parts = []
    for code, _ in LANGS:
        name = names[code]
        if code == current:
            parts.append(f"**{name}**")
        elif from_root:
            parts.append(f"[{name}](README.md)" if code == "en" else f"[{name}](docs/i18n/README.{code}.md)")
        else:
            parts.append(f"[{name}](../../README.md)" if code == "en" else f"[{name}](README.{code}.md)")
    return " · ".join(parts)


def put_bar(path, text):
    s = open(path, encoding="utf-8").read()
    block = f"{START}\n{text}\n{END}"
    if START in s:
        s2 = re.sub(re.escape(START) + ".*?" + re.escape(END), lambda _: block, s, flags=re.S)
    elif "<!-- languages -->" in s:
        s2 = s.replace("<!-- languages -->", block, 1)
    elif re.search(r"^# ", s, flags=re.M):
        # below the first `# ` title
        s2 = re.sub(r"^(# .*\n)", lambda m: m.group(1) + "\n" + block + "\n", s, count=1, flags=re.M)
    else:
        # (the wiki's sidebar: at the end)
        s2 = s.rstrip("\n") + "\n\n" + block + "\n"
    if s2 != s:
        open(path, "w", encoding="utf-8").write(s2)


def main():
    names = {code: native_name(lng) for code, lng in LANGS}
    bad = 0
    put_bar("README.md", bar("en", names, True))
    want = english_anchors()
    for code, _ in LANGS[1:]:
        p = f"docs/i18n/README.{code}.md"
        if not os.path.exists(p):
            print(f"{p}: missing")
            bad += 1
            continue
        put_bar(p, bar(code, names, False))
        have = re.findall(r'<a id="([^"]+)"></a>', open(p, encoding="utf-8").read())
        if have != [a for a in want if a in have] or set(want) - set(have):
            missing = [a for a in want if a not in have]
            print(f"{p}: behind the English README; sections missing or out of order: {', '.join(missing) or '(order)'}")
            bad += 1
    if "--wiki" in sys.argv:
        wiki = sys.argv[sys.argv.index("--wiki") + 1]
        for page in WIKI_PAGES:
            for code, _ in LANGS[1:]:
                if not os.path.exists(os.path.join(wiki, f"{page}-{code}.md")):
                    print(f"wiki {page}-{code}.md: missing")
                    bad += 1
        side = os.path.join(wiki, "_Sidebar.md")
        langs = "**Languages**\n\n" + "\n".join(
            f"- [[{names[c]}|{'Home' if c == 'en' else 'Home-' + c}]]" for c, _ in LANGS)
        put_bar(side, langs)
    sys.exit(1 if bad else 0)


if __name__ == "__main__":
    main()
