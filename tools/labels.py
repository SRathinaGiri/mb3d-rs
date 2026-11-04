#!/usr/bin/env python3
"""The program's labels (Languages/*.lng, [Labels]), in the way of Microsoft
Dynamics 365's label files: the form files and the code refer to a label as
@MB:Id, English.lng holds every label's English text.

    python3 tools/labels.py find TEXT     labels whose text or ID contain TEXT
                                          (reuse one before adding a new one)
    python3 tools/labels.py add "Text"    a new label for Text (or the existing
                                          one with the same text): prints @MB:Id
    python3 tools/labels.py check         every @MB:Id of the forms and the code
                                          exists; unused labels; per language the
                                          labels not translated yet
    python3 tools/labels.py sync          puts all language files in order: the
                                          labels by ID, under each a note (in
                                          English.lng where it is used, in the
                                          others its English text), the form
                                          sections kept, the labels not translated
                                          yet listed at the end as notes

Run from the repository. `check` exits with 1 when a reference is missing.
"""
import os, re, sys, collections

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
LANG = os.path.join(ROOT, "Languages")
FORMS = os.path.join(ROOT, "src", "app", "forms")
SRC = os.path.join(ROOT, "src")
REF = re.compile(r"@MB:([A-Za-z0-9_]+)")


def decode(s):
    out, i = [], 0
    while i < len(s):
        c = s[i]
        if c != "\\":
            out.append(c); i += 1; continue
        n = s[i + 1] if i + 1 < len(s) else None
        out.append({"n": "\n", "t": "\t", "s": " "}.get(n, n if n is not None else "\\"))
        i += 2
    return "".join(out)


def encode(s, key=False):
    s = s.replace("\r\n", "\n").replace("\r", "\n")
    lead = len(s) - len(s.lstrip(" "))
    trail = 0 if lead == len(s) else len(s) - len(s.rstrip(" "))
    out = []
    for i, c in enumerate(s):
        if c == "\\": out.append("\\\\")
        elif c == "\n": out.append("\\n")
        elif c == "\t": out.append("\\t")
        elif c == " " and (i < lead or i >= len(s) - trail): out.append("\\s")
        elif c == "=" and key: out.append("\\=")
        elif c in "#;[" and key and i == 0: out.append("\\" + c)
        else: out.append(c)
    return "".join(out)


def split_eq(t):
    i = 0
    while i < len(t):
        if t[i] == "\\": i += 2; continue
        if t[i] == "=": return i
        i += 1
    return None


def read(path):
    """(header lines, header [(key, value)], labels {id: text} in order,
    sections [(name, [(key, text)])] other than [Labels])"""
    comments, head, labels, sections, sec = [], [], {}, [], ""
    for raw in open(path, encoding="utf-8"):
        line = raw.rstrip("\n").lstrip("﻿")
        t = line.strip()
        if t.startswith("#") and not head and not labels and not sections:
            comments.append(line)
            continue
        if not t or t.startswith("#") or t.startswith(";"):
            continue
        if t.startswith("[") and t.endswith("]"):
            sec = t[1:-1].strip()
            if sec.lower() != "labels":
                sections.append((sec, []))
            continue
        eq = split_eq(t)
        if eq is None:
            continue
        key, val = decode(t[:eq].strip()), t[eq + 1:]
        if val.startswith(" "):
            val = val[1:]
        val = decode(val)
        if sec == "":
            head.append((key, val))
        elif sec.lower() == "labels":
            labels[key] = val
        else:
            sections[-1][1].append((key, val))
    return comments, head, labels, sections


def english():
    return read(os.path.join(LANG, "English.lng"))[2]


def references():
    """label ID -> the places that use it (Form.Control.Property, or 'the program')"""
    refs = collections.defaultdict(list)
    for fn in sorted(os.listdir(FORMS)):
        if not fn.endswith(".dfm"):
            continue
        form, stack, prop = None, [], None
        for line in open(os.path.join(FORMS, fn), encoding="utf-8"):
            t = line.strip()
            m = re.match(r"(object|inherited|inline)\s+(\w+)\s*:", t)
            if m:
                stack.append(m.group(2)); form = form or m.group(2); continue
            if t == "end" and stack:
                stack.pop(); continue
            m = re.match(r"(\w+(?:\.Strings)?) = ", t)
            if m:
                prop = m.group(1).replace(".Strings", "")
            for r in REF.findall(line):
                ctrl = stack[-1] if stack else form
                where = form if ctrl == form else f"{form}.{ctrl}"
                refs[r].append(f"{where}.{prop or 'Caption'}")
    for root, _, files in os.walk(SRC):
        for fn in sorted(files):
            if fn.endswith(".rs"):
                for r in REF.findall(open(os.path.join(root, fn), encoding="utf-8").read()):
                    if r != "Id":
                        refs[r].append("the program")
    return refs


def base_id(text):
    words = re.findall(r"[A-Za-z0-9]+", text)
    i = "".join(w[:1].upper() + w[1:] for w in words[:6]) or "Text"
    return ("N" + i if i[0].isdigit() else i)[:48]


def header(language, code):
    return """# Mandelbulb3D interface language (UTF-8)
#
# The texts are labels, as in Microsoft Dynamics 365: in [Labels], each line
# is "Id=text". The windows and the program refer to a label by its Id
# (@MB:Id), so one label gives every place that shows that text. Translate
# the text right of the "="; a label left out shows the English text. The
# line under a label starting with ";" is its English text (in English.lng:
# where it is used); it is only a note.
#
# A window section ([FormName], "Control.Property = text") may follow: a
# text there is used for that one control instead of its label, where one
# place needs other words than the others.
#
# "\\n" is a line break, "\\\\" a backslash, "\\s" a space at the start or end
# of a text. "&" before a letter underlines it (the Alt key shortcut); {} and
# {0}, {1} ... are values the program fills in (a translation may change
# their order).
#
# Your own language: click the language badge in the title bar of the main
# window, type the language's name and press "Create language file";
# translate the new file with a text editor and press "Reload". The file's
# name is the language's; "code" is shown on the badge. A picture
# <name>.png next to the file is the language's flag (18 x 12).
# "font = file.ttf" names a font next to the file for characters the
# program's fonts lack (Japanese, Chinese, Korean ...).
"""


def where(places):
    u = [p for p in dict.fromkeys(places) if p != "the program"]
    s = ", ".join(u[:3]) + (f" and {len(u) - 3} more" if len(u) > 3 else "")
    if "the program" in places:
        s = (s + "; " if s else "") + "the program"
    return s or "not used"


def write(path, head, labels, sections, notes, missing=None):
    out = [header("", ""), "\n"]
    out += [f"{k} = {encode(v)}\n" for k, v in head]
    out.append("\n[Labels]\n")
    for i in sorted(labels):
        out.append(f"{i}={encode(labels[i])}\n;{notes(i)}\n")
    for name, items in sections:
        if items:
            out.append(f"\n[{name}]\n")
            out += [f"{encode(k, True)} = {encode(v)}\n" for k, v in items]
    if missing:
        # (under [Labels] again: a line translated here is a label)
        out.append("\n[Labels]\n; Not translated yet (shown in English): remove the \";\" and translate\n")
        out += [f";{i}={encode(t)}\n" for i, t in missing]
    open(path, "w", encoding="utf-8").write("".join(out))


def cmd_find(text):
    t = text.lower()
    for i, v in sorted(english().items()):
        if t in v.lower() or t in i.lower():
            print(f"@MB:{i} = {encode(v)}")


def cmd_add(text):
    en_path = os.path.join(LANG, "English.lng")
    comments, head, labels, sections = read(en_path)
    for i, v in labels.items():
        if v == text:
            print(f"@MB:{i}  (exists already)")
            return
    b, i, n = base_id(text), base_id(text), 2
    while i in labels:
        i, n = f"{b}{n}", n + 1
    labels[i] = text
    refs = references()
    write(en_path, head, labels, sections, lambda x: where(refs.get(x, [])))
    print(f"@MB:{i}")


def cmd_check():
    en, refs, bad = english(), references(), 0
    for r, places in sorted(refs.items()):
        if r not in en:
            print(f"missing label @MB:{r} (used in {where(places)})")
            bad += 1
    unused = sorted(set(en) - set(refs))
    if unused:
        print(f"{len(unused)} labels not referenced in the forms or as @MB: in the code "
              f"(the code may pass their English text around): {', '.join(unused[:20])}{' ...' if len(unused) > 20 else ''}")
    for fn in sorted(os.listdir(LANG)):
        if fn.endswith(".lng") and fn != "English.lng":
            labels = read(os.path.join(LANG, fn))[2]
            extra = sorted(set(labels) - set(en))
            print(f"{fn:24} {len(labels):5} of {len(en)} labels translated" + (f"; unknown labels: {', '.join(extra)}" if extra else ""))
            bad += len(extra)
    sys.exit(1 if bad else 0)


def cmd_sync():
    en_path = os.path.join(LANG, "English.lng")
    _, head, en, sections = read(en_path)
    refs = references()
    write(en_path, head, en, sections, lambda i: where(refs.get(i, [])))
    for fn in sorted(os.listdir(LANG)):
        if fn.endswith(".lng") and fn != "English.lng":
            p = os.path.join(LANG, fn)
            _, h, labels, secs = read(p)
            # (a label line written under a form section by hand: a label)
            for _, items in secs:
                for k, v in list(items):
                    if "." not in k and k != "Caption" and k in en:
                        labels.setdefault(k, v)
                        items.remove((k, v))
            labels = {i: t for i, t in labels.items() if i in en}
            missing = [(i, en[i]) for i in sorted(en) if i not in labels]
            write(p, h, labels, secs, lambda i: encode(en[i]), missing)
    print("Languages/*.lng put in order")


if __name__ == "__main__":
    a = sys.argv[1:]
    if not a or a[0] in ("-h", "--help"):
        print(__doc__); sys.exit(0)
    {"find": lambda: cmd_find(" ".join(a[1:])), "add": lambda: cmd_add(" ".join(a[1:])),
     "check": cmd_check, "sync": cmd_sync}.get(a[0], lambda: sys.exit(f"unknown command {a[0]}"))()
