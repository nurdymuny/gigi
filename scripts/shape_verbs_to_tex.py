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


def inline(s: str) -> str:
    """Markdown inline -> LaTeX, protecting code spans from prose escaping."""
    out, i = [], 0
    # `code` first so its contents never see bold/italic handling.
    for m in re.finditer(r"`([^`]+)`", s):
        out.append(("text", s[i:m.start()]))
        out.append(("code", m.group(1)))
        i = m.end()
    out.append(("text", s[i:]))

    parts = []
    for kind, chunk in out:
        if kind == "code":
            parts.append(r"\texttt{" + esc_code(chunk) + "}")
            continue
        # **bold** then *italic*, on escaped prose, using placeholders so the
        # escaping below cannot mangle the markers.
        chunk = re.sub(r"\*\*(.+?)\*\*", lambda m: "\x01" + m.group(1) + "\x02", chunk)
        chunk = re.sub(r"(?<!\*)\*([^*]+)\*(?!\*)", lambda m: "\x03" + m.group(1) + "\x02", chunk)
        chunk = esc_text(chunk)
        chunk = chunk.replace("\x01", r"\textbf{").replace("\x03", r"\emph{").replace("\x02", "}")
        parts.append(chunk)
    return "".join(parts)


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


def emit_table(rows: list[str]) -> list[str]:
    """A markdown pipe table -> booktabs tabular."""
    cells = [[c.strip() for c in r.strip().strip("|").split("|")] for r in rows]
    header, body = cells[0], cells[2:]          # cells[1] is the --- separator
    n = len(header)
    body = [r + [""] * (n - len(r)) if len(r) < n else r[:n] for r in body]

    widest = max((len(c) for r in [header] + body for c in r), default=0)
    if widest > 44 and n <= 3:
        widths = {2: ["5.4cm", "8.0cm"], 3: ["3.6cm", "4.6cm", "5.0cm"]}[n]
        spec = "".join("p{%s}" % w for w in widths)
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


def render_body(chunk: list[str]) -> list[str]:
    """Paragraphs, bullet lists and numbered lists."""
    out: list[str] = []
    i = 0
    while i < len(chunk):
        line = chunk[i]
        if re.match(r"^\s*[-*] ", line):
            out.append(r"\begin{itemize}[leftmargin=*, itemsep=2pt]")
            while i < len(chunk) and re.match(r"^\s*[-*] ", chunk[i]):
                out.append(r"\item " + inline(re.sub(r"^\s*[-*] ", "", chunk[i])))
                i += 1
            out += [r"\end{itemize}", ""]
            continue
        if re.match(r"^\s*\d+\. ", line):
            out.append(r"\begin{enumerate}[leftmargin=*, itemsep=2pt]")
            while i < len(chunk) and re.match(r"^\s*\d+\. ", chunk[i]):
                out.append(r"\item " + inline(re.sub(r"^\s*\d+\.\s*", "", chunk[i])))
                i += 1
            out += [r"\end{enumerate}", ""]
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
