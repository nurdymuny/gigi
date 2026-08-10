#!/usr/bin/env python3
"""Generate the LaTeX shape-verb chapters from docs/SHAPE_VERBS.md.

`docs/SHAPE_VERBS.md` is the single source of truth for what TEXTURE,
PRECEDENCE and CADENCE do. HELICITY's customer manual used to carry a
hand-written second copy of the same material, and it drifted in under a day:
it documented TEXTURE's old fixed verdict bands and omitted PRECEDENCE's
significance instrument entirely, while the shipped service had already moved.

So the manual no longer owns that material. It `\\input`s the fragment this
script emits, and `tests/shape_verbs_tex_is_current.py` fails if the committed
fragment is not what the current markdown produces.

    python scripts/shape_verbs_to_tex.py            # write docs/generated/shape_verbs.tex
    python scripts/shape_verbs_to_tex.py --check    # exit 1 if it would change

The output depends on environments the manual already defines — `keyinsight`,
`warningbox`, `infobox`, `refusal`, `lstlisting`, booktabs rules — and on its
`\\GIGI` / `\\TEXTURE` / `\\PRECEDENCE` / `\\CADENCE` macros. It is a FRAGMENT:
no preamble, no `\\begin{document}`.
"""
from __future__ import annotations

import argparse
import hashlib
import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
SRC = ROOT / "docs" / "SHAPE_VERBS.md"
OUT = ROOT / "docs" / "generated" / "shape_verbs.tex"

def banner(source_sha: str) -> str:
    # The source hash is stamped in so the drift gate needs no Python at test
    # time: `tests/generated_tex_is_current.rs` hashes docs/SHAPE_VERBS.md and
    # compares against this line. Edit the markdown without regenerating and
    # `cargo test` fails.
    return (
        "%% ============================================================\n"
        "%% GENERATED FILE - DO NOT EDIT.\n"
        "%%\n"
        "%% Produced from docs/SHAPE_VERBS.md by scripts/shape_verbs_to_tex.py.\n"
        "%% Edit the markdown and regenerate; hand edits here are overwritten.\n"
        "%%\n"
        f"%% source-sha256: {source_sha}\n"
        "%% ============================================================\n"
    )

# Unicode the markdown uses -> LaTeX. Applied to prose, never inside listings.
UNI = {
    "§": r"\S{}", "±": r"$\pm$", "²": r"$^2$", "³": r"$^3$", "·": r"$\cdot$",
    "½": r"$\tfrac12$", "×": r"$\times$", "–": "--", "—": "---",
    "−": r"$-$", "√": r"$\sqrt{}$", "≈": r"$\approx$", "≤": r"$\le$",
    "≥": r"$\ge$", "→": r"$\to$", "⚠": r"", "é": r"\'e", "’": "'",
    "“": "``", "”": "''", "…": r"\ldots{}",
}

SPECIALS = {"&": r"\&", "%": r"\%", "$": r"\$", "#": r"\#", "_": r"\_"}


def esc_text(t: str) -> str:
    """Escape a run of prose. Inline code and bold are handled by `inline`."""
    t = t.replace("\\", r"\textbackslash{}")
    for ch, rep in SPECIALS.items():
        t = t.replace(ch, rep)
    t = t.replace("{", r"\{").replace("}", r"\}")
    t = t.replace("~", r"\textasciitilde{}").replace("^", r"\textasciicircum{}")
    for ch, rep in UNI.items():
        t = t.replace(ch, rep)
    return t


def esc_code(t: str) -> str:
    """Escape the inside of a \\texttt{}."""
    t = t.replace("\\", r"\textbackslash{}")
    for ch, rep in SPECIALS.items():
        t = t.replace(ch, rep)
    t = t.replace("{", r"\{").replace("}", r"\}")
    t = t.replace("~", r"\textasciitilde{}").replace("^", r"\textasciicircum{}")
    for ch, rep in UNI.items():
        t = t.replace(ch, rep)
    return t


# A long path inside \texttt is one unbreakable token, so TeX must either set it
# whole or move the entire thing to the next line -- and when the preceding prose
# already fills most of the measure it does neither well. That is how
# `examples/texture_precedence_walkthrough.py` ended up 88 pt past the margin
# even after the link markup was fixed. Offering breaks after `/` and `_` lets
# TeX split a path the way a reader would. Zero-width, so nothing moves unless a
# break is actually taken, and short spans are left alone: `min_lag` wrapping
# mid-word would be worse than the problem.
BREAK_AFTER = ("/", r"\_", "-", ".")
MIN_BREAKABLE = 24


def breakable(escaped: str) -> str:
    """Allow line breaks inside a long escaped code span."""
    if len(escaped) < MIN_BREAKABLE:
        return escaped
    for sep in BREAK_AFTER:
        escaped = escaped.replace(sep, sep + r"\allowbreak{}")
    return escaped


# Markdown links. Every target in SHAPE_VERBS.md is a repository-relative path
# and every label already spells that path out, so a PDF reader gains nothing
# from the target -- while an unconverted `[label](target)` both prints the
# brackets literally and pushed one line 334 pt past the margin. The `](` is
# what makes this safe against code spans like `[ALONG o]`, which never have it.
LINK = re.compile(r"\[([^\]]*)\]\(([^)]*)\)")


def inline(s: str) -> str:
    """Markdown inline -> LaTeX, protecting code spans from prose escaping.

    Code spans are lifted out to opaque placeholders rather than converted in
    place, because emphasis is allowed to span them. Converting them in place
    chopped the string into fragments and left `**skipped and counted in
    `notes`**` with its markers in two different fragments, so the bold regex
    matched neither -- and four literal asterisks reached the customer PDF.
    A placeholder keeps the run whole while still shielding the code from the
    prose escaper.
    """
    s = LINK.sub(lambda m: m.group(1), s)

    spans: list[str] = []

    def stash(m: re.Match) -> str:
        spans.append(m.group(1))
        return "\x00%d\x00" % (len(spans) - 1)

    s = re.sub(r"`([^`]+)`", stash, s)

    # **bold** then *italic*, via placeholders so the escaper cannot mangle the
    # markers. `\x00` digits survive both passes untouched.
    s = re.sub(r"\*\*(.+?)\*\*", lambda m: "\x01" + m.group(1) + "\x02", s)
    s = re.sub(r"(?<!\*)\*([^*]+)\*(?!\*)", lambda m: "\x03" + m.group(1) + "\x02", s)
    s = esc_text(s)
    s = s.replace("\x01", r"\textbf{").replace("\x03", r"\emph{").replace("\x02", "}")

    return re.sub(
        r"\x00(\d+)\x00",
        lambda m: r"\texttt{" + breakable(esc_code(spans[int(m.group(1))])) + "}",
        s,
    )


# Sections the customer manual cross-references by name. Slugs are derived from
# titles, so an editorial reword of a heading would silently break those refs;
# pinning the four the manual actually cites keeps `\ref` working across a
# rename. A title that stops matching shows up as an undefined reference at
# build time rather than as a wrong page number.
LABEL_MAP = {
    "texture": "sec:texture",
    "precedence": "sec:precedence",
    "cadence": "sec:gigi-cadence",
    "using all three": "sec:together",
}


def slug(title: str) -> str:
    low = title.lower()
    for key, label in LABEL_MAP.items():
        if low.startswith(key):
            return label[4:]          # strip the "sec:" the caller re-adds
    t = re.sub(r"[^a-z0-9]+", "-", low).strip("-")
    return "gigi-" + t[:40]


# `listings` reads its body byte-for-byte and cannot take raw UTF-8. Anything
# non-ASCII inside a code block has to become ASCII or the build dies with
# "Invalid UTF-8 byte sequence" pointing into lst@FillFixed.
LISTING_ASCII = {
    "±": "+/-", "×": "x", "·": ".", "–": "-", "—": "--", "−": "-",
    "≈": "~", "≤": "<=", "≥": ">=", "→": "->", "√": "sqrt", "²": "^2",
    "³": "^3", "½": "1/2", "§": "S", "⚠": "!", "…": "...",
    "“": '"', "”": '"', "’": "'", "é": "e",
}


def ascii_listing(line: str) -> str:
    for ch, rep in LISTING_ASCII.items():
        line = line.replace(ch, rep)
    return "".join(c if ord(c) < 128 else "?" for c in line)


# The manual's text block is 16.0 cm (A4, 2.5 cm margins). A tabular wider than
# that runs off the page, and LaTeX only mutters `Overfull \hbox` and carries on
# -- so the PDF ships with a truncated table and nothing fails. The Reference
# table did exactly that: four columns, 251 pt over, the CADENCE column sheared
# off at the paper's edge in the customer PDF.
TEXT_WIDTH_CM = 16.0
# Inside a tcolorbox the measure is narrower by the box's own left and right
# padding plus its rule, about 6 mm a side in the manual's house style.
BOX_WIDTH_CM = 14.8
COLSEP_CM = 0.4218                  # 2 x \tabcolsep at the default 6 pt
SAFETY_CM = 0.15
CHAR_CM = 5.0 / 28.4527             # conservative width of one \small tt char

# Widths already checked by eye for the two- and three-column shapes; the
# allocator below handles every other shape, which is where the bug was.
TUNED = {2: [5.4, 8.0], 3: [3.6, 4.6, 5.0]}


def column_widths(cols: list[list[str]],
                  width_cm: float = TEXT_WIDTH_CM) -> list[float]:
    """Widths in cm that fit the text block and never split a single token.

    Allocating purely in proportion to content puts the 12-character header
    `significance` in a 0.9 cm column, which overflows just as badly in the
    other direction. So each column first claims enough room for its longest
    unbreakable token -- `\\texttt` does not hyphenate -- and only the slack
    left over is shared out in proportion to how much text each column holds.
    """
    n = len(cols)
    budget = width_cm - (n - 1) * COLSEP_CM - SAFETY_CM

    floors = [max((len(tok) for cell in col for tok in cell.split()), default=1)
              * CHAR_CM for col in cols]
    demand = [max((len(cell) for cell in col), default=1) for col in cols]

    slack = budget - sum(floors)
    if slack <= 0:
        # Even the longest tokens do not fit. Nothing can be laid out without
        # some overflow, so share the shortfall rather than pick a victim.
        scale = budget / sum(floors)
        return [f * scale for f in floors]

    return [f + slack * d / sum(demand) for f, d in zip(floors, demand)]


def emit_table(rows: list[str], width_cm: float = TEXT_WIDTH_CM) -> list[str]:
    """A markdown pipe table -> booktabs tabular.

    `width_cm` is the measure the table has to fit. Inside a tcolorbox that is
    narrower than the text block by the box's own padding, so callers there
    pass BOX_WIDTH_CM.
    """
    cells = [[c.strip() for c in r.strip().strip("|").split("|")] for r in rows]
    header, body = cells[0], cells[2:]          # cells[1] is the --- separator
    n = len(header)
    body = [r + [""] * (n - len(r)) if len(r) < n else r[:n] for r in body]

    cols = [[r[i] for r in [header] + body] for i in range(n)]
    widest = max((len(c) for col in cols for c in col), default=0)
    if widest > 44:
        widths = TUNED.get(n) or column_widths(cols, width_cm)
        # raggedright: justified p-columns full of \texttt stretch interword
        # space into gaps, because monospace tokens will not compress.
        spec = "".join(r">{\raggedright\arraybackslash}p{%.2fcm}" % w
                       for w in widths)
    else:
        spec = "l" * n

    out = [r"\begin{center}", r"\small", r"\begin{tabular}{@{}%s@{}}" % spec, r"\toprule"]
    out.append(" & ".join(r"\textbf{%s}" % inline(c) for c in header) + r" \\")
    out.append(r"\midrule")
    for r in body:
        out.append(" & ".join(inline(c) for c in r) + r" \\")
    out += [r"\bottomrule", r"\end{tabular}", r"\end{center}"]
    return out


def box_for(title: str) -> str:
    """Pick the manual's box environment from the blockquote's heading."""
    low = title.lower()
    if "⚠" in title or "trap" in low or "do not" in low or "not rely" in low:
        return "warningbox"
    if "refus" in low or "reject" in low:
        return "refusal"
    if "read this" in low or "why" in low or "does not mean" in low:
        return "keyinsight"
    return "infobox"


def convert(md: str) -> str:
    source_sha = hashlib.sha256(md.encode('utf-8')).hexdigest()
    lines = md.split("\n")
    out: list[str] = []
    i = 0
    # Skip the H1 and everything before the first "## " — the manual supplies
    # its own framing for this chapter.
    while i < len(lines) and not lines[i].startswith("## "):
        i += 1

    while i < len(lines):
        line = lines[i]

        # ---- fenced code -------------------------------------------------
        m = re.match(r"^```(\w*)", line)
        if m:
            lang = {"bash": "Shell", "sql": "SQL", "json": "", "": ""}.get(m.group(1), "")
            body = []
            i += 1
            while i < len(lines) and not lines[i].startswith("```"):
                body.append(lines[i])
                i += 1
            i += 1
            opt = "[language=%s]" % lang if lang else ""
            out += [r"\begin{lstlisting}" + opt]
            out += [ascii_listing(b) for b in body]
            out += [r"\end{lstlisting}", ""]
            continue

        # ---- blockquote box ----------------------------------------------
        if line.startswith(">"):
            block = []
            while i < len(lines) and (lines[i].startswith(">") or lines[i].strip() == ""):
                if lines[i].strip() == "":
                    # a blank line ends the quote unless the next is still quoted
                    if i + 1 < len(lines) and lines[i + 1].startswith(">"):
                        block.append("")
                        i += 1
                        continue
                    break
                block.append(re.sub(r"^>\s?", "", lines[i]))
                i += 1
            title, start = "", 0
            if block and block[0].startswith("### "):
                title = block[0][4:].replace("⚠", "").strip()
                start = 1
            env = box_for(block[0] if block else "")
            head = r"\begin{%s}[{%s}]" % (env, inline(title)) if title else r"\begin{%s}" % env
            out.append(head)
            out += render_body(block[start:])
            out += [r"\end{%s}" % env, ""]
            continue

        # ---- table ---------------------------------------------------------
        if line.startswith("|"):
            rows = []
            while i < len(lines) and lines[i].startswith("|"):
                rows.append(lines[i])
                i += 1
            if len(rows) >= 2:
                out += emit_table(rows) + [""]
            continue

        # ---- headings -------------------------------------------------------
        if line.startswith("## "):
            t = re.sub(r"^\d+\s*·\s*", "", line[3:]).strip()
            out += ["", r"\section{%s}\label{sec:%s}" % (inline(t), slug(t)), ""]
            i += 1
            continue
        if line.startswith("### "):
            t = line[4:].strip()
            out += ["", r"\subsection{%s}" % inline(t), ""]
            i += 1
            continue

        # ---- everything else --------------------------------------------
        chunk = []
        while i < len(lines) and not re.match(r"^(##|###|\||>|```)", lines[i]):
            chunk.append(lines[i])
            i += 1
        out += render_body(chunk)

    text = "\n".join(out)
    text = re.sub(r"\n{3,}", "\n\n", text)
    return banner(source_sha) + "\n" + text.strip() + "\n"


ITEM = re.compile(r"^(\s*)([-*]|\d+\.)\s+(.*)$")


def collect_list(chunk: list[str], i: int) -> tuple[list[str], int]:
    """Gather one list, folding wrapped continuation lines into their item.

    Markdown wraps a long item by indenting the rest of it. Matching only
    lines that *start* with a marker ended the list at the first wrap: the
    remainder of the item became a stray unindented paragraph, and the next
    item opened a brand new list -- which is why "1. CADENCE / 2. TEXTURE /
    3. PRECEDENCE" printed as 1, 1, 2 with prose falling out between them.
    """
    items: list[str] = []
    while i < len(chunk):
        m = ITEM.match(chunk[i])
        if m:
            items.append(m.group(3).strip())
            i += 1
            continue
        # Indented and non-blank directly under an item: that item continues.
        # A blank line ends the list, which is what markdown means by it.
        if items and chunk[i].strip() and chunk[i][:1].isspace():
            items[-1] += " " + chunk[i].strip()
            i += 1
            continue
        break
    return items, i


def render_body(chunk: list[str]) -> list[str]:
    """Paragraphs, bullet lists and numbered lists."""
    out: list[str] = []
    i = 0
    while i < len(chunk):
        line = chunk[i]
        m = ITEM.match(line)
        if m:
            env = "enumerate" if m.group(2).endswith(".") else "itemize"
            items, i = collect_list(chunk, i)
            out.append(r"\begin{%s}[leftmargin=*, itemsep=2pt]" % env)
            out += [r"\item " + inline(it) for it in items]
            out += [r"\end{%s}" % env, ""]
            continue

        # Tables. `convert` handles these at top level but this function renders
        # the inside of every blockquote box, and it used to have no table case
        # at all -- so a table in a warningbox fell through to the paragraph
        # collector, which joined its rows into one line. Three of them reached
        # the customer PDF as literal `| ordered ALONG seq | +0.7536 |` pipes.
        if line.lstrip().startswith("|"):
            rows = []
            while i < len(chunk) and chunk[i].lstrip().startswith("|"):
                rows.append(chunk[i].strip())
                i += 1
            if len(rows) >= 2:
                out += emit_table(rows, BOX_WIDTH_CM) + [""]
            continue
        if line.strip() == "" or set(line.strip()) == {"-"}:
            i += 1
            continue
        para = []
        while i < len(chunk) and chunk[i].strip() and not re.match(r"^\s*([-*]|\d+\.) ", chunk[i]):
            para.append(chunk[i].strip())
            i += 1
        if para:
            out += [inline(" ".join(para)), ""]
    return out


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--check", action="store_true",
                    help="exit 1 if the committed fragment is stale")
    args = ap.parse_args()

    generated = convert(SRC.read_text(encoding="utf-8"))

    if args.check:
        if not OUT.exists():
            print(f"MISSING: {OUT.relative_to(ROOT)} has never been generated")
            return 1
        current = OUT.read_text(encoding="utf-8")
        if current != generated:
            print(f"STALE: {OUT.relative_to(ROOT)} does not match docs/SHAPE_VERBS.md")
            print("Run: python scripts/shape_verbs_to_tex.py")
            return 1
        print(f"current: {OUT.relative_to(ROOT)} matches docs/SHAPE_VERBS.md")
        return 0

    OUT.parent.mkdir(parents=True, exist_ok=True)
    OUT.write_text(generated, encoding="utf-8")
    print(f"wrote {OUT.relative_to(ROOT)} ({len(generated.splitlines())} lines)")
    return 0


if __name__ == "__main__":
    sys.exit(main())
