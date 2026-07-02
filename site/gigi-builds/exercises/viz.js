/* GIGI Builds — per-chapter visual instruments.
 *
 * Standalone add-on: each chapter page loads this file with
 *   <script defer src="viz.js" data-ch="NN"></script>
 * and gets one interactive canvas instrument mounted above its first
 * exercise. Nothing here reaches into the page's own scripts (workbook,
 * console, ch1 playground) — the panel builds its own DOM, injects its
 * own CSS, and computes its own data, so regenerating or hand-editing
 * the pages never collides with it.
 *
 * The instruments build on one another WITHIN each Part of the book and
 * reset at Part boundaries:
 *   Part I   (ch1–3)  the shape: readings → stalks over a base → addresses
 *   Part II  (ch4–7)  one record's journey through the machine
 *   Part III (ch8–10) signals: vitals → the room behind λ₁ → the wire
 *   Part IV  (ch11–12) transport on a curved patch: holonomy → torsion
 *   Part V   (ch13–14) receipts: shard equality → honest error bars
 *   Part VI  (ch15–17) one cloud, many frames: gauge → verify → delegate
 *   Part VII (ch18)   the keeper's console
 *
 * All math is the engine's math where the engine has one (Welford, κ=|v−μ|/R
 * with R=100 as in the Chapter 1 worked example, Laplacian λ₁, blocked
 * errors); everything is deterministic (seeded PRNG), offline, and
 * dependency-free. The only network touch is the click-gated live HEALTH
 * ping in Chapter 18.
 */
(function () {
  "use strict";
  var script = document.currentScript;
  var CH = script ? parseInt(script.getAttribute("data-ch"), 10) : 0;
  if (!CH) return;

  var PART_OF = { 1: 1, 2: 1, 3: 1, 4: 2, 5: 2, 6: 2, 7: 2, 8: 3, 9: 3, 10: 3,
    11: 4, 12: 4, 13: 5, 14: 5, 15: 6, 16: 6, 17: 6, 18: 7 };
  var PART_COLOR = { 1: "#2a78d6", 2: "#1baf7a", 3: "#c98500", 4: "#008300",
    5: "#4a3aa7", 6: "#e34948", 7: "#d55181" };
  var PART_NAME = { 1: "I", 2: "II", 3: "III", 4: "IV", 5: "V", 6: "VI", 7: "VII" };
  // shared canvas ink
  var INK = "#17161a", INK2 = "#52514e", INK3 = "#807f76",
      PAPER2 = "#f1ecdf", LINE = "#dcd5c4",
      BLUE = "#2a78d6", AQUA = "#1baf7a", YELLOW = "#c98500",
      GREEN = "#008300", VIOLET = "#4a3aa7", RED = "#e34948", MAGENTA = "#d55181";

  /* ---------- tiny toolkit ---------- */

  function mulberry32(seed) {
    var a = seed >>> 0;
    return function () {
      a |= 0; a = (a + 0x6D2B79F5) | 0;
      var t = Math.imul(a ^ (a >>> 15), 1 | a);
      t = (t + Math.imul(t ^ (t >>> 7), 61 | t)) ^ t;
      return ((t ^ (t >>> 14)) >>> 0) / 4294967296;
    };
  }

  function el(tag, cls, txt) {
    var e = document.createElement(tag);
    if (cls) e.className = cls;
    if (txt !== undefined) e.textContent = txt;
    return e;
  }

  function makeCanvas(w, h) {
    var dpr = Math.min(2, window.devicePixelRatio || 1);
    var cv = el("canvas", "gbviz-canvas");
    cv.width = w * dpr; cv.height = h * dpr;
    cv.style.aspectRatio = w + " / " + h;
    var ctx = cv.getContext("2d");
    ctx.scale(dpr, dpr);
    return { cv: cv, ctx: ctx, W: w, H: h };
  }

  function button(label, onClick, alt) {
    var b = el("button", alt ? "gbviz-btn alt" : "gbviz-btn", label);
    b.type = "button";
    b.addEventListener("click", onClick);
    return b;
  }

  function slider(panel, label, min, max, step, val, fmt, onInput) {
    var wrap = el("label", "gbviz-slider");
    var span = el("span", null, label + " ");
    var out = el("strong", null, fmt(val));
    span.appendChild(out);
    var inp = el("input");
    inp.type = "range"; inp.min = min; inp.max = max; inp.step = step; inp.value = val;
    inp.addEventListener("input", function () {
      var v = parseFloat(inp.value);
      out.textContent = fmt(v);
      onInput(v);
    });
    wrap.appendChild(span); wrap.appendChild(inp);
    return { el: wrap, input: inp, out: out };
  }

  function readout(label) {
    var box = el("div", "gbviz-stat");
    var v = el("div", "v", "–"), k = el("div", "k", label);
    box.appendChild(v); box.appendChild(k);
    return { el: box, set: function (txt) { v.textContent = txt; },
             mark: function (on) { v.classList.toggle("hot", !!on); } };
  }

  function row(cls) { return el("div", "gbviz-row" + (cls ? " " + cls : "")); }

  function note(txt) { var p = el("p", "gbviz-note"); p.innerHTML = txt; return p; }

  // rAF loop that pauses when the panel scrolls out of view or tab hides.
  function makeLoop(target, tick) {
    var running = false, visible = true, raf = 0, t = 0;
    function frame() {
      raf = 0;
      if (!running || !visible || document.hidden) return;
      tick(t++);
      raf = requestAnimationFrame(frame);
    }
    if ("IntersectionObserver" in window) {
      new IntersectionObserver(function (entries) {
        visible = entries[0].isIntersecting;
        if (visible && running && !raf) raf = requestAnimationFrame(frame);
      }).observe(target);
    }
    document.addEventListener("visibilitychange", function () {
      if (!document.hidden && running && !raf) raf = requestAnimationFrame(frame);
    });
    return {
      start: function () { if (!running) { running = true; if (!raf) raf = requestAnimationFrame(frame); } },
      stop: function () { running = false; },
      get running() { return running; }
    };
  }

  /* ---------- panel chrome ---------- */

  var CSS = "" +
    ".gbviz{background:#fff;border:1px solid " + LINE + ";border-left:5px solid var(--gbviz-accent);" +
      "border-radius:12px;padding:22px 26px;margin:22px 0}" +
    ".gbviz h2{margin:0 0 4px;font-size:19px;display:flex;align-items:baseline;gap:10px;flex-wrap:wrap}" +
    ".gbviz .gbviz-chip{font-size:11px;font-weight:800;letter-spacing:.08em;text-transform:uppercase;" +
      "color:#fff;background:var(--gbviz-accent);border-radius:7px;padding:3px 9px;flex:0 0 auto}" +
    ".gbviz .gbviz-sub{color:" + INK2 + ";font-size:14px;margin:0 0 14px;max-width:60em}" +
    ".gbviz-row{display:flex;gap:10px;flex-wrap:wrap;align-items:center;margin:10px 0}" +
    ".gbviz-btn{background:var(--gbviz-accent);color:#fff;font-weight:700;border:0;border-radius:9px;" +
      "padding:9px 14px;font-size:14px;cursor:pointer}" +
    ".gbviz-btn:hover{filter:brightness(1.08)}" +
    ".gbviz-btn:disabled{opacity:.45;cursor:default}" +
    ".gbviz-btn.alt{background:" + PAPER2 + ";color:" + INK + ";border:1px solid " + LINE + "}" +
    ".gbviz-btn:focus-visible,.gbviz input:focus-visible{outline:3px solid #eda100;outline-offset:2px}" +
    ".gbviz-canvas{display:block;width:100%;height:auto;border:1px solid " + LINE + ";" +
      "border-radius:10px;background:#fdfcf8;touch-action:none}" +
    ".gbviz-slider{display:inline-flex;flex-direction:column;gap:2px;font-size:13px;color:" + INK2 + ";min-width:170px}" +
    ".gbviz-slider strong{color:" + INK + ";font-variant-numeric:tabular-nums}" +
    ".gbviz-slider input{width:100%}" +
    ".gbviz-stats{display:flex;gap:22px;flex-wrap:wrap;margin:12px 0 2px}" +
    ".gbviz-stat .v{font-family:ui-monospace,Menlo,Consolas,monospace;font-size:17px;font-weight:700;" +
      "font-variant-numeric:tabular-nums}" +
    ".gbviz-stat .v.hot{color:" + RED + "}" +
    ".gbviz-stat .k{font-size:11.5px;color:" + INK3 + ";text-transform:uppercase;letter-spacing:.06em}" +
    ".gbviz-note{color:" + INK2 + ";font-size:13px;margin:10px 0 0;max-width:60em}" +
    ".gbviz-note code{font-size:.9em}";

  function mountPanel(meta) {
    var style = el("style"); style.textContent = CSS;
    document.head.appendChild(style);
    var part = PART_OF[CH], accent = PART_COLOR[part];
    var panel = el("section", "gbviz");
    panel.style.setProperty("--gbviz-accent", accent);
    panel.setAttribute("aria-label", "Chapter " + CH + " visual instrument");
    var h = el("h2");
    var chip = el("span", "gbviz-chip", "Part " + PART_NAME[part] + " instrument");
    h.appendChild(chip);
    h.appendChild(document.createTextNode("See it — " + meta.title));
    panel.appendChild(h);
    var sub = el("p", "gbviz-sub");
    sub.innerHTML = meta.sub + (meta.builds ? " <em>Builds on the Chapter " + meta.builds +
      " instrument — same picture, next layer.</em>" : "");
    panel.appendChild(sub);
    var main = document.querySelector("main.wrap") || document.body;
    var firstEx = main.querySelector("article.ex");
    if (firstEx) main.insertBefore(panel, firstEx); else main.appendChild(panel);
    return panel;
  }

  /* =============================================================
   * Part I — The Shape of Data. One picture, three layers:
   * readings on a base line → fiber stalks over it → addresses.
   * ============================================================= */

  var VIZ = {};

  /* ch1 — the moon on the strip chart. The playground above prints the
   * numbers; this draws them: every insert is a dot, its κ (against the
   * stats BEFORE it, R=100 — the worked example's math) is the bar
   * underneath, and the 3σ band is the tinted lane the plain lives in. */
  VIZ[1] = {
    title: "the moon on the strip chart",
    sub: "The playground above prints κ as numbers; this is the same math drawn. " +
      "Each insert is a dot at its value, the bar under it is its κ at insert " +
      "(<code>|v − μ|/R</code>, R = 100, stats <em>before</em> the record — the engine's own rule), " +
      "and the tinted lane is μ ± 3σ. Plant the plain, drop the moon, and watch one bar dwarf the lane.",
    make: function (panel) {
      var C = makeCanvas(840, 300), st, hist;
      function reset() { st = { n: 0, mean: 0, m2: 0 }; hist = []; draw(); }
      function kappa(v) { return st.n < 2 ? 0 : Math.abs(v - st.mean) / 100.0; }
      function insert(v) {
        var k = kappa(v);
        st.n += 1;
        if (st.n === 1) { st.mean = v; st.m2 = 0; }
        else { var d = v - st.mean; st.mean += d / st.n; st.m2 += d * (v - st.mean); }
        hist.push({ v: v, k: k });
      }
      var rd = { n: readout("records"), mean: readout("mean (Welford)"),
                 kmax: readout("largest κ at insert"), flag: readout("flagged at 3σ") };
      function draw() {
        var x = C.ctx, W = C.W, H = C.H;
        x.clearRect(0, 0, W, H);
        var padL = 46, padR = 12, chartH = 190, barTop = chartH + 18;
        var n = Math.max(hist.length, 160);
        var vals = hist.map(function (r) { return r.v; });
        var lo = Math.min.apply(null, vals.concat([19.5])),
            hi = Math.max.apply(null, vals.concat([21.1]));
        var span = hi - lo || 1; lo -= span * 0.08; hi += span * 0.08;
        function X(i) { return padL + (W - padL - padR) * (i + 0.5) / n; }
        function Y(v) { return 12 + (chartH - 24) * (1 - (v - lo) / (hi - lo)); }
        var sigma = st.n > 1 ? Math.sqrt(st.m2 / (st.n - 1)) : 0;
        // 3σ lane
        if (st.n > 1) {
          x.fillStyle = "rgba(42,120,214,0.10)";
          var yTop = Y(st.mean + 3 * sigma), yBot = Y(st.mean - 3 * sigma);
          x.fillRect(padL, Math.max(6, yTop), W - padL - padR, Math.max(2, yBot - yTop));
          x.strokeStyle = BLUE; x.setLineDash([4, 4]); x.beginPath();
          x.moveTo(padL, Y(st.mean)); x.lineTo(W - padR, Y(st.mean)); x.stroke();
          x.setLineDash([]);
        }
        // axis labels
        x.fillStyle = INK3; x.font = "11px ui-monospace,Menlo,monospace";
        x.textAlign = "right";
        [lo + (hi - lo) * 0.9, lo + (hi - lo) * 0.5, lo + (hi - lo) * 0.1].forEach(function (v) {
          x.fillText(v >= 100 ? v.toFixed(0) : v.toFixed(1), padL - 6, Y(v) + 4);
        });
        x.textAlign = "left";
        x.fillText("value", padL - 40, 12);
        x.fillText("κ at insert", padL - 40, barTop + 10);
        // dots + κ bars
        var kmax = 0, flagged = 0;
        hist.forEach(function (r, i) {
          if (r.k > kmax) kmax = r.k;
          var out = st.n > 1 && Math.abs(r.v - st.mean) > 3 * sigma;
          if (out) flagged++;
          x.fillStyle = out ? RED : BLUE;
          x.beginPath(); x.arc(X(i), Y(r.v), out ? 4.5 : 2.4, 0, 6.2832); x.fill();
        });
        var kscale = Math.max(kmax, 0.05);
        hist.forEach(function (r, i) {
          var h = (H - barTop - 14) * (r.k / kscale);
          x.fillStyle = r.k > 3 * sigma / 100 && r.k > 0.5 ? RED : "rgba(42,120,214,0.55)";
          x.fillRect(X(i) - 1.4, H - 8 - h, 2.8, h);
        });
        x.strokeStyle = LINE; x.beginPath();
        x.moveTo(padL, H - 8); x.lineTo(W - padR, H - 8); x.stroke();
        rd.n.set(st.n);
        rd.mean.set(st.n ? st.mean.toFixed(4) : "–");
        rd.kmax.set(hist.length ? kmax.toFixed(4) : "–");
        rd.kmax.mark(kmax > 1);
        rd.flag.set(flagged); rd.flag.mark(flagged > 0);
      }
      var r1 = row();
      r1.appendChild(button("Plant 150 plain readings", function () {
        for (var i = 0; i < 150; i++) insert(20.0 + 0.6 * i / 149);
        draw();
      }));
      r1.appendChild(button("Drop the moon (500.0)", function () { insert(500.0); draw(); }));
      r1.appendChild(button("reset", reset, true));
      panel.appendChild(r1);
      panel.appendChild(C.cv);
      var stats = el("div", "gbviz-stats");
      [rd.n, rd.mean, rd.kmax, rd.flag].forEach(function (r) { stats.appendChild(r.el); });
      panel.appendChild(stats);
      panel.appendChild(note("The moon's κ bar lands at 4.797 — |500 − 20.3|/100, priced " +
        "against the mean <em>before</em> it arrived. The playground's receipt says 4.7653 " +
        "because a <code>SECTION</code> read reprices against current stats, and by then the " +
        "moon has dragged μ to ~23.5. Same rule, two moments — E1.7 is exactly that difference. " +
        "And the moon moved everyone: re-plant the plain and its κ bars come up a hair taller."));
      reset();
    }
  };

  /* ch2 — sections, not rows: the same base line, now with the fiber
   * stalk over each base point made visible. Toggle between the table
   * view (rows in a grid) and the bundle view (a section drawn through
   * the stalks); widen σ0 and watch the stalks fatten. */
  VIZ[2] = {
    title: "rows become sections",
    sub: "The same 24 stations, drawn twice. <strong>Table view</strong> is how a row-store sees them: " +
      "a grid, geometry discarded. <strong>Bundle view</strong> is Chapter 2's picture: a base line of " +
      "stations, a fiber stalk above each, and the data as a <em>section</em> — one chosen point per " +
      "stalk. The shaded stalk width is the declared fiber width σ₀; widen it and the section gets " +
      "room to move before anything counts as strange.",
    builds: 1,
    make: function (panel) {
      var C = makeCanvas(840, 300);
      var rng = mulberry32(20260702);
      var N = 24, temps = [];
      for (var i = 0; i < N; i++)
        temps.push(20.3 + Math.sin(i / N * Math.PI * 2) * 0.22 + (rng() - 0.5) * 0.12);
      temps[17] = 21.4; // one section pulled toward the stalk edge
      var mode = "bundle", sigma0 = 0.5;
      function draw() {
        var x = C.ctx, W = C.W, H = C.H;
        x.clearRect(0, 0, W, H);
        if (mode === "table") {
          var cols = 6, rows_ = 4, cw = (W - 40) / cols, chh = (H - 50) / rows_;
          x.font = "12px ui-monospace,Menlo,monospace";
          for (var i = 0; i < N; i++) {
            var cx = 20 + (i % cols) * cw, cy = 30 + Math.floor(i / cols) * chh;
            x.strokeStyle = LINE; x.strokeRect(cx, cy, cw - 8, chh - 8);
            x.fillStyle = INK3; x.fillText("s" + (i + 1), cx + 8, cy + 17);
            x.fillStyle = i === 17 ? RED : INK;
            x.fillText("temp=" + temps[i].toFixed(2), cx + 8, cy + 33);
          }
          x.fillStyle = INK3; x.font = "11px system-ui,sans-serif";
          x.fillText("24 rows. Which one is odd? You'd have to read them all — the grid has no geometry.", 20, H - 8);
        } else {
          var padL = 40, padR = 16, baseY = H - 46;
          var mid = 20.3, vspan = Math.max(1.6, sigma0 * 2.6);
          function X(i) { return padL + (W - padL - padR) * i / (N - 1); }
          function Y(v) { return baseY - (baseY - 26) * ((v - (mid - vspan)) / (2 * vspan)); }
          // base line
          x.strokeStyle = INK2; x.lineWidth = 2; x.beginPath();
          x.moveTo(padL - 10, baseY); x.lineTo(W - padR + 6, baseY); x.stroke(); x.lineWidth = 1;
          x.fillStyle = INK3; x.font = "11px system-ui,sans-serif";
          x.fillText("base space (stations)", padL - 10, baseY + 26);
          x.save(); x.translate(14, (baseY + 26) / 2); x.rotate(-Math.PI / 2);
          x.textAlign = "center"; x.fillText("fiber (temp)", 0, 0); x.restore();
          for (var i = 0; i < N; i++) {
            var sx = X(i);
            // σ0 band on the stalk
            x.fillStyle = "rgba(42,120,214,0.10)";
            var y1 = Y(mid + sigma0), y2 = Y(mid - sigma0);
            x.fillRect(sx - 5, y1, 10, y2 - y1);
            x.strokeStyle = LINE; x.beginPath();
            x.moveTo(sx, baseY); x.lineTo(sx, 26); x.stroke();
            x.fillStyle = INK3; x.beginPath(); x.arc(sx, baseY, 2.5, 0, 6.2832); x.fill();
          }
          // the section: a polyline through one point per stalk
          x.strokeStyle = BLUE; x.lineWidth = 2; x.beginPath();
          for (var i = 0; i < N; i++) {
            var sx = X(i), sy = Y(temps[i]);
            i ? x.lineTo(sx, sy) : x.moveTo(sx, sy);
          }
          x.stroke(); x.lineWidth = 1;
          for (var i = 0; i < N; i++) {
            var outside = Math.abs(temps[i] - mid) > sigma0;
            x.fillStyle = outside ? RED : BLUE;
            x.beginPath(); x.arc(X(i), Y(temps[i]), outside ? 5 : 3.2, 0, 6.2832); x.fill();
          }
          x.fillStyle = INK3;
          x.fillText("one section = one value chosen in each stalk — the odd one sticks out of its σ₀ band on sight", padL - 10, 16);
        }
      }
      var r1 = row();
      var bTable = button("Table view", function () { mode = "table"; sync(); }, true);
      var bBundle = button("Bundle view", function () { mode = "bundle"; sync(); });
      function sync() {
        bTable.className = "gbviz-btn" + (mode === "table" ? "" : " alt");
        bBundle.className = "gbviz-btn" + (mode === "bundle" ? "" : " alt");
        sl.input.disabled = mode === "table";
        draw();
      }
      r1.appendChild(bTable); r1.appendChild(bBundle);
      var sl = slider(panel, "declared fiber width σ₀", 0.15, 1.3, 0.05, 0.5,
        function (v) { return v.toFixed(2); },
        function (v) { sigma0 = v; draw(); });
      r1.appendChild(sl.el);
      panel.appendChild(r1);
      panel.appendChild(C.cv);
      panel.appendChild(note("Chapter 1's strip chart was values in time order; this is the " +
        "same kind of data laid over its base space. Station 18 is outside its stalk's σ₀ band " +
        "at the default width — drag σ₀ up and watch the fiber widen until it swallows the " +
        "anomaly. That's E2's widening experiments in one knob."));
      sync();
    }
  };

  /* ch3 — the address is a coordinate chart: hash the section names into
   * a 16×16 address grid, watch the birthday bound fill it with
   * collisions, and flip one bit of a key to see the avalanche. */
  VIZ[3] = {
    title: "the address space, filling up",
    sub: "Chapter 2's stalks need addresses. Here is a 256-slot address chart (16×16): each insert " +
      "hashes its key into a slot. Collisions arrive on the birthday schedule — expect the first " +
      "around √(π·256/2) ≈ 20 keys, long before the chart looks full. Then flip one bit of a key " +
      "and watch the avalanche: a good hash moves the address to the other side of the chart.",
    builds: 2,
    make: function (panel) {
      var C = makeCanvas(840, 320);
      // 32-bit avalanche finisher (murmur3-style) over the key string
      function hash32(s) {
        var h = 0x9e3779b9;
        for (var i = 0; i < s.length; i++) {
          h ^= s.charCodeAt(i);
          h = Math.imul(h, 0x85ebca6b); h ^= h >>> 13;
        }
        h ^= h >>> 16; h = Math.imul(h, 0xc2b2ae35); h ^= h >>> 16;
        return h >>> 0;
      }
      var GRID = 16, SLOTS = GRID * GRID;
      var occ, keys, nextId, collisions, firstColAt, lastPair;
      function reset() {
        occ = {}; keys = []; nextId = 1; collisions = 0; firstColAt = null; lastPair = null;
        draw();
      }
      function insertKeys(k) {
        for (var j = 0; j < k; j++) {
          var key = "station-" + nextId++;
          var slot = hash32(key) & (SLOTS - 1);
          if (occ[slot]) { collisions++; if (firstColAt === null) firstColAt = keys.length + 1; }
          occ[slot] = (occ[slot] || 0) + 1;
          keys.push(key);
        }
        lastPair = null;
        draw();
      }
      function flipBit() {
        if (!keys.length) return;
        var key = keys[keys.length - 1];
        var flipped = key.slice(0, -1) +
          String.fromCharCode(key.charCodeAt(key.length - 1) ^ 1);
        var a = hash32(key) & (SLOTS - 1), b = hash32(flipped) & (SLOTS - 1);
        // hamming distance of the full 32-bit hashes
        var x = (hash32(key) ^ hash32(flipped)) >>> 0, bits = 0;
        while (x) { bits += x & 1; x >>>= 1; }
        lastPair = { key: key, flipped: flipped, a: a, b: b, bits: bits };
        draw();
      }
      var rd = { n: readout("keys inserted"), col: readout("collisions"),
                 first: readout("first collision at"), av: readout("avalanche (bits flipped / 32)") };
      function draw() {
        var x = C.ctx, W = C.W, H = C.H;
        x.clearRect(0, 0, W, H);
        var cell = Math.min((H - 24) / GRID, 18), gx = 16, gy = 12;
        for (var s = 0; s < SLOTS; s++) {
          var cx = gx + (s % GRID) * cell, cy = gy + Math.floor(s / GRID) * cell;
          var c = occ[s] || 0;
          x.fillStyle = c === 0 ? "#fdfcf8" : c === 1 ? "rgba(42,120,214,0.55)" : RED;
          x.fillRect(cx, cy, cell - 2, cell - 2);
          x.strokeStyle = LINE; x.strokeRect(cx + 0.5, cy + 0.5, cell - 2, cell - 2);
        }
        var gridW = GRID * cell;
        // birthday curve: expected collisions vs n, with our trace
        var px = gx + gridW + 34, pw = W - px - 16, py = 16, ph = H - 60;
        x.strokeStyle = LINE; x.strokeRect(px, py, pw, ph);
        x.fillStyle = INK3; x.font = "11px system-ui,sans-serif";
        x.fillText("expected collisions vs keys (birthday bound, N=256)", px, py - 4);
        var NMAX = 120;
        x.strokeStyle = INK3; x.setLineDash([3, 3]); x.beginPath();
        for (var n = 0; n <= NMAX; n++) {
          // E[collisions] = n - N(1 - (1-1/N)^n)
          var e = n - SLOTS * (1 - Math.pow(1 - 1 / SLOTS, n));
          var X = px + pw * n / NMAX, Y = py + ph - ph * Math.min(1, e / 30);
          n ? x.lineTo(X, Y) : x.moveTo(X, Y);
        }
        x.stroke(); x.setLineDash([]);
        if (keys.length) {
          var n0 = Math.min(keys.length, NMAX);
          x.fillStyle = BLUE;
          x.beginPath();
          x.arc(px + pw * n0 / NMAX, py + ph - ph * Math.min(1, collisions / 30), 5, 0, 6.2832);
          x.fill();
          x.fillStyle = INK2;
          x.fillText("you: " + keys.length + " keys, " + collisions + " collisions",
            px + 8, py + ph - 8);
        }
        x.fillStyle = INK3;
        var mark = "√(πN/2) ≈ 20";
        x.fillText(mark, px + pw * 20 / NMAX - 14, py + ph + 14);
        x.strokeStyle = YELLOW; x.beginPath();
        x.moveTo(px + pw * 20 / NMAX, py); x.lineTo(px + pw * 20 / NMAX, py + ph); x.stroke();
        // avalanche pair
        if (lastPair) {
          [["a", BLUE], ["b", MAGENTA]].forEach(function (p) {
            var s = lastPair[p[0]];
            var cx = gx + (s % GRID) * cell, cy = gy + Math.floor(s / GRID) * cell;
            x.lineWidth = 3; x.strokeStyle = p[1];
            x.strokeRect(cx - 1.5, cy - 1.5, cell + 1, cell + 1);
            x.lineWidth = 1;
          });
          x.fillStyle = INK2; x.font = "12px ui-monospace,Menlo,monospace";
          x.fillText(lastPair.key + " → slot " + lastPair.a +
            "   ‖   1 bit flipped → slot " + lastPair.b, gx, H - 8);
        } else {
          x.fillStyle = INK3; x.font = "11px system-ui,sans-serif";
          x.fillText("address chart — blue = occupied, red = collision", gx, H - 8);
        }
        rd.n.set(keys.length); rd.col.set(collisions); rd.col.mark(collisions > 0);
        rd.first.set(firstColAt === null ? "–" : "key #" + firstColAt);
        rd.av.set(lastPair ? lastPair.bits + " / 32" : "–");
        rd.av.mark(lastPair && (lastPair.bits < 10 || lastPair.bits > 22));
      }
      var r1 = row();
      r1.appendChild(button("Insert 10 keys", function () { insertKeys(10); }));
      r1.appendChild(button("Insert 1 key", function () { insertKeys(1); }, true));
      r1.appendChild(button("Flip one bit of the last key", function () { flipBit(); }));
      r1.appendChild(button("reset", reset, true));
      panel.appendChild(r1);
      panel.appendChild(C.cv);
      var stats = el("div", "gbviz-stats");
      [rd.n, rd.col, rd.first, rd.av].forEach(function (r) { stats.appendChild(r.el); });
      panel.appendChild(stats);
      panel.appendChild(note("This is the browser-side stand-in for the chapter's avalanche " +
        "measurement — the real one runs against <code>src/hash.rs</code> and your receipt is its " +
        "bit-flip histogram. A healthy hash flips ~16 of 32 output bits per input bit; watch the " +
        "readout stay near the middle. And the first red cell almost always lands near key #20, " +
        "not #256 — the birthday bound is the whole reason the address chart carries more bits " +
        "than the section count seems to need."));
      reset();
    }
  };

  /* =============================================================
   * Part II — The Engine Room. One record's journey:
   * through the store → onto disk → onto the wire → spoken to.
   * ============================================================= */

  /* ch4 — anatomy of an insert: the record chip walks the path
   * base index → fiber store → Welford accumulators → κ stamp,
   * and each station updates its little display as it passes. */
  VIZ[4] = {
    title: "one insert, four stations",
    sub: "Press insert and follow the record through the BundleStore: the base index claims its " +
      "address, the fiber store takes the value, the Welford accumulators fold it in (n, μ, M₂ — " +
      "the six-liner), and the κ stamp prices it against the stats <em>before</em> it arrived. " +
      "The moon takes the same path — the machine doesn't branch, the number just comes out loud.",
    make: function (panel) {
      var C = makeCanvas(840, 250);
      var st = { n: 0, mean: 0, m2: 0 }, addr = 0;
      var anim = null; // {v, t}
      var STATIONS = [
        { x: 90, name: "base index", sub: function () { return "addr " + (addr ? "#" + addr : "–"); } },
        { x: 310, name: "fiber store", sub: function () { return st.n + " values"; } },
        { x: 530, name: "Welford", sub: function () { return st.n ? "μ=" + st.mean.toFixed(3) : "n=0"; } },
        { x: 750, name: "κ stamp", sub: function () { return lastK === null ? "–" : "κ=" + lastK.toFixed(4); } }
      ];
      var lastK = null;
      var loop = makeLoop(panel, function () { tick(); });
      function insert(v) {
        if (anim) return;
        anim = { v: v, t: 0 };
        loop.start();
      }
      function tick() {
        if (!anim) { loop.stop(); draw(); return; }
        anim.t += 1;
        var stage = Math.floor(anim.t / 34);
        if (anim.t % 34 === 33 && stage < 4) {
          // arriving at station `stage`
          if (stage === 0) addr += 1;
          if (stage === 2) {
            lastK = st.n < 2 ? 0 : Math.abs(anim.v - st.mean) / 100.0;
            st.n += 1;
            if (st.n === 1) { st.mean = anim.v; st.m2 = 0; }
            else { var d = anim.v - st.mean; st.mean += d / st.n; st.m2 += d * (anim.v - st.mean); }
          }
        }
        if (anim.t >= 34 * 4) { anim = null; }
        draw();
      }
      function draw() {
        var x = C.ctx, W = C.W, H = C.H;
        x.clearRect(0, 0, W, H);
        var railY = 120;
        x.strokeStyle = LINE; x.lineWidth = 6; x.beginPath();
        x.moveTo(30, railY); x.lineTo(W - 30, railY); x.stroke(); x.lineWidth = 1;
        var sigma = st.n > 1 ? Math.sqrt(st.m2 / (st.n - 1)) : 0;
        STATIONS.forEach(function (s, i) {
          var active = anim && Math.floor(anim.t / 34) === i && anim.t % 34 > 22;
          x.fillStyle = active ? AQUA : "#fff";
          x.strokeStyle = active ? AQUA : INK3;
          x.lineWidth = active ? 3 : 1.5;
          x.beginPath();
          x.roundRect ? x.roundRect(s.x - 62, railY - 74, 124, 52, 9)
                      : x.rect(s.x - 62, railY - 74, 124, 52);
          x.fill(); x.stroke(); x.lineWidth = 1;
          x.fillStyle = active ? "#fff" : INK;
          x.font = "700 13px system-ui,sans-serif"; x.textAlign = "center";
          x.fillText(s.name, s.x, railY - 52);
          x.fillStyle = active ? "#fff" : INK2;
          x.font = "12px ui-monospace,Menlo,monospace";
          x.fillText(s.sub(), s.x, railY - 34);
          x.textAlign = "left";
        });
        // the record chip
        if (anim) {
          var stage = Math.min(3, Math.floor(anim.t / 34));
          var frac = (anim.t % 34) / 34;
          var from = stage === 0 ? 30 : STATIONS[stage - 1].x;
          var to = STATIONS[stage].x;
          var cx = from + (to - from) * Math.min(1, frac * 1.4);
          x.fillStyle = anim.v > 100 ? RED : AQUA;
          x.beginPath(); x.arc(cx, railY, 11, 0, 6.2832); x.fill();
          x.fillStyle = "#fff"; x.font = "700 9px ui-monospace,monospace"; x.textAlign = "center";
          x.fillText(anim.v > 100 ? "500" : anim.v.toFixed(1), cx, railY + 3);
          x.textAlign = "left";
        }
        // accumulator panel
        x.fillStyle = INK2; x.font = "12px ui-monospace,Menlo,monospace";
        x.fillText("n=" + st.n + "   μ=" + (st.n ? st.mean.toFixed(4) : "–") +
          "   M₂=" + (st.n ? st.m2.toFixed(4) : "–") +
          "   σ=" + (st.n > 1 ? sigma.toFixed(4) : "–") +
          (lastK !== null ? "   last κ=" + lastK.toFixed(4) : ""), 30, H - 40);
        x.fillStyle = INK3; x.font = "11px system-ui,sans-serif";
        x.fillText("the thirteen-row insert path, four stops of it — src/bundle.rs owns every box", 30, H - 18);
      }
      var r1 = row();
      r1.appendChild(button("Insert a plain reading (20.3)", function () { insert(20.28 + (st.n % 7) * 0.01); }));
      r1.appendChild(button("Insert the moon (500.0)", function () { insert(500.0); }));
      r1.appendChild(button("reset", function () {
        st = { n: 0, mean: 0, m2: 0 }; addr = 0; lastK = null; anim = null; draw();
      }, true));
      panel.appendChild(r1);
      panel.appendChild(C.cv);
      panel.appendChild(note("Part I drew what the data looks like; Part II opens the machine. " +
        "The κ stamp fires <em>during</em> the insert, against the accumulators as they were before " +
        "the record — insert the moon twice and watch its second κ come out smaller, because the " +
        "first moon already dragged μ toward it. E4's hand-check is exactly this replay."));
      draw();
    }
  };

  /* ch5 — durability: WAL lane, memory lane, snapshot lane.
   * Append, snapshot (CoW), crash, replay. */
  VIZ[5] = {
    title: "the crash you can afford",
    sub: "Three lanes. <strong>WAL</strong> is the append-only truth on disk. <strong>Memory</strong> is " +
      "the live BundleStore. <strong>Snapshot</strong> is the mmap'd copy-on-write picture — taking one " +
      "copies nothing until a page is dirtied. Append a few records, snapshot, append more, then pull " +
      "the plug: memory dies, and replay rebuilds it from snapshot + WAL tail. Count what was re-read.",
    builds: 4,
    make: function (panel) {
      var C = makeCanvas(840, 250);
      var wal = [], mem = [], snap = null, crashed = false, replayed = 0, flash = 0;
      var loop = makeLoop(panel, function () { if (flash > 0) { flash--; draw(); } else loop.stop(); });
      function draw() {
        var x = C.ctx, W = C.W, H = C.H;
        x.clearRect(0, 0, W, H);
        var lanes = [
          { y: 44, name: "WAL (disk, append-only)", recs: wal, color: INK2 },
          { y: 116, name: crashed ? "memory — CRASHED" : "memory (live store)", recs: crashed ? [] : mem, color: crashed ? RED : AQUA },
          { y: 188, name: snap ? "snapshot @ record " + snap.upTo + " (mmap, CoW)" : "snapshot — none yet", recs: snap ? wal.slice(0, snap.upTo) : [], color: VIOLET }
        ];
        x.font = "11.5px system-ui,sans-serif";
        lanes.forEach(function (l) {
          x.fillStyle = l.color; x.fillText(l.name, 24, l.y - 22);
          x.strokeStyle = LINE; x.strokeRect(24, l.y - 14, W - 48, 30);
          l.recs.forEach(function (r, i) {
            var rx = 30 + i * 24;
            if (rx > W - 60) return;
            var shared = l.color === VIOLET && snap && !snap.copied[i];
            x.fillStyle = shared ? "rgba(74,58,167,0.25)" : l.color;
            x.fillRect(rx, l.y - 9, 19, 20);
            x.fillStyle = "#fff"; x.font = "9px ui-monospace,monospace"; x.textAlign = "center";
            x.fillText(r, rx + 9.5, l.y + 4); x.textAlign = "left";
            x.font = "11.5px system-ui,sans-serif";
          });
        });
        if (snap) {
          x.fillStyle = INK3; x.font = "10.5px system-ui,sans-serif";
          x.fillText("faded = page shared with the live store (never copied); solid = copied on first write after the snapshot",
            24, 226);
        }
        if (replayed && !crashed) {
          x.fillStyle = GREEN; x.font = "700 12px system-ui,sans-serif";
          x.fillText("recovered: snapshot gave " + (mem.length - replayed) +
            " records for free, WAL replayed only " + replayed, 24, H - 6);
        } else {
          x.fillStyle = INK3; x.font = "11px system-ui,sans-serif";
          x.fillText("src/storage/snapshot.rs + the WAL own this story — E5's receipts are hex dumps of these lanes", 24, H - 6);
        }
        if (flash > 0 && crashed) {
          x.fillStyle = "rgba(227,73,72," + (flash / 30 * 0.25) + ")";
          x.fillRect(0, 0, W, H);
        }
      }
      var n = 0;
      var bAppend = button("Append 3 records", function () {
        if (crashed) return;
        for (var i = 0; i < 3; i++) { n++; wal.push("r" + n); mem.push("r" + n);
          if (snap) { /* first write after snapshot dirties its page */ } }
        replayed = 0; draw();
      });
      var bSnap = button("Take snapshot", function () {
        if (crashed || !wal.length) return;
        snap = { upTo: wal.length, copied: {} };
        replayed = 0; draw();
      });
      var bCrash = button("Pull the plug", function () {
        if (crashed) return;
        crashed = true; replayed = 0; flash = 30; loop.start();
        bAppend.disabled = bSnap.disabled = true; bReplay.disabled = false;
        draw();
      });
      var bReplay = button("Reopen + replay", function () {
        if (!crashed) return;
        crashed = false;
        var fromSnap = snap ? snap.upTo : 0;
        mem = wal.slice(0, fromSnap).concat(wal.slice(fromSnap));
        replayed = wal.length - fromSnap;
        bAppend.disabled = bSnap.disabled = false; bReplay.disabled = true;
        // pages touched after reopen get copied
        draw();
      });
      bReplay.disabled = true;
      var r1 = row();
      [bAppend, bSnap, bCrash, bReplay].forEach(function (b) { r1.appendChild(b); });
      r1.appendChild(button("reset", function () {
        wal = []; mem = []; snap = null; crashed = false; replayed = 0; n = 0;
        bAppend.disabled = bSnap.disabled = false; bReplay.disabled = true; draw();
      }, true));
      panel.appendChild(r1);
      panel.appendChild(C.cv);
      panel.appendChild(note("Try the two orderings: crash <em>before</em> your first snapshot " +
        "and the replay walks the whole WAL; snapshot first and the replay only walks the tail. " +
        "The difference is E5's cold-start number — the snapshot is why Chapter 1's reopen could " +
        "answer <code>compute_anomalies</code> immediately, no warmup."));
      draw();
    }
  };

  /* ch6 — DHOOM byte lane: serialize a record, hover the bytes,
   * toggle sparsity to watch the zero-section run collapse. */
  VIZ[6] = {
    title: "the record, spelled in bytes",
    sub: "Chapter 5 wrote the record to disk; this is the same record leaving on the wire. Every cell " +
      "is one byte of a DHOOM-style frame. Hover to read a byte's job. Then make the fiber sparse — " +
      "the zero-section run collapses to a two-byte marker, and the frame gets visibly shorter. " +
      "The wire knows the zero section, so it never pays to ship it.",
    builds: 5,
    make: function (panel) {
      var C = makeCanvas(840, 230);
      var sparse = false, hover = -1;
      function frame() {
        // [magic ver] [bundle-id] [base len + bytes] [field count] fields... [crc]
        var b = [];
        function push(v, cls, label) { b.push({ v: v & 255, cls: cls, label: label }); }
        push(0xD4, "hdr", "magic byte 1 — 'this is DHOOM'");
        push(0x00, "hdr", "protocol version");
        push(0x07, "hdr", "bundle id (sensors)");
        var key = "s151";
        push(key.length, "base", "base key length");
        for (var i = 0; i < key.length; i++)
          push(key.charCodeAt(i), "base", "base key byte '" + key[i] + "'");
        var fields = sparse
          ? [["temp", 500.0]]
          : [["temp", 500.0], ["wind", 3.5], ["hum", 0.61]];
        var zeroCount = sparse ? 6 : 0;
        push(fields.length + zeroCount, "hdr", "fiber field count (declared schema order)");
        fields.forEach(function (f) {
          push(0x01, "tag", "field tag: numeric, present");
          // fake an f32 for compactness of the picture
          var dv = new DataView(new ArrayBuffer(4));
          dv.setFloat32(0, f[1]);
          for (var i = 0; i < 4; i++)
            push(dv.getUint8(i), "val", "f32 byte of " + f[0] + " = " + f[1]);
        });
        if (sparse) {
          push(0x00, "zero", "zero-section marker: next N fields sit ON the zero section");
          push(zeroCount, "zero", "run length — " + zeroCount + " fields, zero bytes shipped for them");
        }
        push(0x5A, "crc", "checksum");
        push(0xA5, "crc", "checksum");
        return b;
      }
      var CLS_COLOR = { hdr: INK2, base: BLUE, tag: YELLOW, val: AQUA, zero: MAGENTA, crc: INK3 };
      function draw() {
        var x = C.ctx, W = C.W, H = C.H;
        x.clearRect(0, 0, W, H);
        var bytes = frame();
        var cell = 34, perRow = Math.floor((W - 48) / cell), gx = 24, gy = 30;
        bytes.forEach(function (b, i) {
          var cx = gx + (i % perRow) * cell, cy = gy + Math.floor(i / perRow) * 46;
          x.fillStyle = CLS_COLOR[b.cls];
          if (i === hover) { x.fillStyle = INK; }
          x.fillRect(cx, cy, cell - 4, 30);
          x.fillStyle = "#fff"; x.font = "11px ui-monospace,Menlo,monospace"; x.textAlign = "center";
          x.fillText(("0" + b.v.toString(16)).slice(-2), cx + (cell - 4) / 2, cy + 19);
          x.textAlign = "left";
        });
        x.fillStyle = INK; x.font = "700 13px ui-monospace,monospace";
        x.fillText(bytes.length + " bytes on the wire" +
          (sparse ? "  (the dense spelling of the same 7 fields: 46)" : ""), gx, 18);
        // legend
        var lg = [["header", "hdr"], ["base key", "base"], ["field tag", "tag"],
                  ["value", "val"], ["zero-section run", "zero"], ["crc", "crc"]];
        var lx = gx;
        var ly = gy + Math.ceil(bytes.length / perRow) * 46 + 16;
        x.font = "11px system-ui,sans-serif";
        lg.forEach(function (l) {
          x.fillStyle = CLS_COLOR[l[1]]; x.fillRect(lx, ly - 9, 10, 10);
          x.fillStyle = INK2; x.fillText(l[0], lx + 14, ly);
          lx += x.measureText(l[0]).width + 34;
        });
        x.fillStyle = hover >= 0 && bytes[hover] ? INK : INK3;
        x.font = "12px ui-monospace,Menlo,monospace";
        x.fillText(hover >= 0 && bytes[hover] ? "byte " + hover + ": " + bytes[hover].label
          : "hover a byte — every one has a job", gx, ly + 24);
      }
      C.cv.addEventListener("pointermove", function (ev) {
        var r = C.cv.getBoundingClientRect();
        var mx = (ev.clientX - r.left) * (C.W / r.width),
            my = (ev.clientY - r.top) * (C.H / r.height);
        var cell = 34, perRow = Math.floor((C.W - 48) / cell);
        var col = Math.floor((mx - 24) / cell), rw = Math.floor((my - 30) / 46);
        var i = (col >= 0 && col < perRow && rw >= 0) ? rw * perRow + col : -1;
        if (i !== hover) { hover = i; draw(); }
      });
      C.cv.addEventListener("pointerleave", function () { hover = -1; draw(); });
      var r1 = row();
      var bDense = button("Dense record (3 fields live)", function () { sparse = false; sync(); });
      var bSparse = button("Sparse record (1 of 7 fields live)", function () { sparse = true; sync(); }, true);
      function sync() {
        bDense.className = "gbviz-btn" + (sparse ? " alt" : "");
        bSparse.className = "gbviz-btn" + (sparse ? "" : " alt");
        hover = -1; draw();
      }
      r1.appendChild(bDense); r1.appendChild(bSparse);
      panel.appendChild(r1);
      panel.appendChild(C.cv);
      panel.appendChild(note("This picture is schematic — the shipped encoder's exact layout is " +
        "what your E6 hex dumps will show, and counting <em>those</em> bytes is the receipt. The " +
        "shape of the win is the same: fields that sit on the zero section cost a marker and a run " +
        "length, not values. Sparse fibers are the normal case in a wide bundle, which is why the " +
        "wire format knowing the zero section is a design decision and not a compression trick."));
      draw();
    }
  };

  /* ch7 — GQL: geometry determines complexity. Two lanes race:
   * COVER ON city (index hop) vs COVER WHERE temp (full scan). */
  VIZ[7] = {
    title: "two queries race",
    sub: "The machine is assembled; now speak to it. Same bundle, two spellings: " +
      "<code>COVER sensors ON city='Moscow'</code> rides the base index straight to its k rows; " +
      "<code>COVER sensors WHERE temp &lt; −20</code> must walk every fiber. Scale n and race them — " +
      "the ON lane's cost is the answer's size, the WHERE lane's cost is the bundle's size. " +
      "That is what “geometry determines complexity” cashes out to.",
    builds: 6,
    make: function (panel) {
      var C = makeCanvas(840, 240);
      var n = 4096, running = false, prog = 0, done = false;
      var hitFrac = 1 / 16; // Moscow's share
      var loop = makeLoop(panel, function () {
        if (!running) return;
        prog += 0.012;
        if (prog >= 1) { prog = 1; running = false; done = true; loop.stop(); }
        draw();
      });
      function draw() {
        var x = C.ctx, W = C.W, H = C.H;
        x.clearRect(0, 0, W, H);
        var hits = Math.max(1, Math.round(n * hitFrac));
        var lanes = [
          { y: 60, label: "COVER sensors ON city = 'Moscow';", ops: hits, kind: "index" },
          { y: 150, label: "COVER sensors WHERE temp < -20;", ops: n, kind: "scan" }
        ];
        x.font = "12.5px ui-monospace,Menlo,monospace";
        var maxOps = n;
        lanes.forEach(function (l) {
          x.fillStyle = INK; x.fillText(l.label, 24, l.y - 24);
          x.strokeStyle = LINE; x.strokeRect(24, l.y - 12, W - 200, 26);
          // this lane finishes when the racer has done l.ops of maxOps work
          var frac = Math.min(1, prog * maxOps / l.ops);
          x.fillStyle = l.kind === "index" ? AQUA : YELLOW;
          x.fillRect(24, l.y - 12, (W - 200) * frac, 26);
          x.fillStyle = INK2; x.font = "12px ui-monospace,monospace";
          var opsDone = Math.round(Math.min(l.ops, prog * maxOps));
          x.fillText(opsDone.toLocaleString() + " / " + l.ops.toLocaleString() + " rows touched",
            W - 168, l.y + 6);
          if (frac >= 1 && (prog > 0 || done)) {
            x.fillStyle = GREEN; x.font = "700 12px system-ui,sans-serif";
            x.fillText("✓ done", W - 168, l.y - 12);
          }
          x.font = "12.5px ui-monospace,Menlo,monospace";
        });
        x.fillStyle = INK3; x.font = "11.5px system-ui,sans-serif";
        x.fillText("index lane touches only the " + hits.toLocaleString() +
          " Moscow rows (the base index hands them over); the scan lane reads all " +
          n.toLocaleString() + " fibers to ask each one a question", 24, 210);
        if (done) {
          x.fillStyle = INK; x.font = "700 13px system-ui,sans-serif";
          x.fillText("same answer, " + Math.round(n / Math.max(1, hits)) + "× the work — and the ratio grows with n", 24, 232);
        }
      }
      var r1 = row();
      r1.appendChild(button("Race", function () {
        prog = 0; done = false; running = true; loop.start();
      }));
      var sl = slider(panel, "records in the bundle (n)", 9, 17, 1, 12,
        function (v) { return Math.pow(2, v).toLocaleString(); },
        function (v) { n = Math.pow(2, v); prog = 0; done = false; running = false; draw(); });
      r1.appendChild(sl.el);
      panel.appendChild(r1);
      panel.appendChild(C.cv);
      panel.appendChild(note("Run the same pair against the live console above with " +
        "<code>EXPLAIN</code> in front and the plan rows say which lane you bought. E7's audit " +
        "is this race with wire transcripts for receipts — and the chapter's warning stands: a " +
        "clause the parser doesn't own doesn't slow the race down, it silently never runs. " +
        "(That's why the engine now refuses trailing clauses out loud.)"));
      draw();
    }
  };

  /* =============================================================
   * Part III — The Signals That Run While You Sleep.
   * vitals stream → the room behind λ₁ → alerts on the wire.
   * ============================================================= */

  /* ch8 — interoception: four vitals streaming. */
  VIZ[8] = {
    title: "the engine's vitals, streaming",
    sub: "Four channels of interoception, updating as inserts arrive: the running mean, the running " +
      "σ, the per-insert κ, and confidence 1/(1+K). Let it breathe for a while, then inject a burst " +
      "of bad readings and watch which channel notices first — and which one <em>recovers</em> first " +
      "when the burst stops.",
    make: function (panel) {
      var C = makeCanvas(840, 300);
      var rng = mulberry32(88), st, series, burst, tGlobal;
      function reset() {
        st = { n: 0, mean: 0, m2: 0 };
        series = { mean: [], sigma: [], kappa: [], conf: [] };
        burst = 0; tGlobal = 0;
      }
      function step() {
        tGlobal++;
        var v = 20.3 + Math.sin(tGlobal / 40) * 0.15 + (rng() - 0.5) * 0.2;
        if (burst > 0) { v += 6 + rng() * 4; burst--; }
        var k = st.n < 2 ? 0 : Math.abs(v - st.mean) / 100.0;
        st.n++;
        if (st.n === 1) { st.mean = v; st.m2 = 0; }
        else { var d = v - st.mean; st.mean += d / st.n; st.m2 += d * (v - st.mean); }
        var sigma = st.n > 1 ? Math.sqrt(st.m2 / (st.n - 1)) : 0;
        push("mean", st.mean); push("sigma", sigma); push("kappa", k);
        push("conf", 1 / (1 + k * 20));
      }
      function push(ch, v) { series[ch].push(v); if (series[ch].length > 240) series[ch].shift(); }
      var CHANNELS = [
        { key: "mean", label: "running μ", color: BLUE },
        { key: "sigma", label: "running σ", color: VIOLET },
        { key: "kappa", label: "κ at insert", color: YELLOW },
        { key: "conf", label: "confidence 1/(1+K)", color: AQUA }
      ];
      function draw() {
        var x = C.ctx, W = C.W, H = C.H;
        x.clearRect(0, 0, W, H);
        var laneH = (H - 20) / 4;
        CHANNELS.forEach(function (ch, li) {
          var data = series[ch.key];
          var y0 = 10 + li * laneH;
          x.strokeStyle = LINE; x.beginPath();
          x.moveTo(120, y0 + laneH - 12); x.lineTo(W - 12, y0 + laneH - 12); x.stroke();
          x.fillStyle = ch.color; x.font = "700 12px system-ui,sans-serif";
          x.fillText(ch.label, 14, y0 + laneH / 2);
          if (data.length > 1) {
            var lo = Math.min.apply(null, data), hi = Math.max.apply(null, data);
            if (hi - lo < 1e-9) { hi = lo + 1e-9; }
            x.strokeStyle = ch.color; x.lineWidth = 1.8; x.beginPath();
            data.forEach(function (v, i) {
              var px = 120 + (W - 132) * i / 239;
              var py = y0 + 6 + (laneH - 24) * (1 - (v - lo) / (hi - lo));
              i ? x.lineTo(px, py) : x.moveTo(px, py);
            });
            x.stroke(); x.lineWidth = 1;
            x.fillStyle = INK2; x.font = "11px ui-monospace,Menlo,monospace";
            x.fillText(data[data.length - 1].toFixed(4), W - 84, y0 + 14);
          }
        });
        if (burst > 0) {
          x.fillStyle = RED; x.font = "700 12px system-ui,sans-serif";
          x.fillText("burst: " + burst + " bad readings left", 120, 14);
        }
      }
      var loop = makeLoop(panel, function () { step(); draw(); });
      var bRun = button("Stream", function () {
        if (loop.running) { loop.stop(); bRun.textContent = "Stream"; }
        else { loop.start(); bRun.textContent = "Pause"; }
      });
      var r1 = row();
      r1.appendChild(bRun);
      r1.appendChild(button("Inject 25 bad readings", function () { burst = 25; if (!loop.running) { loop.start(); bRun.textContent = "Pause"; } }));
      r1.appendChild(button("reset", function () { reset(); draw(); }, true));
      panel.appendChild(r1);
      panel.appendChild(C.cv);
      panel.appendChild(note("κ spikes on the first bad insert — it's priced against the stats " +
        "before the record, so it needs no window and no batch job. μ and σ drift and then " +
        "<em>stay</em> drifted after the burst: Welford never forgets, which is exactly the " +
        "contract E8 stresses at both ends of the n-axis. This panel is the browser twin of the " +
        "Prometheus scrape your receipts come from."));
      reset();
      for (var i = 0; i < 120; i++) step();
      draw();
    }
  };

  /* ch9 — spectra & communities: toy graph, live λ₁ (Jacobi), Fiedler split. */
  VIZ[9] = {
    title: "the room behind λ₁",
    sub: "Chapter 8 streamed λ₁ as one number on a monitor; here is the room the number is " +
      "listening to. Two communities of a dozen nodes each; the slider adds bridges between them. " +
      "λ₁ — the spectral gap, the second-smallest eigenvalue of the graph Laplacian, computed live " +
      "— stays near zero while the graph is nearly two rooms, and climbs as the rooms fuse. Node " +
      "colors are the Fiedler vector's sign: the split the spectrum sees.",
    builds: 8,
    make: function (panel) {
      var C = makeCanvas(840, 330);
      var rng = mulberry32(9), N = 24;
      var pos = [], baseEdges = [], bridgePool = [];
      for (var i = 0; i < N; i++) {
        var side = i < N / 2 ? 0 : 1;
        var a = (i % (N / 2)) / (N / 2) * Math.PI * 2;
        pos.push({
          x: 210 + side * 420 + Math.cos(a) * (95 + rng() * 22),
          y: 165 + Math.sin(a) * (95 + rng() * 22)
        });
      }
      function connected(edges) {
        var seen = [0], q = [0];
        while (q.length) {
          var u = q.pop();
          edges.forEach(function (e) {
            var v = e[0] === u ? e[1] : e[1] === u ? e[0] : -1;
            if (v >= 0 && seen.indexOf(v) < 0) { seen.push(v); q.push(v); }
          });
        }
        return seen.length === N;
      }
      // ring + chords inside each community
      for (var s = 0; s < 2; s++) {
        var off = s * N / 2;
        for (var i = 0; i < N / 2; i++) {
          baseEdges.push([off + i, off + (i + 1) % (N / 2)]);
          if (i % 3 === 0) baseEdges.push([off + i, off + (i + 5) % (N / 2)]);
        }
      }
      for (var k = 0; k < 12; k++)
        bridgePool.push([Math.floor(rng() * N / 2), N / 2 + Math.floor(rng() * N / 2)]);
      var nBridges = 1;
      // Jacobi eigensolver for symmetric matrices (N=24 — instant)
      function eigs(A) {
        var n = A.length, V = [], i, j;
        var M = A.map(function (r) { return r.slice(); });
        for (i = 0; i < n; i++) { V.push(new Array(n).fill(0)); V[i][i] = 1; }
        for (var sweep = 0; sweep < 40; sweep++) {
          var off = 0;
          for (i = 0; i < n; i++) for (j = i + 1; j < n; j++) off += M[i][j] * M[i][j];
          if (off < 1e-12) break;
          for (i = 0; i < n; i++) for (j = i + 1; j < n; j++) {
            if (Math.abs(M[i][j]) < 1e-14) continue;
            var theta = (M[j][j] - M[i][i]) / (2 * M[i][j]);
            var t = Math.sign(theta || 1) / (Math.abs(theta) + Math.sqrt(theta * theta + 1));
            var c = 1 / Math.sqrt(t * t + 1), s2 = t * c;
            for (var k2 = 0; k2 < n; k2++) {
              var mik = M[i][k2], mjk = M[j][k2];
              M[i][k2] = c * mik - s2 * mjk; M[j][k2] = s2 * mik + c * mjk;
            }
            for (var k2 = 0; k2 < n; k2++) {
              var mki = M[k2][i], mkj = M[k2][j];
              M[k2][i] = c * mki - s2 * mkj; M[k2][j] = s2 * mki + c * mkj;
              var vki = V[k2][i], vkj = V[k2][j];
              V[k2][i] = c * vki - s2 * vkj; V[k2][j] = s2 * vki + c * vkj;
            }
          }
        }
        var order = [];
        for (i = 0; i < n; i++) order.push(i);
        order.sort(function (a, b) { return M[a][a] - M[b][b]; });
        return {
          values: order.map(function (i) { return M[i][i]; }),
          vector: function (k3) {
            var idx = order[k3];
            return V.map(function (r) { return r[idx]; });
          }
        };
      }
      var rd = { l1: readout("λ₁ (spectral gap)"), e: readout("edges"), b: readout("bridges") };
      function draw() {
        var edges = baseEdges.concat(bridgePool.slice(0, nBridges));
        var L = [];
        for (var i = 0; i < N; i++) L.push(new Array(N).fill(0));
        edges.forEach(function (e) {
          L[e[0]][e[0]]++; L[e[1]][e[1]]++;
          L[e[0]][e[1]]--; L[e[1]][e[0]]--;
        });
        var eig = eigs(L);
        var l1 = eig.values[1], fiedler = eig.vector(1);
        var x = C.ctx, W = C.W, H = C.H;
        x.clearRect(0, 0, W, H);
        edges.forEach(function (e) {
          var bridge = (e[0] < N / 2) !== (e[1] < N / 2);
          x.strokeStyle = bridge ? YELLOW : LINE;
          x.lineWidth = bridge ? 2.4 : 1.2;
          x.beginPath();
          x.moveTo(pos[e[0]].x, pos[e[0]].y); x.lineTo(pos[e[1]].x, pos[e[1]].y);
          x.stroke();
        });
        x.lineWidth = 1;
        pos.forEach(function (p, i) {
          x.fillStyle = fiedler[i] < 0 ? BLUE : MAGENTA;
          x.beginPath(); x.arc(p.x, p.y, 8, 0, 6.2832); x.fill();
          x.strokeStyle = "#fff"; x.stroke();
        });
        // gap bar
        var bx = 24, by = H - 26, bw = W - 48;
        x.strokeStyle = LINE; x.strokeRect(bx, by, bw, 14);
        x.fillStyle = l1 < 0.15 ? RED : l1 < 0.6 ? YELLOW : GREEN;
        x.fillRect(bx, by, bw * Math.min(1, l1 / 2.2), 14);
        x.fillStyle = INK2; x.font = "11.5px system-ui,sans-serif";
        x.fillText("λ₁ = " + l1.toFixed(4) + "  —  " +
          (l1 < 0.15 ? "nearly two rooms: the gap hears the wall"
            : l1 < 0.6 ? "rooms fusing — the wall is getting doors"
            : "one room: no wall left to hear"), bx, by - 6);
        rd.l1.set(l1.toFixed(4)); rd.l1.mark(l1 < 0.15);
        rd.e.set(edges.length); rd.b.set(nBridges);
      }
      var r1 = row();
      var sl = slider(panel, "bridges between the communities", 0, 12, 1, 1,
        function (v) { return v; },
        function (v) { nBridges = v; draw(); });
      r1.appendChild(sl.el);
      panel.appendChild(r1);
      panel.appendChild(C.cv);
      var stats = el("div", "gbviz-stats");
      [rd.l1, rd.e, rd.b].forEach(function (r) { stats.appendChild(r.el); });
      panel.appendChild(stats);
      panel.appendChild(note("At zero bridges the graph disconnects and λ₁ hits exactly 0 — the " +
        "early-exit case the chapter's first room is about. One bridge gives a tiny but nonzero " +
        "gap; every additional bridge raises it, fast at first, then saturating. The node colors " +
        "are the Fiedler vector's signs, and they find the two communities without being told — " +
        "that's the whole trick behind <code>SPECTRAL</code>'s community readout."));
      draw();
    }
  };

  /* ch10 — the brain on the wire: signal → threshold → verb frames. */
  VIZ[10] = {
    title: "from signal to verb",
    sub: "The monitor from Chapter 8 and the gap from Chapter 9, wired to an output. The trace is a " +
      "streaming vital; drag the threshold; every crossing emits a frame onto the wire lane below — " +
      "the same shape the engine's alert verbs ship. Set the threshold too tight and read the wire: " +
      "an alarm that fires hourly is a signal you've taught everyone to ignore.",
    builds: 9,
    make: function (panel) {
      var C = makeCanvas(840, 300);
      var rng = mulberry32(1010), data = [], frames = [], thr = 0.62, t = 0, dragging = false;
      function step() {
        t++;
        var base = 0.32 + 0.1 * Math.sin(t / 60) + (rng() - 0.5) * 0.08;
        if (t % 190 > 158) base += 0.45 * Math.sin((t % 190 - 158) / 32 * Math.PI);
        data.push(Math.max(0, base));
        if (data.length > 360) data.shift();
        var v = data[data.length - 1], prev = data.length > 1 ? data[data.length - 2] : 0;
        if (v > thr && prev <= thr) {
          frames.push({ t: t, v: v });
          if (frames.length > 24) frames.shift();
        }
      }
      function draw() {
        var x = C.ctx, W = C.W, H = C.H;
        x.clearRect(0, 0, W, H);
        var chartH = 190, padL = 40;
        function Y(v) { return 14 + (chartH - 28) * (1 - v); }
        // threshold
        x.strokeStyle = RED; x.setLineDash([5, 4]); x.beginPath();
        x.moveTo(padL, Y(thr)); x.lineTo(W - 12, Y(thr)); x.stroke(); x.setLineDash([]);
        x.fillStyle = RED; x.font = "11px system-ui,sans-serif";
        x.fillText("threshold " + thr.toFixed(2) + " (drag me)", padL + 6, Y(thr) - 6);
        // trace
        x.strokeStyle = YELLOW; x.lineWidth = 1.8; x.beginPath();
        data.forEach(function (v, i) {
          var px = padL + (W - padL - 12) * i / 359;
          i ? x.lineTo(px, Y(v)) : x.moveTo(px, Y(v));
        });
        x.stroke(); x.lineWidth = 1;
        // wire lane
        var wy = chartH + 40;
        x.strokeStyle = INK2; x.lineWidth = 2; x.beginPath();
        x.moveTo(padL, wy); x.lineTo(W - 12, wy); x.stroke(); x.lineWidth = 1;
        x.fillStyle = INK3; x.font = "11px system-ui,sans-serif";
        x.fillText("the wire — one frame per crossing", padL, wy + 34);
        frames.forEach(function (f) {
          var age = t - f.t;
          if (age > 360) return;
          var px = padL + (W - padL - 12) * (1 - age / 360);
          x.fillStyle = MAGENTA;
          x.fillRect(px - 14, wy - 12, 28, 24);
          x.fillStyle = "#fff"; x.font = "700 9px ui-monospace,monospace"; x.textAlign = "center";
          x.fillText("ALRT", px, wy + 3); x.textAlign = "left";
        });
        var rate = frames.filter(function (f) { return t - f.t < 360; }).length;
        x.fillStyle = rate > 6 ? RED : INK2; x.font = "700 12px system-ui,sans-serif";
        x.fillText(rate + " alert" + (rate === 1 ? "" : "s") + " on screen" +
          (rate > 6 ? " — nobody reads seven alarms" : ""), W - 260, wy + 34);
      }
      C.cv.addEventListener("pointerdown", function (ev) { dragging = true; move(ev); });
      window.addEventListener("pointerup", function () { dragging = false; });
      C.cv.addEventListener("pointermove", move);
      function move(ev) {
        if (!dragging) return;
        var r = C.cv.getBoundingClientRect();
        var my = (ev.clientY - r.top) * (C.H / r.height);
        thr = Math.max(0.05, Math.min(0.98, 1 - (my - 14) / (190 - 28)));
        draw();
      }
      var loop = makeLoop(panel, function () { step(); draw(); });
      var bRun = button("Stream", function () {
        if (loop.running) { loop.stop(); bRun.textContent = "Stream"; }
        else { loop.start(); bRun.textContent = "Pause"; }
      });
      var r1 = row(); r1.appendChild(bRun);
      panel.appendChild(r1);
      panel.appendChild(C.cv);
      panel.appendChild(note("Part III end-to-end: a running statistic (ch8), a spectral quantity " +
        "behind it (ch9), and the verb that leaves the building (this chapter). The engine's eleven " +
        "wire verbs are this lane with types; E10's exercises pressure the seams — what happens when " +
        "frames arrive faster than anyone drains them is not a hypothetical, it's the backlog you " +
        "can make here with one drag."));
      for (var i = 0; i < 300; i++) step();
      draw();
    }
  };

  /* =============================================================
   * Part IV — What the Loop Remembers: transport on a curved patch.
   * ============================================================= */

  /* ch11 — holonomy: parallel-transport an arrow around a loop on a
   * curvature bump; the return angle is the loop integral of K. */
  VIZ[11] = {
    title: "the carry that comes home rotated",
    sub: "The heat map is a curvature field — one Gaussian bump of strength K₀. Drag the loop " +
      "(move its center, resize with the slider), then run it: an arrow is parallel-transported " +
      "around, step by step. On flat ground it comes home pointing where it started. Around the " +
      "bump it comes home <em>rotated</em> — by exactly ∬K dA, the curvature enclosed. The loop " +
      "remembers what it walked around.",
    make: function (panel) {
      var C = makeCanvas(840, 330);
      var K0 = 1.6, S = 90; // bump strength, width in px
      var bump = { x: 300, y: 165 };
      var loopC = { x: 380, y: 165 }, loopR = 90;
      var angle = 0, runT = -1, dragging = false;
      function Kat(px, py) {
        var dx = px - bump.x, dy = py - bump.y;
        return K0 * Math.exp(-(dx * dx + dy * dy) / (2 * S * S));
      }
      // enclosed curvature: numeric integral over the loop disk (in units where px²·K/S² ~ rad)
      function enclosed() {
        var sum = 0, step = 8;
        for (var px = loopC.x - loopR; px <= loopC.x + loopR; px += step)
          for (var py = loopC.y - loopR; py <= loopC.y + loopR; py += step) {
            var dx = px - loopC.x, dy = py - loopC.y;
            if (dx * dx + dy * dy <= loopR * loopR)
              sum += Kat(px, py) * step * step;
          }
        return sum / (S * S); // radians
      }
      var rd = { def: readout("holonomy angle (deficit)"), enc: readout("∬K dA enclosed") };
      function draw() {
        var x = C.ctx, W = C.W, H = C.H;
        x.clearRect(0, 0, W, H);
        // curvature heat map (coarse cells)
        for (var px = 0; px < W; px += 12)
          for (var py = 0; py < H - 40; py += 12) {
            var k = Kat(px + 6, py + 6) / K0;
            if (k < 0.02) continue;
            x.fillStyle = "rgba(0,131,0," + (k * 0.32) + ")";
            x.fillRect(px, py, 12, 12);
          }
        // the loop
        x.strokeStyle = INK; x.lineWidth = 2;
        x.beginPath(); x.arc(loopC.x, loopC.y, loopR, 0, 6.2832); x.stroke(); x.lineWidth = 1;
        x.fillStyle = INK; x.beginPath(); x.arc(loopC.x, loopC.y, 3, 0, 6.2832); x.fill();
        x.fillStyle = INK3; x.font = "11px system-ui,sans-serif";
        x.fillText("drag the loop center", loopC.x - 46, loopC.y + loopR + 16);
        var target = enclosed();
        // transported arrow
        var frac = runT < 0 ? 1 : Math.min(1, runT / 140);
        var theta = -Math.PI / 2 + frac * Math.PI * 2;
        var ax = loopC.x + Math.cos(theta) * loopR, ay = loopC.y + Math.sin(theta) * loopR;
        var carried = runT < 0 ? angle : frac * target;
        drawArrow(x, ax, ay, carried, runT >= 0 || angle !== 0);
        // start ghost
        drawArrow(x, loopC.x, loopC.y - loopR, 0, false, true);
        if (runT >= 0 && frac >= 1) { angle = target; runT = -1; }
        rd.def.set((angle * 180 / Math.PI).toFixed(2) + "°");
        rd.def.mark(Math.abs(angle) > 0.05);
        rd.enc.set((target * 180 / Math.PI).toFixed(2) + "°");
      }
      function drawArrow(x, px, py, rot, hot, ghost) {
        x.save(); x.translate(px, py); x.rotate(rot);
        x.strokeStyle = ghost ? INK3 : hot ? RED : GREEN;
        x.fillStyle = x.strokeStyle;
        x.lineWidth = ghost ? 1.5 : 3;
        if (ghost) x.setLineDash([3, 3]);
        x.beginPath(); x.moveTo(0, 0); x.lineTo(34, 0); x.stroke();
        x.beginPath(); x.moveTo(34, 0); x.lineTo(25, -6); x.lineTo(25, 6); x.closePath(); x.fill();
        x.restore(); x.setLineDash([]); x.lineWidth = 1;
      }
      var loop = makeLoop(panel, function () {
        if (runT < 0) { loop.stop(); return; }
        runT += 2; draw();
      });
      C.cv.addEventListener("pointerdown", function (ev) { dragging = true; move(ev); });
      window.addEventListener("pointerup", function () { dragging = false; });
      C.cv.addEventListener("pointermove", move);
      function move(ev) {
        if (!dragging) return;
        var r = C.cv.getBoundingClientRect();
        loopC.x = (ev.clientX - r.left) * (C.W / r.width);
        loopC.y = Math.min(C.H - 60, Math.max(40, (ev.clientY - r.top) * (C.H / r.height)));
        angle = 0; draw();
      }
      var r1 = row();
      r1.appendChild(button("Transport the arrow around the loop", function () {
        angle = 0; runT = 0; loop.start();
      }));
      var sl = slider(panel, "loop radius", 30, 150, 5, 90,
        function (v) { return v + "px"; },
        function (v) { loopR = v; angle = 0; draw(); });
      r1.appendChild(sl.el);
      var sl2 = slider(panel, "bump strength K₀", 0, 3, 0.1, 1.6,
        function (v) { return v.toFixed(1); },
        function (v) { K0 = v; angle = 0; draw(); });
      r1.appendChild(sl2.el);
      panel.appendChild(r1);
      panel.appendChild(C.cv);
      var stats = el("div", "gbviz-stats");
      [rd.def, rd.enc].forEach(function (r) { stats.appendChild(r.el); });
      panel.appendChild(stats);
      panel.appendChild(note("Drag the loop off the bump and the deficit goes to zero — " +
        "flat_transport really is trivial, which is the first of the chapter's five named " +
        "call sites. Grow the loop until it swallows the bump and the deficit stops growing: " +
        "holonomy prices what's <em>enclosed</em>, not the path length. In the engine the loop is " +
        "a cycle of shards and the arrow is your data's frame; <code>holonomy_debt</code> is this " +
        "angle with a ledger."));
      draw();
    }
  };

  /* ch12 — torsion: X-then-Y vs Y-then-X fail to land together. */
  VIZ[12] = {
    title: "the parallelogram that won't close",
    sub: "Same patch as Chapter 11, different failure. Walk east then north; walk north then east. " +
      "With no torsion the two paths close a parallelogram. Turn torsion up and the corners miss — " +
      "the gap between the two endpoints is the torsion tensor eating the commutator. Holonomy " +
      "rotated the <em>frame</em>; torsion breaks the <em>position</em>.",
    builds: 11,
    make: function (panel) {
      var C = makeCanvas(840, 320);
      var tau = 0.0, animT = -1;
      var O = { x: 280, y: 230 }, DX = 240, DY = 130;
      function endpoints() {
        // path A: east then north — torsion twists the second leg
        var ax = O.x + DX, ay = O.y;
        var a2x = ax + tau * DY * 0.6, a2y = ay - DY + tau * DY * 0.2;
        // path B: north then east — opposite twist
        var bx = O.x, by = O.y - DY;
        var b2x = bx + DX - tau * DX * 0.25, b2y = by - tau * DX * 0.45;
        return { a: [O, { x: ax, y: ay }, { x: a2x, y: a2y }],
                 b: [O, { x: bx, y: by }, { x: b2x, y: b2y }] };
      }
      var rd = { gap: readout("closing gap |T(u,v)|") };
      function draw() {
        var x = C.ctx, W = C.W, H = C.H;
        x.clearRect(0, 0, W, H);
        var ep = endpoints();
        var frac = animT < 0 ? 1 : Math.min(1, animT / 120);
        function drawPath(pts, color, f) {
          x.strokeStyle = color; x.lineWidth = 3; x.beginPath();
          x.moveTo(pts[0].x, pts[0].y);
          var seg = f * 2;
          if (seg >= 1) x.lineTo(pts[1].x, pts[1].y);
          else { x.lineTo(pts[0].x + (pts[1].x - pts[0].x) * seg, pts[0].y + (pts[1].y - pts[0].y) * seg); x.stroke(); x.lineWidth = 1; return; }
          var s2 = seg - 1;
          x.lineTo(pts[1].x + (pts[2].x - pts[1].x) * Math.min(1, s2),
                   pts[1].y + (pts[2].y - pts[1].y) * Math.min(1, s2));
          x.stroke(); x.lineWidth = 1;
        }
        drawPath(ep.a, BLUE, frac);
        drawPath(ep.b, MAGENTA, frac);
        x.fillStyle = INK; x.beginPath(); x.arc(O.x, O.y, 4, 0, 6.2832); x.fill();
        x.font = "12px system-ui,sans-serif";
        x.fillStyle = BLUE; x.fillText("east, then north", O.x + DX * 0.35, O.y + 20);
        x.fillStyle = MAGENTA; x.fillText("north, then east", O.x - 128, O.y - DY * 0.5);
        if (frac >= 1) {
          var A = ep.a[2], B = ep.b[2];
          var gap = Math.hypot(A.x - B.x, A.y - B.y);
          [["A", A, BLUE], ["B", B, MAGENTA]].forEach(function (p) {
            x.fillStyle = p[2]; x.beginPath(); x.arc(p[1].x, p[1].y, 6, 0, 6.2832); x.fill();
          });
          if (gap > 1.5) {
            x.strokeStyle = RED; x.lineWidth = 2.5; x.setLineDash([5, 4]);
            x.beginPath(); x.moveTo(A.x, A.y); x.lineTo(B.x, B.y); x.stroke();
            x.setLineDash([]); x.lineWidth = 1;
            x.fillStyle = RED; x.font = "700 12px system-ui,sans-serif";
            x.fillText("the asymmetric carry", (A.x + B.x) / 2 + 10, (A.y + B.y) / 2);
          } else {
            x.fillStyle = GREEN; x.font = "700 12px system-ui,sans-serif";
            x.fillText("closed — no torsion", A.x + 12, A.y);
          }
          rd.gap.set(gap.toFixed(1) + " px");
          rd.gap.mark(gap > 1.5);
          if (animT >= 0) animT = -1;
        }
      }
      var loop = makeLoop(panel, function () {
        if (animT < 0) { loop.stop(); return; }
        animT += 2; draw();
      });
      var r1 = row();
      r1.appendChild(button("Walk both orders", function () { animT = 0; loop.start(); }));
      var sl = slider(panel, "torsion τ", 0, 0.6, 0.02, 0,
        function (v) { return v.toFixed(2); },
        function (v) { tau = v; draw(); });
      r1.appendChild(sl.el);
      panel.appendChild(r1);
      panel.appendChild(C.cv);
      var stats = el("div", "gbviz-stats");
      stats.appendChild(rd.gap.el);
      panel.appendChild(stats);
      panel.appendChild(note("At τ = 0 the two orders commute and the parallelogram closes — " +
        "that's the substrate that “refuses to compute” torsion, and measuring an exact " +
        "zero on it is half of E12. The other half is the connection that learns a real τ from " +
        "asymmetric data: update A-then-B, update B-then-A, subtract. The red dash is that " +
        "subtraction, drawn."));
      draw();
    }
  };

  /* =============================================================
   * Part V — The Theorem: receipts you can trust.
   * shard equality, then honest error bars.
   * ============================================================= */

  /* ch13 — the sharded answer must equal the unsharded one. */
  VIZ[13] = {
    title: "cut it, merge it, get the same number",
    sub: "One dataset of 512 readings, one aggregate (mean via Welford). Slide the shard count: the " +
      "data is cut, each shard folds its own accumulator, and the merged result is compared to the " +
      "single-node truth bit-for-bit. Then flip on the sloppy merge — averaging the shard means " +
      "without their weights — and watch unequal shards break the theorem by a number the receipt " +
      "can name.",
    make: function (panel) {
      var C = makeCanvas(840, 270);
      var rng = mulberry32(13), N = 512, data = [];
      for (var i = 0; i < N; i++)
        data.push(20.3 + Math.sin(i / 37) * 0.4 + (rng() - 0.5) * 0.5 + (i > 480 ? 3.0 : 0));
      var truth = welford(data);
      function welford(a) {
        var st = { n: 0, mean: 0, m2: 0 };
        a.forEach(function (v) {
          st.n++;
          if (st.n === 1) { st.mean = v; st.m2 = 0; }
          else { var d = v - st.mean; st.mean += d / st.n; st.m2 += d * (v - st.mean); }
        });
        return st;
      }
      function mergeW(a, b) {
        if (!a.n) return b;
        var n = a.n + b.n, d = b.mean - a.mean;
        return { n: n, mean: a.mean + d * b.n / n,
                 m2: a.m2 + b.m2 + d * d * a.n * b.n / n };
      }
      var k = 4, sloppy = false;
      var rd = { merged: readout("merged mean"), truth: readout("single-node mean"),
                 delta: readout("|delta|") };
      function draw() {
        // unequal cut: shard i gets a chunk proportional to i+1
        var weights = [], tot = 0;
        for (var i = 0; i < k; i++) { weights.push(i + 1); tot += i + 1; }
        var shards = [], at = 0;
        for (var i = 0; i < k; i++) {
          var take = i === k - 1 ? N - at : Math.round(N * weights[i] / tot);
          shards.push(data.slice(at, at + take)); at += take;
        }
        var accs = shards.map(welford);
        var merged;
        if (sloppy) {
          var m = 0; accs.forEach(function (a) { m += a.mean; });
          merged = { n: N, mean: m / k };
        } else {
          merged = accs.reduce(mergeW, { n: 0, mean: 0, m2: 0 });
        }
        var delta = Math.abs(merged.mean - truth.mean);
        var exact = delta < 1e-12;
        var x = C.ctx, W = C.W, H = C.H;
        x.clearRect(0, 0, W, H);
        // shard bars
        var bx = 24, bw = W - 48, by = 34;
        var atX = bx;
        accs.forEach(function (a, i) {
          var w = bw * a.n / N;
          x.fillStyle = ["#2a78d6", "#1baf7a", "#c98500", "#d55181", "#4a3aa7", "#e34948", "#008300", "#807f76"][i % 8];
          x.fillRect(atX, by, w - 3, 44);
          x.fillStyle = "#fff"; x.font = "700 11px ui-monospace,monospace"; x.textAlign = "center";
          if (w > 56) {
            x.fillText("n=" + a.n, atX + w / 2, by + 18);
            x.fillText("μ=" + a.mean.toFixed(3), atX + w / 2, by + 34);
          }
          x.textAlign = "left";
          atX += w;
        });
        x.fillStyle = INK3; x.font = "11px system-ui,sans-serif";
        x.fillText("512 readings cut into " + k + " deliberately unequal shards, each folding its own (n, μ, M₂)", bx, by - 8);
        // merge funnel
        var my = by + 88;
        x.strokeStyle = INK3;
        accs.forEach(function (a, i) {
          var cx = bx + bw * (i + 0.5) / k;
          x.beginPath(); x.moveTo(cx, by + 82); x.lineTo(W / 2, my + 26); x.stroke();
        });
        x.fillStyle = sloppy ? RED : GREEN;
        x.fillRect(W / 2 - 150, my + 26, 300, 40);
        x.fillStyle = "#fff"; x.font = "700 13px ui-monospace,monospace"; x.textAlign = "center";
        x.fillText(sloppy ? "sloppy: mean of means" : "merge: weighted Welford fold", W / 2, my + 44);
        x.fillText("μ = " + merged.mean.toFixed(10), W / 2, my + 60);
        x.textAlign = "left";
        // verdict
        x.font = "700 14px system-ui,sans-serif";
        x.fillStyle = exact ? GREEN : RED;
        x.fillText(exact ? "✓ equal to the single-node answer, bit for bit"
          : "✗ off by " + delta.toExponential(3) + " — the theorem just failed you",
          W / 2 - 150, my + 92);
        rd.merged.set(merged.mean.toFixed(10));
        rd.truth.set(truth.mean.toFixed(10));
        rd.delta.set(exact ? "0 (exact)" : delta.toExponential(3));
        rd.delta.mark(!exact);
      }
      var r1 = row();
      var sl = slider(panel, "shards k", 1, 8, 1, 4,
        function (v) { return v; }, function (v) { k = v; draw(); });
      r1.appendChild(sl.el);
      var bGood = button("Weighted fold", function () { sloppy = false; sync(); });
      var bBad = button("Sloppy merge", function () { sloppy = true; sync(); }, true);
      function sync() {
        bGood.className = "gbviz-btn" + (sloppy ? " alt" : "");
        bBad.className = "gbviz-btn" + (sloppy ? "" : " alt");
        draw();
      }
      r1.appendChild(bGood); r1.appendChild(bBad);
      panel.appendChild(r1);
      panel.appendChild(C.cv);
      var stats = el("div", "gbviz-stats");
      [rd.merged, rd.truth, rd.delta].forEach(function (r) { stats.appendChild(r.el); });
      panel.appendChild(stats);
      panel.appendChild(note("The weighted fold is exact at every k — that's the trivial-atlas " +
        "guarantee E13 starts from, and it's why the shards can be as unequal as you like. The " +
        "sloppy merge is only wrong when the shards are unequal <em>and</em> the data drifts — " +
        "which is exactly the case distributed systems live in. Set k = 1 and even the sloppy " +
        "merge is right: every bad estimator is exact on the atlas with one chart."));
      draw();
    }
  };

  /* ch14 — honest error bars: naive vs blocked on AR(1) data. */
  VIZ[14] = {
    title: "the error bar that tells the truth",
    sub: "An AR(1) stream — each sample remembers the last with strength φ. Both error bars are " +
      "computed from the same 256 samples: the naive one assumes independence, the honest one " +
      "blocks the data at ~2τ and prices the correlation (the engine's <code>WITH JACKKNIFE</code> " +
      "math). Redraw many runs and count coverage: the naive bar misses the true mean far more " +
      "than its 95% promise. Disclosure beats optimism.",
    builds: 13,
    make: function (panel) {
      var C = makeCanvas(840, 300);
      var rngState = 14, phi = 0.8, runs = [], cover = { naive: 0, honest: 0, n: 0 };
      function draw256() {
        var rng = mulberry32(rngState++);
        function gauss() {
          var u = Math.max(rng(), 1e-12), v = rng();
          return Math.sqrt(-2 * Math.log(u)) * Math.cos(6.2832 * v);
        }
        var n = 256, xs = [], x0 = 0;
        for (var i = 0; i < 40; i++) x0 = phi * x0 + gauss(); // burn-in
        for (var i = 0; i < n; i++) { x0 = phi * x0 + gauss(); xs.push(x0); }
        var mean = 0; xs.forEach(function (v) { mean += v; }); mean /= n;
        var s2 = 0; xs.forEach(function (v) { s2 += (v - mean) * (v - mean); }); s2 /= (n - 1);
        var errNaive = Math.sqrt(s2 / n);
        // blocked: block size ≈ 2τ_int, τ_int = (1+φ)/(2(1−φ)) analytic
        var tau = (1 + phi) / (2 * (1 - phi));
        var B = Math.max(1, Math.min(n >> 2, Math.round(2 * tau)));
        var nb = Math.floor(n / B), bm = [];
        for (var b = 0; b < nb; b++) {
          var s = 0;
          for (var j = 0; j < B; j++) s += xs[b * B + j];
          bm.push(s / B);
        }
        var bmean = 0; bm.forEach(function (v) { bmean += v; }); bmean /= nb;
        var bs2 = 0; bm.forEach(function (v) { bs2 += (v - bmean) * (v - bmean); });
        bs2 /= (nb - 1);
        var errHonest = Math.sqrt(bs2 / nb);
        return { mean: mean, naive: errNaive, honest: errHonest, tau: tau };
      }
      function addRun() {
        var r = draw256();
        runs.push(r); if (runs.length > 40) runs.shift();
        cover.n++;
        if (Math.abs(r.mean) <= 1.96 * r.naive) cover.naive++;
        if (Math.abs(r.mean) <= 1.96 * r.honest) cover.honest++;
      }
      var rd = { tau: readout("τ_int (analytic)"), cn: readout("naive coverage"),
                 ch: readout("honest coverage") };
      function draw() {
        var x = C.ctx, W = C.W, H = C.H;
        x.clearRect(0, 0, W, H);
        var padL = 30, plotW = W - padL - 16, cy0 = 20, rowH = Math.min(26, (H - 70) / Math.max(runs.length, 1));
        // true mean line
        var scale = 0.9; // x units → px: mean range ~ [-scale, scale]
        function X(v) { return padL + plotW * (0.5 + v / (2 * scale)); }
        x.strokeStyle = INK; x.setLineDash([4, 4]); x.beginPath();
        x.moveTo(X(0), 10); x.lineTo(X(0), H - 46); x.stroke(); x.setLineDash([]);
        x.fillStyle = INK3; x.font = "11px system-ui,sans-serif";
        x.fillText("true mean = 0", X(0) + 6, 18);
        runs.forEach(function (r, i) {
          var y = cy0 + i * rowH;
          // naive bar (thin, above) and honest bar (thick, below)
          var missN = Math.abs(r.mean) > 1.96 * r.naive;
          var missH = Math.abs(r.mean) > 1.96 * r.honest;
          x.strokeStyle = missN ? RED : INK3; x.lineWidth = 2;
          x.beginPath(); x.moveTo(X(r.mean - 1.96 * r.naive), y - 3);
          x.lineTo(X(r.mean + 1.96 * r.naive), y - 3); x.stroke();
          x.strokeStyle = missH ? RED : VIOLET; x.lineWidth = 4;
          x.beginPath(); x.moveTo(X(r.mean - 1.96 * r.honest), y + 4);
          x.lineTo(X(r.mean + 1.96 * r.honest), y + 4); x.stroke();
          x.lineWidth = 1;
          x.fillStyle = INK;
          x.beginPath(); x.arc(X(r.mean), y, 2.6, 0, 6.2832); x.fill();
        });
        // legend
        x.font = "11.5px system-ui,sans-serif";
        x.strokeStyle = INK3; x.lineWidth = 2;
        x.beginPath(); x.moveTo(padL, H - 30); x.lineTo(padL + 30, H - 30); x.stroke();
        x.fillStyle = INK2; x.fillText("naive ±1.96·σ/√n (assumes independence)", padL + 38, H - 26);
        x.strokeStyle = VIOLET; x.lineWidth = 4;
        x.beginPath(); x.moveTo(padL + 320, H - 30); x.lineTo(padL + 350, H - 30); x.stroke();
        x.lineWidth = 1;
        x.fillStyle = INK2; x.fillText("blocked at 2τ (the JACKKNIFE math)", padL + 358, H - 26);
        var tau = (1 + phi) / (2 * (1 - phi));
        rd.tau.set(tau.toFixed(2) + (Math.abs(phi - 0.8) < 1e-9 ? "  (φ=0.8 → 4.5)" : ""));
        rd.cn.set(cover.n ? Math.round(100 * cover.naive / cover.n) + "% of " + cover.n : "–");
        rd.cn.mark(cover.n > 10 && cover.naive / cover.n < 0.9);
        rd.ch.set(cover.n ? Math.round(100 * cover.honest / cover.n) + "% of " + cover.n : "–");
      }
      var r1 = row();
      r1.appendChild(button("Draw a run", function () { addRun(); draw(); }));
      r1.appendChild(button("Draw 25 runs", function () {
        for (var i = 0; i < 25; i++) addRun(); draw();
      }));
      var sl = slider(panel, "correlation φ", 0, 0.95, 0.05, 0.8,
        function (v) { return v.toFixed(2); },
        function (v) { phi = v; runs = []; cover = { naive: 0, honest: 0, n: 0 }; draw(); });
      r1.appendChild(sl.el);
      r1.appendChild(button("reset", function () {
        runs = []; cover = { naive: 0, honest: 0, n: 0 }; draw();
      }, true));
      panel.appendChild(r1);
      panel.appendChild(C.cv);
      var stats = el("div", "gbviz-stats");
      [rd.tau, rd.cn, rd.ch].forEach(function (r) { stats.appendChild(r.el); });
      panel.appendChild(stats);
      panel.appendChild(note("At φ = 0.8, τ_int = 4.5 and the naive bar is √(2τ) ≈ 3× too " +
        "short — its “95%” coverage lands near 60%. Set φ = 0 and the two bars agree: " +
        "honesty costs nothing on independent data. This is the exact machinery behind " +
        "<code>INTEGRATE … WITH JACKKNIFE ALONG order</code> — try it in the console above and " +
        "compare <code>err</code> to <code>err_naive</code> on real bundle data."));
      addRun(); addRun(); addRun(); draw();
    }
  };

  /* =============================================================
   * Part VI — Gauge Encryption: one cloud, three frames.
   * ============================================================= */

  // shared Part VI data: a 60-point "G" glyph cloud
  function gGlyph() {
    var pts = [];
    // arc of the G
    for (var i = 0; i < 40; i++) {
      var a = 0.5 + i / 39 * 5.0;
      pts.push([Math.cos(a) * 80, Math.sin(a) * 80]);
    }
    // crossbar
    for (var i = 0; i < 12; i++) pts.push([10 + i * 6, 8]);
    for (var i = 0; i < 8; i++) pts.push([76, 16 + i * 8]);
    return pts;
  }
  function gaugeApply(pts, theta, tx, ty) {
    var c = Math.cos(theta), s = Math.sin(theta);
    return pts.map(function (p) {
      return [c * p[0] - s * p[1] + tx, s * p[0] + c * p[1] + ty];
    });
  }
  function invariantTuple(pts) {
    var n = pts.length, sum = 0, sum2 = 0, mn = Infinity, mx = 0;
    for (var i = 0; i < n; i++) for (var j = i + 1; j < n; j++) {
      var d = Math.hypot(pts[i][0] - pts[j][0], pts[i][1] - pts[j][1]);
      sum += d; sum2 += d * d;
      if (d < mn) mn = d; if (d > mx) mx = d;
    }
    return { n: n, sumD: sum, sumD2: sum2, minD: mn, maxD: mx };
  }

  /* ch15 — the gauge dial: your frame vs the wire frame. */
  VIZ[15] = {
    title: "readable here, noise there",
    sub: "Sixty fiber points that spell something — in <strong>your frame</strong>. The wire carries " +
      "the same points pushed through a gauge transformation g (a rotation + shift here; the real " +
      "thing composes more). Spin the dial: the wire view scrambles, but the geometry — every " +
      "pairwise distance — never moves. The key isn't a password on top of the data; the key is " +
      "<em>which frame you're standing in</em>.",
    make: function (panel) {
      var C = makeCanvas(840, 300);
      var pts = gGlyph(), theta = 2.2, drift = { x: 40, y: -20 };
      var inv0 = invariantTuple(pts);
      var rd = { d2: readout("Σd² (your frame)"), d2w: readout("Σd² (wire frame)") };
      function draw() {
        var x = C.ctx, W = C.W, H = C.H;
        x.clearRect(0, 0, W, H);
        var cxL = W * 0.27, cxR = W * 0.73, cy = H / 2 + 8;
        x.strokeStyle = LINE; x.beginPath();
        x.moveTo(W / 2, 12); x.lineTo(W / 2, H - 12); x.stroke();
        x.font = "700 12.5px system-ui,sans-serif";
        x.fillStyle = INK; x.fillText("your frame (key held)", cxL - 70, 22);
        x.fillStyle = INK2; x.fillText("the wire frame (what an eavesdropper sees)", cxR - 140, 22);
        pts.forEach(function (p) {
          x.fillStyle = RED;
          x.beginPath(); x.arc(cxL + p[0] * 0.9, cy + p[1] * 0.9, 3, 0, 6.2832); x.fill();
        });
        var wire = gaugeApply(pts, theta, drift.x, drift.y);
        wire.forEach(function (p) {
          x.fillStyle = INK2;
          x.beginPath(); x.arc(cxR + p[0] * 0.9, cy + p[1] * 0.9, 3, 0, 6.2832); x.fill();
        });
        var invW = invariantTuple(wire);
        rd.d2.set(inv0.sumD2.toExponential(6));
        rd.d2w.set(invW.sumD2.toExponential(6));
      }
      var r1 = row();
      var sl = slider(panel, "gauge angle θ", 0, 6.28, 0.02, 2.2,
        function (v) { return v.toFixed(2) + " rad"; },
        function (v) { theta = v; draw(); });
      r1.appendChild(sl.el);
      var sl2 = slider(panel, "gauge shift", -120, 120, 4, 40,
        function (v) { return v; },
        function (v) { drift.x = v; drift.y = -v / 2; draw(); });
      r1.appendChild(sl2.el);
      r1.appendChild(button("θ = 0 (drop the gauge)", function () {
        theta = 0; drift = { x: 0, y: 0 };
        sl.input.value = 0; sl.out.textContent = "0.00 rad";
        sl2.input.value = 0; sl2.out.textContent = "0";
        draw();
      }, true));
      panel.appendChild(r1);
      panel.appendChild(C.cv);
      var stats = el("div", "gbviz-stats");
      [rd.d2, rd.d2w].forEach(function (r) { stats.appendChild(r.el); });
      panel.appendChild(stats);
      panel.appendChild(note("The two Σd² readouts agree to every digit at any dial position — " +
        "distances are gauge-invariant, which is why the engine can still do geometry (κ, " +
        "spectra, anomaly pricing) on data it cannot read. A rotation in 2D is a toy key; " +
        "the chapter's five modes compose shifts, rotations, and per-field scrambles until the " +
        "frame is unguessable — but this picture is the whole idea."));
      draw();
    }
  };

  /* ch16 — the verifier without decryption: Carol's clipboard. */
  VIZ[16] = {
    title: "Carol checks without reading",
    sub: "Carol holds no key. She receives the wire-frame cloud from Chapter 15 plus the sender's " +
      "signed invariant tuple, recomputes the tuple on the encrypted points, and compares. Verify " +
      "the honest shipment, then tamper with one point and verify again — she catches it and names " +
      "the field that moved, all without ever seeing the plaintext.",
    builds: 15,
    make: function (panel) {
      var C = makeCanvas(840, 300);
      var pts = gGlyph();
      var claimed = invariantTuple(pts);
      var wire, tampered, verdict = null;
      function reship() {
        wire = gaugeApply(pts, 2.2, 40, -20); tampered = false; verdict = null; draw();
      }
      function draw() {
        var x = C.ctx, W = C.W, H = C.H;
        x.clearRect(0, 0, W, H);
        var cxL = W * 0.25, cy = H / 2 + 6;
        wire.forEach(function (p, i) {
          x.fillStyle = tampered && i === 7 ? RED : INK2;
          x.beginPath();
          x.arc(cxL + p[0] * 0.85, cy + p[1] * 0.85, tampered && i === 7 ? 5 : 3, 0, 6.2832);
          x.fill();
        });
        x.fillStyle = INK3; x.font = "11.5px system-ui,sans-serif";
        x.fillText("the shipment (wire frame — Carol can't read it)", 30, 22);
        // clipboard
        var bx = W * 0.52, by = 40, bw = W * 0.44;
        x.strokeStyle = LINE; x.strokeRect(bx, by, bw, 200);
        x.fillStyle = INK; x.font = "700 13px system-ui,sans-serif";
        x.fillText("Carol's clipboard — InvariantTuple", bx + 14, by + 24);
        var got = invariantTuple(wire);
        // the 10⁻¹⁰ grain: recomputation on the honest shipment agrees to
        // floating-point roundoff; any pixel-scale tamper is 10⁶× louder
        var fields = [["n", "n", 0], ["Σd", "sumD", 1e-10], ["Σd²", "sumD2", 1e-10],
                      ["min d", "minD", 1e-10], ["max d", "maxD", 1e-10]];
        x.font = "12px ui-monospace,Menlo,monospace";
        fields.forEach(function (f, i) {
          var yy = by + 52 + i * 24;
          var ok = Math.abs(got[f[1]] - claimed[f[1]]) <= f[2] * Math.max(1, Math.abs(claimed[f[1]]));
          x.fillStyle = INK2;
          x.fillText(f[0], bx + 14, yy);
          x.fillText("claimed " + fmt(claimed[f[1]]), bx + 76, yy);
          x.fillStyle = verdict === null ? INK3 : ok ? GREEN : RED;
          x.fillText(verdict === null ? "recomputed —" :
            (ok ? "✓ " : "✗ ") + fmt(got[f[1]]), bx + 250, yy);
        });
        function fmt(v) { return v > 9999 ? v.toExponential(4) : (+v.toFixed(4)).toString(); }
        if (verdict !== null) {
          x.font = "700 14px system-ui,sans-serif";
          x.fillStyle = verdict ? GREEN : RED;
          x.fillText(verdict ? "VerifyResult::Verified — geometry intact"
            : "VerifyResult::Rejected { field: \"sumD2\", … } — someone touched the shipment",
            30, H - 16);
        }
      }
      var r1 = row();
      r1.appendChild(button("Verify the shipment", function () {
        var got = invariantTuple(wire);
        verdict = Math.abs(got.sumD2 - claimed.sumD2) <= 1e-10 * claimed.sumD2 &&
                  Math.abs(got.sumD - claimed.sumD) <= 1e-10 * claimed.sumD;
        draw();
      }));
      r1.appendChild(button("Tamper with one point", function () {
        if (!tampered) { wire[7] = [wire[7][0] + 22, wire[7][1] - 14]; tampered = true; }
        verdict = null; draw();
      }));
      r1.appendChild(button("Fresh shipment", reship, true));
      panel.appendChild(r1);
      panel.appendChild(C.cv);
      panel.appendChild(note("The tamper moved one encrypted point by a few pixels and Σd² " +
        "named it — 59 of the 60 points' pairwise distances still agree, so the culprit is " +
        "locatable, not just detectable. E16's builds do this against the real " +
        "<code>InvariantTuple::compute</code> with the 10⁻¹⁰ grain: your forged deltas have to " +
        "slide under that, and the chapter's claim is they can't."));
      reship();
    }
  };

  /* ch17 — delegation & forward secrecy: the ratchet chain. */
  VIZ[17] = {
    title: "the ratchet only turns forward",
    sub: "Chapter 15's gauge, composed. Each epoch re-gauges the cloud with a fresh gᵢ, and a " +
      "reader's key at epoch k opens epochs k and earlier-composed views only as delegated. " +
      "Advance the ratchet a few epochs, then revoke at an epoch of your choice: everything " +
      "after the revocation stays noise to the old key — that's forward secrecy as geometry, " +
      "not policy.",
    builds: 15,
    make: function (panel) {
      var C = makeCanvas(840, 300);
      var pts = gGlyph();
      var epoch = 0, revokedAt = null;
      var rngG = mulberry32(17);
      var gauges = []; // cumulative params per epoch
      function ensure(n) {
        while (gauges.length <= n)
          gauges.push({ theta: rngG() * 6.28, tx: (rngG() - 0.5) * 120, ty: (rngG() - 0.5) * 80 });
      }
      function viewAt(e, keyEpoch) {
        // a key from keyEpoch can undo gauges up to keyEpoch; epochs after stay composed
        var p = pts;
        for (var i = keyEpoch + 1; i <= e; i++) {
          ensure(i);
          p = gaugeApply(p, gauges[i].theta, gauges[i].tx, gauges[i].ty);
        }
        return p;
      }
      function draw() {
        var x = C.ctx, W = C.W, H = C.H;
        x.clearRect(0, 0, W, H);
        // ratchet chain
        var chainY = 34;
        for (var e = 0; e <= Math.max(epoch, 5); e++) {
          var cx = 60 + e * 90;
          if (cx > W - 40) break;
          var live = e <= epoch;
          var locked = revokedAt !== null && e > revokedAt;
          x.fillStyle = !live ? "#fdfcf8" : locked ? RED : GREEN;
          x.strokeStyle = live ? (locked ? RED : GREEN) : LINE;
          x.beginPath(); x.arc(cx, chainY, 15, 0, 6.2832); x.fill(); x.stroke();
          x.fillStyle = live ? "#fff" : INK3;
          x.font = "700 11px ui-monospace,monospace"; x.textAlign = "center";
          x.fillText("g" + e, cx, chainY + 4); x.textAlign = "left";
          if (e > 0) {
            x.strokeStyle = live ? INK2 : LINE;
            x.beginPath(); x.moveTo(cx - 75 + 15, chainY); x.lineTo(cx - 15, chainY); x.stroke();
          }
        }
        x.fillStyle = INK3; x.font = "11px system-ui,sans-serif";
        x.fillText("epoch " + epoch + (revokedAt !== null ? "  ·  old key revoked after epoch " + revokedAt : "  ·  no revocation yet"), 30, 64);
        // two readers
        var keyNew = epoch, keyOld = revokedAt === null ? epoch : revokedAt;
        [["reader with the CURRENT key (epoch " + keyNew + ")", keyNew, W * 0.27],
         ["reader with the OLD key (epoch " + keyOld + ")", keyOld, W * 0.73]].forEach(function (r) {
          var view = viewAt(epoch, r[1]);
          var cy = H / 2 + 40;
          x.fillStyle = INK2; x.font = "12px system-ui,sans-serif";
          x.fillText(r[0], r[2] - 130, 96);
          var readable = r[1] >= epoch;
          view.forEach(function (p) {
            x.fillStyle = readable ? RED : INK3;
            x.beginPath(); x.arc(r[2] + p[0] * 0.62, cy + p[1] * 0.62, 2.6, 0, 6.2832); x.fill();
          });
          x.fillStyle = readable ? GREEN : RED; x.font = "700 12px system-ui,sans-serif";
          x.fillText(readable ? "✓ reads plaintext" : "✗ sees composed noise", r[2] - 60, H - 14);
        });
      }
      var r1 = row();
      r1.appendChild(button("Advance the ratchet (new epoch)", function () {
        epoch++; ensure(epoch); draw();
      }));
      r1.appendChild(button("Revoke the old key here", function () {
        revokedAt = epoch; epoch++; ensure(epoch); draw();
      }));
      r1.appendChild(button("reset", function () { epoch = 0; revokedAt = null; draw(); }, true));
      panel.appendChild(r1);
      panel.appendChild(C.cv);
      panel.appendChild(note("The old key isn't <em>denied</em> anything — it simply stops being " +
        "the frame the data lives in. Each advance composes one more gauge the old key can't " +
        "undo, and composition doesn't commute backwards: that's the RG-flow ratchet in one " +
        "picture. E17's builds do this with the four delegation seals and measure what the " +
        "chapter concedes: which parts of the story are not PQ-safe yet."));
      draw();
    }
  };

  /* =============================================================
   * Part VII — Running the Engine: the keeper's console.
   * ============================================================= */

  /* ch18 — bring-up sequence + click-gated live HEALTH ping. */
  VIZ[18] = {
    title: "the keeper's console",
    sub: "The bring-up every operator watches: open the store, replay the WAL tail, map the " +
      "snapshot, init the app bundles, flip ready. Run it and read the latencies. Then — this one " +
      "is real — ping the public engine and get a live <code>HEALTH</code> receipt from the same " +
      "instance the consoles on every chapter page talk to.",
    make: function (panel) {
      var C = makeCanvas(840, 240);
      var STAGES = [
        { name: "open store", ms: [2, 6] },
        { name: "replay WAL tail", ms: [8, 40] },
        { name: "mmap snapshot", ms: [1, 4] },
        { name: "init_app_bundles", ms: [12, 30] },
        { name: "ready flip", ms: [0, 1] }
      ];
      var rng = mulberry32(18), state = STAGES.map(function () { return { done: false, ms: 0 }; });
      var running = -1, health = null, healthErr = null;
      var loop = makeLoop(panel, function (t) {
        if (running < 0) { loop.stop(); return; }
        if (t % 14 === 13) {
          var s = STAGES[running];
          state[running] = { done: true, ms: s.ms[0] + rng() * (s.ms[1] - s.ms[0]) };
          running++;
          if (running >= STAGES.length) running = -1;
        }
        draw();
      });
      function draw() {
        var x = C.ctx, W = C.W, H = C.H;
        x.clearRect(0, 0, W, H);
        var total = 0;
        STAGES.forEach(function (s, i) {
          var y = 30 + i * 34;
          var st = state[i];
          x.fillStyle = st.done ? GREEN : running === i ? YELLOW : "#fdfcf8";
          x.strokeStyle = st.done ? GREEN : running === i ? YELLOW : LINE;
          x.beginPath(); x.arc(40, y, 9, 0, 6.2832); x.fill(); x.stroke();
          if (st.done) {
            x.fillStyle = "#fff"; x.font = "700 11px system-ui,sans-serif"; x.textAlign = "center";
            x.fillText("✓", 40, y + 4); x.textAlign = "left";
          }
          x.fillStyle = INK; x.font = "13px ui-monospace,Menlo,monospace";
          x.fillText(s.name, 62, y + 4);
          if (st.done) {
            total += st.ms;
            x.fillStyle = INK2;
            x.fillText(st.ms.toFixed(1) + " ms", 250, y + 4);
            x.fillStyle = AQUA;
            x.fillRect(320, y - 6, st.ms * 3.2, 12);
          }
        });
        if (state[4].done) {
          x.fillStyle = GREEN; x.font = "700 13px system-ui,sans-serif";
          x.fillText("READY in " + total.toFixed(1) + " ms — first query answerable now, no warmup (the snapshot carried the geometry)", 40, 210);
        } else {
          x.fillStyle = INK3; x.font = "11.5px system-ui,sans-serif";
          x.fillText("a 503 before the ready flip is correct behavior — E18 captures that body as a receipt", 40, 210);
        }
        // live health, right column
        var hx = 560;
        x.fillStyle = INK; x.font = "700 12.5px system-ui,sans-serif";
        x.fillText("live HEALTH — gigi-stream.fly.dev", hx, 26);
        x.font = "12px ui-monospace,Menlo,monospace";
        if (healthErr) {
          x.fillStyle = RED;
          wrapText(x, healthErr, hx, 48, W - hx - 16, 16);
        } else if (health) {
          x.fillStyle = INK2;
          health.slice(0, 9).forEach(function (line, i) {
            x.fillText(line, hx, 48 + i * 18);
          });
        } else {
          x.fillStyle = INK3;
          x.fillText("(press the button — one request,", hx, 48);
          x.fillText(" read-only, no key needed)", hx, 66);
        }
      }
      function wrapText(x, txt, tx, ty, maxW, lh) {
        var words = txt.split(" "), line = "", y = ty;
        words.forEach(function (w) {
          if (x.measureText(line + w).width > maxW) { x.fillText(line, tx, y); line = w + " "; y += lh; }
          else line += w + " ";
        });
        x.fillText(line, tx, y);
      }
      var r1 = row();
      r1.appendChild(button("Run bring-up", function () {
        state = STAGES.map(function () { return { done: false, ms: 0 }; });
        running = 0; loop.start();
      }));
      var bPing = button("Ping the real engine (HEALTH tetmesh_demo)", function () {
        bPing.disabled = true; health = null; healthErr = null; draw();
        fetch("https://gigi-stream.fly.dev/v1/public/gql", {
          method: "POST", headers: { "Content-Type": "application/json" },
          body: JSON.stringify({ query: "HEALTH tetmesh_demo;" })
        }).then(function (r) { return r.json(); }).then(function (j) {
          var rows = j.rows || j.result || j;
          var lines = [];
          (Array.isArray(rows) ? rows : [rows]).slice(0, 4).forEach(function (row) {
            Object.keys(row).slice(0, 9).forEach(function (k) {
              var v = row[k];
              lines.push(k + " = " + (typeof v === "number" ? +v.toFixed(6) : v));
            });
          });
          health = lines.length ? lines : [JSON.stringify(j).slice(0, 60)];
          bPing.disabled = false; draw();
        }).catch(function (e) {
          healthErr = "unreachable: " + e + " — fine offline; the bring-up story stands";
          bPing.disabled = false; draw();
        });
      });
      r1.appendChild(bPing);
      panel.appendChild(r1);
      panel.appendChild(C.cv);
      panel.appendChild(note("Six parts of instruments led here: the shape (I), the machine " +
        "(II), the signals (III), the memory (IV), the theorem (V), the lock (VI) — and the " +
        "keeper's job is to keep all of it answering. The HEALTH numbers on the right are the " +
        "real engine's κ and spectral state for the tetmesh bundle from the Web-Extra demo, " +
        "measured the moment you pressed the button. Your relief shift gets receipts, not vibes."));
      draw();
    }
  };

  /* ---------- boot ---------- */

  function boot() {
    var def = VIZ[CH];
    if (!def) return;
    try {
      var panel = mountPanel(def);
      def.make(panel);
    } catch (e) {
      // an instrument must never take the page down with it
      if (window.console && console.error) console.error("gbviz:", e);
    }
  }
  if (document.readyState === "loading")
    document.addEventListener("DOMContentLoaded", boot);
  else boot();
})();
