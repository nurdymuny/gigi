#!/usr/bin/env python3
"""Generate the GIGI Builds exercise pages from the book's LaTeX source.

Usage:
    python3 scripts/build_exercise_pages.py <chapters_dir> <out_dir>

<chapters_dir> holds the book's chapterNN.tex files (NOT committed to this
repo — the book source stays private; only the exercises are published, which
is what the book itself invites readers to do). <out_dir> is typically
site/gigi-builds/exercises/.

Every chapter ends with `\\section{Exercises}` followed by exactly seven
`\\paragraph{EN.M --- Title.}` blocks whose bodies use \\textbf{Build.} /
\\textbf{Receipt.} / \\textbf{Bonus.} markers. This script converts that
LaTeX subset to HTML (KaTeX renders the inline math in the browser) and
emits index.html + ch01.html .. ch18.html styled to match the companion
site.
"""

import html
import re
import sys
from pathlib import Path

PARTS = [
    # (roman, title, chapters, css color var)
    ("I", "The Shape of Data", range(1, 4), "--blue"),
    ("II", "The Engine Room", range(4, 8), "--aqua"),
    ("III", "The Signals That Run While You Sleep", range(8, 11), "--yellow"),
    ("IV", "What the Loop Remembers", range(11, 13), "--green"),
    ("V", "The Theorem", range(13, 15), "--violet"),
    ("VI", "Gauge Encryption", range(15, 18), "--red"),
    ("VII", "Running the Engine", range(18, 19), "--magenta"),
]

REF_TEXT = {
    "ch:delegation": "Chapter 17",
    "ch:running": "Chapter 18",
}


def part_of(ch):
    for roman, title, rng, color in PARTS:
        if ch in rng:
            return roman, title, color
    raise ValueError(ch)


def read_braced(s, i):
    """s[i] == '{' -> (content, index after closing brace), brace-aware."""
    assert s[i] == "{"
    depth, j = 0, i
    while j < len(s):
        if s[j] == "{" and (j == 0 or s[j - 1] != "\\"):
            depth += 1
        elif s[j] == "}" and s[j - 1] != "\\":
            depth -= 1
            if depth == 0:
                return s[i + 1 : j], j + 1
        j += 1
    raise ValueError("unbalanced braces")


TEX_ESCAPES = [
    ("\\%", "%"), ("\\_", "_"), ("\\&", "&"), ("\\$", "$"), ("\\#", "#"),
    ("\\{", "{"), ("\\}", "}"), ("\\ldots", "\u2026"), ("\\S", "\u00a7"),
]


def tex_to_html(t):
    """Convert the exercise-section LaTeX subset to HTML."""
    maths, codes = [], []

    # 1. protect inline math so text rules don't touch it
    def stash_math(m):
        maths.append(m.group(1))
        return f"\x00M{len(maths) - 1}\x00"

    t = re.sub(r"\$([^$]+)\$", stash_math, t)

    # 2. protect verbatim/code content: typography (smart dashes/quotes)
    # must NOT run inside it -- `--release` has to survive copy-paste, and
    # the typeset book keeps literal `--` in typewriter font too.
    def stash_code(content):
        codes.append(content)
        return f"\x00C{len(codes) - 1}\x00"

    t = re.sub(r"\\verb(.)(.*?)\1", lambda m: stash_code(m.group(2)), t)

    out, i = [], 0
    while True:
        j = t.find("\\texttt{", i)
        if j < 0:
            out.append(t[i:])
            break
        out.append(t[i:j])
        inner, k = read_braced(t, j + len("\\texttt"))
        for a, b in TEX_ESCAPES:
            inner = inner.replace(a, b)
        out.append(stash_code(inner))
        i = k
    t = "".join(out)

    # 3. structural commands
    t = t.replace("\\sloppy", "")
    t = re.sub(r"\\label\{[^}]*\}", "", t)
    t = re.sub(r"\\v\{C\}", "\u010c", t)
    t = re.sub(r"\\v\{c\}", "\u010d", t)
    t = re.sub(
        r"(?:Chapter|Figure|Table)?~?\\ref\{([^}]*)\}",
        lambda m: REF_TEXT.get(m.group(1), "the book"),
        t,
    )

    # 4. brace-aware inline commands
    def convert_cmd(text, cmd, open_tag, close_tag):
        res, i = [], 0
        pat = "\\" + cmd + "{"
        while True:
            j = text.find(pat, i)
            if j < 0:
                res.append(text[i:])
                return "".join(res)
            res.append(text[i:j])
            inner, k = read_braced(text, j + len(pat) - 1)
            res.append(open_tag + inner + close_tag)
            i = k

    t = convert_cmd(t, "textbf", "\x01strong\x02", "\x01/strong\x02")
    t = convert_cmd(t, "emph", "\x01em\x02", "\x01/em\x02")
    t = convert_cmd(t, "textit", "\x01em\x02", "\x01/em\x02")

    # 5. escapes and typography on the remaining prose
    for a, b in TEX_ESCAPES + [("~", " "), ("``", "\u201c"), ("''", "\u201d")]:
        t = t.replace(a, b)
    t = re.sub(r"(?<!-)---(?!-)", " \u2014 ", t)
    t = re.sub(r"(?<!-)--(?!-)", "\u2013", t)

    t = html.escape(t, quote=False)

    # 6. restore tags, code spans, and math
    t = t.replace("\x01", "<").replace("\x02", ">")
    t = re.sub(
        r"\x00C(\d+)\x00",
        lambda m: "<code>" + html.escape(codes[int(m.group(1))], quote=False) + "</code>",
        t,
    )
    t = re.sub(
        r"\x00M(\d+)\x00",
        lambda m: r"\(" + html.escape(maths[int(m.group(1))], quote=False) + r"\)",
        t,
    )
    return t.strip()



def parse_chapter(path):
    src = path.read_text()
    title = re.search(r"\\chapter\{([^}]*)\}", src).group(1)
    ex = src[src.index("\\section{Exercises}"):]
    ex = ex[ex.index("}") + 1 :]

    # split into intro + \paragraph blocks (brace-aware titles)
    blocks, positions = [], []
    i = 0
    while True:
        j = ex.find("\\paragraph{", i)
        if j < 0:
            break
        positions.append(j)
        i = j + 1
    intro = ex[: positions[0]] if positions else ex
    for n, j in enumerate(positions):
        head, k = read_braced(ex, j + len("\\paragraph"))
        body = ex[k : positions[n + 1] if n + 1 < len(positions) else len(ex)]
        m = re.match(r"(E\d+\.\d+)\s*---\s*(.*?)\.?\s*$", head)
        blocks.append((m.group(1), m.group(2), body.strip()))
    return title, intro.strip(), blocks


def render_body(body):
    """Split a body on Build./Receipt./Bonus. markers into labeled segments."""
    parts = re.split(r"\\textbf\{(Build|Receipt|Bonus)\.\}", body)
    out = []
    if parts[0].strip():
        out.append(f'<p>{tex_to_html(parts[0])}</p>')
    for label, seg in zip(parts[1::2], parts[2::2]):
        out.append(
            f'<div class="seg seg-{label.lower()}"><span class="seglabel">{label}</span>'
            f"<p>{tex_to_html(seg)}</p></div>"
        )
    return "\n".join(out)


CSS = """
  :root {
    --ink: #17161a; --ink-2: #52514e; --ink-3: #807f76;
    --paper: #faf7f0; --paper-2: #f1ecdf; --night: #101426; --line: #dcd5c4;
    --blue: #2a78d6; --aqua: #1baf7a; --yellow: #eda100; --green: #008300;
    --violet: #4a3aa7; --red: #e34948; --magenta: #d55181;
  }
  * { box-sizing: border-box; }
  body { margin: 0; background: var(--paper); color: var(--ink);
    font: 16px/1.65 system-ui, -apple-system, "Segoe UI", sans-serif; }
  a { color: var(--blue); }
  a:focus-visible { outline: 3px solid var(--yellow); outline-offset: 2px; }
  code { font-family: ui-monospace, "Cascadia Code", Menlo, Consolas, monospace;
    background: var(--paper-2); border-radius: 5px; padding: 1px 6px; font-size: 0.86em;
    overflow-wrap: anywhere; }
  .wrap { max-width: 860px; margin: 0 auto; padding: 0 22px; }
  nav { position: sticky; top: 0; z-index: 50; background: rgba(16,20,38,0.92);
    backdrop-filter: blur(8px); }
  nav .wrap { display: flex; align-items: center; gap: 18px; height: 54px; max-width: 1080px; }
  .brand { color: #fff; font-weight: 800; text-decoration: none; font-size: 15px;
    display: flex; align-items: center; gap: 9px; }
  .brand .dot { width: 12px; height: 12px; border-radius: 50%;
    background: radial-gradient(circle at 35% 35%, #fff, #b9c4e8 45%, #6c7dbb); }
  nav .crumbs { color: #c9d1ef; font-size: 13.5px; margin-left: auto; }
  nav .crumbs a { color: #c9d1ef; text-decoration: none; }
  nav .crumbs a:hover { color: #fff; }
  header.page { padding: 46px 0 10px; }
  .eyebrow { text-transform: uppercase; letter-spacing: 0.14em; font-size: 12px;
    font-weight: 700; color: var(--ink-3); }
  h1 { font-size: clamp(26px, 4vw, 38px); line-height: 1.15; letter-spacing: -0.015em;
    margin: 8px 0 6px; }
  .partline { display: inline-flex; align-items: center; gap: 8px; font-size: 13.5px;
    color: var(--ink-2); font-weight: 650; }
  .partline .sw { width: 12px; height: 12px; border-radius: 4px; }
  .intro { color: var(--ink-2); font-size: 16.5px; max-width: 46em; }
  .ex { background: #fff; border: 1px solid var(--line); border-radius: 12px;
    padding: 22px 26px; margin: 18px 0; }
  .ex h2 { display: flex; align-items: baseline; gap: 12px; font-size: 19px;
    margin: 0 0 10px; line-height: 1.3; }
  .ex .eid { font-family: ui-monospace, Menlo, Consolas, monospace; font-size: 13px;
    font-weight: 700; color: #fff; border-radius: 7px; padding: 3px 9px; flex: 0 0 auto; }
  .seg { display: grid; grid-template-columns: 76px 1fr; gap: 12px; margin-top: 12px; }
  .seg p { margin: 0; font-size: 15px; }
  .seglabel { font-size: 11.5px; font-weight: 800; text-transform: uppercase;
    letter-spacing: 0.08em; padding-top: 3px; }
  .seg-build .seglabel { color: var(--blue); }
  .seg-receipt .seglabel { color: var(--aqua); }
  .seg-bonus .seglabel { color: var(--ink-3); }
  .pager { display: flex; justify-content: space-between; gap: 14px; margin: 40px 0 12px; }
  .pager a { text-decoration: none; font-weight: 650; font-size: 14.5px;
    border: 1px solid var(--line); background: #fff; border-radius: 10px; padding: 10px 16px; }
  footer { color: var(--ink-3); font-size: 13px; padding: 26px 0 44px; }
  /* hub */
  .hub { display: grid; grid-template-columns: repeat(auto-fill, minmax(340px, 1fr));
    gap: 14px; margin: 26px 0 40px; }
  .hub a { display: block; background: #fff; border: 1px solid var(--line);
    border-left-width: 5px; border-radius: 10px; padding: 15px 18px;
    text-decoration: none; color: var(--ink); }
  .hub a:hover { border-color: var(--ink-3); }
  .hub .n { font-family: Georgia, serif; color: var(--ink-3); font-size: 13px; }
  .hub h3 { margin: 2px 0 4px; font-size: 16.5px; line-height: 1.3; }
  .hub .range { color: var(--ink-2); font-size: 13.5px; }
  @media (max-width: 620px) { .seg { grid-template-columns: 1fr; gap: 2px; } }
  /* workbook layer */
  .exwork { display: flex; gap: 12px; align-items: center; margin-top: 16px;
    border-top: 1px dashed var(--line); padding-top: 12px; flex-wrap: wrap; }
  .exwork label { display: flex; gap: 7px; align-items: center; font-size: 13px;
    font-weight: 700; color: var(--ink-2); cursor: pointer; white-space: nowrap; }
  .exwork input[type=checkbox] { width: 17px; height: 17px; accent-color: var(--aqua); }
  .exwork .receipt { flex: 1; min-width: 220px; border: 1px solid var(--line);
    border-radius: 8px; padding: 8px 10px; font: 13px ui-monospace, Menlo, Consolas, monospace;
    background: var(--paper); color: var(--ink); }
  .exwork .receipt:focus { outline: 2px solid var(--blue); }
  .ex.is-done { border-left: 5px solid var(--aqua); }
  .ex.is-done .eid { filter: saturate(0.6); }
  .progress { font-size: 13.5px; color: var(--ink-2); font-weight: 650; margin-top: 6px; }
  .progress .bar { display: inline-block; width: 120px; height: 8px; background: var(--paper-2);
    border: 1px solid var(--line); border-radius: 99px; vertical-align: middle; margin: 0 8px; }
  .progress .fill { display: block; height: 100%; border-radius: 99px; background: var(--aqua); }
  code { cursor: copy; }
  #toast { position: fixed; bottom: 22px; left: 50%; transform: translateX(-50%);
    background: var(--night); color: #fff; border-radius: 99px; padding: 8px 18px;
    font-size: 13px; opacity: 0; transition: opacity 0.25s; pointer-events: none; z-index: 99; }
  #toast.show { opacity: 1; }
  /* live console */
  .console { background: var(--night); color: #d9e0fa; border-radius: 12px;
    padding: 4px 20px 6px; margin: 22px 0; }
  .console summary { cursor: pointer; font-weight: 700; font-size: 14.5px; padding: 12px 0; }
  .console .row { display: flex; gap: 10px; margin: 10px 0; flex-wrap: wrap; }
  .console input, .console textarea { background: #0b0e1c; color: #d9e0fa;
    border: 1px solid #38406b; border-radius: 8px; padding: 9px 11px;
    font: 13px ui-monospace, Menlo, Consolas, monospace; }
  .console input { flex: 1; min-width: 240px; }
  .console textarea { width: 100%; min-height: 64px; resize: vertical; }
  .console button { background: var(--yellow); color: #221a00; font-weight: 750;
    border: 0; border-radius: 8px; padding: 9px 18px; cursor: pointer; font-size: 13.5px; }
  .console .chip { background: transparent; color: #c9d1ef; border: 1px solid #38406b; }
  .console pre { background: #0b0e1c; border: 1px solid #38406b; border-radius: 8px;
    padding: 12px; overflow: auto; max-height: 300px; font-size: 12.5px; white-space: pre-wrap; }
  .console .hint { color: #8f9bcd; font-size: 12px; margin: 6px 0 12px; }
  /* playground */
  .playground { background: #fff; border: 2px solid var(--blue); border-radius: 12px;
    padding: 20px 24px; margin: 22px 0; }
  .playground h2 { margin: 0 0 4px; font-size: 19px; }
  .playground .sub { color: var(--ink-2); font-size: 14px; margin: 0 0 14px; }
  .playground .row { display: flex; gap: 10px; flex-wrap: wrap; align-items: center; margin: 10px 0; }
  .playground button { background: var(--blue); color: #fff; font-weight: 700; border: 0;
    border-radius: 8px; padding: 9px 16px; cursor: pointer; font-size: 13.5px; }
  .playground button.alt { background: var(--paper-2); color: var(--ink); border: 1px solid var(--line); }
  .playground input[type=number] { width: 110px; border: 1px solid var(--line); border-radius: 8px;
    padding: 8px 10px; font: 13.5px ui-monospace, Menlo, monospace; }
  .stats { display: grid; grid-template-columns: repeat(auto-fit, minmax(120px, 1fr)); gap: 8px 18px;
    background: var(--paper-2); border-radius: 10px; padding: 14px 18px; margin: 12px 0; }
  .stats .v { font-size: 17px; font-weight: 750; font-variant-numeric: tabular-nums; }
  .stats .k { font-size: 11px; color: var(--ink-3); text-transform: uppercase; letter-spacing: 0.06em; }
  .targets { font-size: 13.5px; }
  .targets li { margin: 4px 0; }
  .hit { color: var(--green); font-weight: 750; }
"""

KATEX = """
<link rel="stylesheet" href="https://cdn.jsdelivr.net/npm/katex@0.16.9/dist/katex.min.css">
<script defer src="https://cdn.jsdelivr.net/npm/katex@0.16.9/dist/katex.min.js"></script>
<script defer src="https://cdn.jsdelivr.net/npm/katex@0.16.9/dist/contrib/auto-render.min.js"
  onload="renderMathInElement(document.body, {delimiters: [{left: '\\\\(', right: '\\\\)', display: false}]});"></script>
"""



# Per-chapter interactive playgrounds, injected below the console.
# Ch 1: the curvature odometer from E1.1/E1.6, running the engine's own
# math in the browser — Welford stats (src/bundle.rs:758-771) and
# per-record kappa = mean_f |v - mu_before| / range (src/bundle.rs:933-967,
# declared-range normalisation per effective_range).
PLAYGROUNDS = {
    1: """
  <div class="playground">
    <h2>🌡️ Try it here first — the curvature odometer</h2>
    <p class="sub">This is E1.1 running in your browser with the engine's exact math:
    Welford running stats + per-record \\(\\kappa = |v-\\mu|/R\\) with declared range
    \\(R\\). Plant the plain, drop the spike, watch the odometer notice. Then go
    reproduce it in Rust for the real receipt.</p>
    <div class="row">
      <button id="pg-plain">Insert 150 plain readings [20.0–20.6]</button>
      <button id="pg-spike">Insert s151 = 500.0</button>
      <label>custom value <input type="number" id="pg-val" value="42" step="any"></label>
      <button class="alt" id="pg-insert">insert</button>
      <button class="alt" id="pg-reset">reset</button>
    </div>
    <div class="stats">
      <div><div class="v" id="pg-n">0</div><div class="k">records</div></div>
      <div><div class="v" id="pg-mean">–</div><div class="k">mean (Welford)</div></div>
      <div><div class="v" id="pg-sigma">–</div><div class="k">σ</div></div>
      <div><div class="v" id="pg-klast">–</div><div class="k">κ at insert (prior stats)</div></div>
      <div><div class="v" id="pg-kspike">–</div><div class="k">κ of s151 now</div></div>
      <div><div class="v" id="pg-ktyp">–</div><div class="k">κ of a typical reading (20.3)</div></div>
      <div><div class="v" id="pg-flag">0</div><div class="k">flagged at 3σ</div></div>
    </div>
    <ul class="targets">
      <li id="t-spike">◻ book receipt: κ(s151) = 4.7653 (BLD-CH1-WORKED-KAPPA)</li>
      <li id="t-typ">◻ book receipt: typical reading's κ rises to ≈ 0.0317 after the spike</li>
    </ul>
  </div>
  <script>
  (function () {
    var R = 100.0;   // declared RANGE, per the worked example
    var st, vals, lastK;
    function reset() { st = { n: 0, mean: 0, m2: 0 }; vals = []; lastK = null; draw(); }
    function kappa(v) { return st.n < 2 ? 0 : Math.abs(v - st.mean) / R; }
    function insert(v) {
      lastK = kappa(v);                       // stats BEFORE the record
      st.n += 1;
      if (st.n === 1) { st.mean = v; st.m2 = 0; }
      else { var d = v - st.mean; st.mean += d / st.n; st.m2 += d * (v - st.mean); }
      vals.push(v);
      draw();
    }
    function draw() {
      var sigma = st.n > 1 ? Math.sqrt(st.m2 / (st.n - 1)) : 0;
      var flagged = st.n > 1 ? vals.filter(function (v) {
        return Math.abs(v - st.mean) > 3 * sigma; }).length : 0;
      function set(id, txt) { document.getElementById(id).textContent = txt; }
      set("pg-n", st.n);
      set("pg-mean", st.n ? st.mean.toFixed(4) : "–");
      set("pg-sigma", st.n > 1 ? sigma.toFixed(4) : "–");
      set("pg-klast", lastK === null ? "–" : lastK.toFixed(6));
      var kSpike = (st.n > 1 && vals.indexOf(500.0) >= 0) ? kappa(500.0) : null;
      set("pg-kspike", kSpike === null ? "–" : kSpike.toFixed(6));
      set("pg-ktyp", st.n > 1 ? kappa(20.3).toFixed(6) : "–");
      set("pg-flag", flagged);
      var tS = document.getElementById("t-spike"), tT = document.getElementById("t-typ");
      if (kSpike !== null && Math.abs(kSpike - 4.7653) < 0.01) {
        tS.classList.add("hit"); tS.textContent = "✓ book receipt hit: κ(s151) ≈ " + kSpike.toFixed(4) + " (book: 4.7653) — the engine recomputes κ against current stats when you SECTION the record";
      }
      if (st.n > 150 && Math.abs(kappa(20.3) - 0.0317) < 0.002) {
        tT.classList.add("hit"); tT.textContent = "✓ book receipt hit: typical κ ≈ " + kappa(20.3).toFixed(6) + " (book: 0.031728)";
      }
    }
    document.getElementById("pg-plain").addEventListener("click", function () {
      for (var i = 0; i < 150; i++) insert(20.0 + 0.6 * i / 149);
    });
    document.getElementById("pg-spike").addEventListener("click", function () { insert(500.0); });
    document.getElementById("pg-insert").addEventListener("click", function () {
      insert(parseFloat(document.getElementById("pg-val").value) || 0);
    });
    document.getElementById("pg-reset").addEventListener("click", reset);
    reset();
  })();
  </script>
""",
}


def page(title_text, body_html, crumbs):
    return f"""<!DOCTYPE html>
<html lang="en">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<title>{html.escape(title_text)} — GIGI Builds exercises</title>
<style>{CSS}</style>
{KATEX}
</head>
<body>
<nav aria-label="Site">
  <div class="wrap">
    <a class="brand" href="../index.html"><span class="dot" aria-hidden="true"></span>GIGI&nbsp;Builds</a>
    <span class="crumbs">{crumbs}</span>
  </div>
</nav>
{body_html}
<div id="toast" role="status"></div>
<script>
(function () {{
  var KEY = "gb-ex";
  var store = {{}};
  try {{ store = JSON.parse(localStorage.getItem(KEY) || "{{}}"); }} catch (e) {{}}
  function save() {{ localStorage.setItem(KEY, JSON.stringify(store)); }}
  function toast(msg) {{
    var t = document.getElementById("toast");
    t.textContent = msg; t.classList.add("show");
    clearTimeout(t._h); t._h = setTimeout(function () {{ t.classList.remove("show"); }}, 1400);
  }}
  // per-exercise done + receipt
  document.querySelectorAll("[data-done]").forEach(function (cb) {{
    var id = cb.getAttribute("data-done");
    var st = store[id] || {{}};
    cb.checked = !!st.done;
    cb.closest(".ex") && cb.closest(".ex").classList.toggle("is-done", !!st.done);
    cb.addEventListener("change", function () {{
      store[id] = store[id] || {{}}; store[id].done = cb.checked; save();
      cb.closest(".ex").classList.toggle("is-done", cb.checked);
      updateProgress();
    }});
  }});
  document.querySelectorAll("[data-receipt]").forEach(function (inp) {{
    var id = inp.getAttribute("data-receipt");
    var st = store[id] || {{}};
    if (st.receipt) inp.value = st.receipt;
    inp.addEventListener("change", function () {{
      store[id] = store[id] || {{}}; store[id].receipt = inp.value; save();
    }});
  }});
  function updateProgress() {{
    document.querySelectorAll("[data-progress]").forEach(function (el) {{
      var ids = el.getAttribute("data-progress").split(",");
      var done = ids.filter(function (id) {{ return store[id] && store[id].done; }}).length;
      el.innerHTML = done + "/" + ids.length + " done" +
        '<span class="bar"><span class="fill" style="width:' + (100 * done / ids.length) + '%"></span></span>';
    }});
  }}
  updateProgress();
  // click-to-copy on code chips
  document.querySelectorAll("code").forEach(function (c) {{
    c.title = "click to copy";
    c.addEventListener("click", function () {{
      navigator.clipboard && navigator.clipboard.writeText(c.textContent).then(
        function () {{ toast("copied"); }});
    }});
  }});
  // live console
  document.querySelectorAll(".console").forEach(function (box) {{
    var run = box.querySelector(".run"), ep = box.querySelector(".ep"),
        q = box.querySelector("textarea"), out = box.querySelector("pre");
    var saved = localStorage.getItem("gb-endpoint");
    if (saved) ep.value = saved;
    box.querySelectorAll(".chip").forEach(function (ch) {{
      ch.addEventListener("click", function () {{ q.value = ch.getAttribute("data-q"); }});
    }});
    run.addEventListener("click", function () {{
      var base = ep.value.replace(/\/+$/, "");
      localStorage.setItem("gb-endpoint", ep.value);
      out.textContent = "running…";
      fetch(base + "/v1/gql", {{ method: "POST", headers: {{ "Content-Type": "application/json" }},
        body: JSON.stringify({{ query: q.value }}) }})
        .then(function (r) {{ return r.text(); }})
        .then(function (txt) {{
          try {{ out.textContent = JSON.stringify(JSON.parse(txt), null, 2); }}
          catch (e) {{ out.textContent = txt; }}
        }})
        .catch(function (e) {{
          out.textContent = "✗ " + e + "\\n\\nIf you are pointing at your own engine, start it with GIGI_CORS_ORIGIN=* (dev only). The public instance allows browser calls but is read-only.";
        }});
    }});
  }});
}})();
</script>
<footer>
  <div class="wrap">From <em>GIGI Builds</em> (2026) · © Bee Rosa Davis · engine at
  <a href="https://github.com/nurdymuny/gigi">github.com/nurdymuny/gigi</a> ·
  questions → <a href="mailto:bee_davis@alumni.brown.edu">bee_davis@alumni.brown.edu</a></div>
</footer>
</body>
</html>
"""


def main():
    chapters_dir, out_dir = Path(sys.argv[1]), Path(sys.argv[2])
    out_dir.mkdir(parents=True, exist_ok=True)
    chapters = {}
    for ch in range(1, 19):
        title, intro, blocks = parse_chapter(chapters_dir / f"chapter{ch:02d}.tex")
        chapters[ch] = (title, intro, blocks)

    # per-chapter pages
    for ch, (title, intro, blocks) in chapters.items():
        roman, ptitle, color = part_of(ch)
        cards = []
        for eid, etitle, body in blocks:
            cards.append(
                f'<article class="ex" id="{eid.lower().replace(".", "-")}">'
                f'<h2><span class="eid" style="background: var({color})">{eid}</span>'
                f"{tex_to_html(etitle)}</h2>{render_body(body)}"
                f'<div class="exwork">'
                f'<label><input type="checkbox" data-done="{eid}"> done</label>'
                f'<input class="receipt" data-receipt="{eid}" '
                f'placeholder="paste your receipt — the number, the test line, the error…"></div>'
                f"</article>"
            )
        prev_link = (
            f'<a href="ch{ch-1:02d}.html">← Chapter {ch-1} exercises</a>'
            if ch > 1 else "<span></span>"
        )
        next_link = (
            f'<a href="ch{ch+1:02d}.html">Chapter {ch+1} exercises →</a>'
            if ch < 18 else '<a href="index.html">All chapters →</a>'
        )
        eids = ",".join(b[0] for b in blocks)
        console = f"""
  <details class="console">
    <summary>⚡ Run it live — GQL console</summary>
    <p class="hint">Point at the public read-only instance, or your own engine
    (start it with <code style="cursor:text">GIGI_CORS_ORIGIN=*</code> for browser access — dev only).</p>
    <div class="row"><input class="ep" value="https://gigi-stream.fly.dev" aria-label="engine endpoint">
    <button class="run">Run</button></div>
    <textarea aria-label="GQL statement">SHOW BUNDLES;</textarea>
    <div class="row">
      <button class="chip" data-q="SHOW BUNDLES;">SHOW BUNDLES</button>
      <button class="chip" data-q="HEALTH sensors;">HEALTH</button>
      <button class="chip" data-q="SECTION sensors AT id='s1';">SECTION AT</button>
      <button class="chip" data-q="INTEGRATE sensors OVER city MEASURE count(*), min(temp);">INTEGRATE</button>
    </div>
    <pre aria-live="polite">—</pre>
  </details>"""
        body_html = f"""
<header class="page">
  <div class="wrap">
    <p class="eyebrow"><a href="index.html" style="text-decoration:none; color:inherit;">Exercises</a> · Chapter {ch}</p>
    <h1>{tex_to_html(title)}</h1>
    <p class="partline"><span class="sw" style="background: var({color})"></span>Part {roman} — {html.escape(ptitle)} · seven builds, seven receipts</p>
    <p class="progress" data-progress="{eids}"></p>
  </div>
</header>
<main class="wrap">
  <p class="intro">{tex_to_html(intro)}</p>
  {console}
  {PLAYGROUNDS.get(ch, "")}
  {''.join(cards)}
  <div class="pager">{prev_link}{next_link}</div>
</main>"""
        crumbs = f'<a href="index.html">exercises</a> · chapter {ch:02d}'
        (out_dir / f"ch{ch:02d}.html").write_text(page(f"Chapter {ch}: {title}", body_html, crumbs))

    # hub page
    cards = []
    for ch, (title, intro, blocks) in chapters.items():
        roman, ptitle, color = part_of(ch)
        eids = ",".join(b[0] for b in blocks)
        cards.append(
            f'<a href="ch{ch:02d}.html" style="border-left-color: var({color})">'
            f'<span class="n">Part {roman} · Chapter {ch}</span>'
            f"<h3>{tex_to_html(title)}</h3>"
            f'<span class="range">{blocks[0][0]}–{blocks[-1][0]} · 7 exercises · '
            f'<span class="progress" style="margin:0" data-progress="{eids}"></span></span></a>'
        )
    body_html = f"""
<header class="page">
  <div class="wrap">
    <p class="eyebrow">GIGI Builds · companion site</p>
    <h1>The Exercises</h1>
    <p class="intro">Every chapter of <em>GIGI Builds</em> ends the same way: seven exercises,
    each one a build with a receipt — a number, a passing test, or an error message you
    reproduce against the ones printed in the book. All 126 are here. The engine is free for
    exactly this: clone it, run them, break things, write me.</p>
  </div>
</header>
<main class="wrap">
  <div class="hub">{''.join(cards)}</div>
</main>"""
    (out_dir / "index.html").write_text(page("All exercises", body_html, "exercises"))
    print(f"wrote {len(chapters)} chapter pages + hub to {out_dir}")


if __name__ == "__main__":
    main()
