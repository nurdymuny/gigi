/* GIGI Builds — shared helpers for the 3D web-extra demos.
 * Load AFTER assets/three.min.js (exposes window.THREE) and BEFORE the
 * page's own demo script. Everything hangs off window.WX. No page DOM
 * assumptions beyond the elements each helper is handed.
 */
(function () {
  "use strict";
  var WX = window.WX = {};

  /* deterministic PRNG — same one the exercise instruments use */
  WX.mulberry32 = function (seed) {
    var a = seed >>> 0;
    return function () {
      a |= 0; a = (a + 0x6D2B79F5) | 0;
      var t = Math.imul(a ^ (a >>> 15), 1 | a);
      t = (t + Math.imul(t ^ (t >>> 7), 61 | t)) ^ t;
      return ((t ^ (t >>> 14)) >>> 0) / 4294967296;
    };
  };

  WX.PAL = { blue: 0x3987e5, aqua: 0x199e70, yellow: 0xc98500, orange: 0xd95926,
             red: 0xe66767, magenta: 0xd55181, violet: 0x9085e9,
             ink: 0xe8e6e0, ink3: 0x85827a, line: 0x3a3934 };

  /* round sprite for THREE.Points — square points read as pixel dust */
  WX.dotTexture = function () {
    if (WX._dotTex) return WX._dotTex;
    var cv = document.createElement("canvas");
    cv.width = cv.height = 64;
    var x = cv.getContext("2d");
    var g = x.createRadialGradient(32, 32, 0, 32, 32, 30);
    g.addColorStop(0, "rgba(255,255,255,1)");
    g.addColorStop(0.7, "rgba(255,255,255,1)");
    g.addColorStop(1, "rgba(255,255,255,0)");
    x.fillStyle = g;
    x.beginPath(); x.arc(32, 32, 30, 0, 6.2832); x.fill();
    WX._dotTex = new THREE.CanvasTexture(cv);
    return WX._dotTex;
  };
  WX.dots = function (geo, opts) {
    opts = opts || {};
    var mat = new THREE.PointsMaterial({
      size: opts.size || 0.03,
      sizeAttenuation: true,
      map: WX.dotTexture(),
      transparent: true,
      alphaTest: 0.35,
      depthWrite: false
    });
    if (opts.color !== undefined) mat.color = new THREE.Color(opts.color);
    if (opts.vertexColors) mat.vertexColors = true;
    if (opts.opacity !== undefined) mat.opacity = opts.opacity;
    return new THREE.Points(geo, mat);
  };

  /* renderer + scene + camera + rAF loop that pauses offscreen/hidden */
  WX.stage = function (container, opts) {
    opts = opts || {};
    var W = opts.width || 1120, H = opts.height || 620;
    var renderer = new THREE.WebGLRenderer({ antialias: true });
    renderer.setPixelRatio(Math.min(2, window.devicePixelRatio || 1));
    renderer.setSize(W, H, false);
    renderer.domElement.style.width = "100%";
    renderer.domElement.style.height = "auto";
    renderer.domElement.setAttribute("aria-label", opts.label || "3D visualization");
    renderer.domElement.setAttribute("role", "img");
    container.appendChild(renderer.domElement);
    var scene = new THREE.Scene();
    scene.background = new THREE.Color(0x141413);
    var camera = new THREE.PerspectiveCamera(opts.fov || 45, W / H, 0.01, 100);
    var visible = true, running = true, raf = 0;
    var ticks = [];
    function frame(t) {
      raf = 0;
      if (!running || !visible || document.hidden) return;
      for (var i = 0; i < ticks.length; i++) ticks[i](t);
      renderer.render(scene, camera);
      raf = requestAnimationFrame(frame);
    }
    function kick() { if (!raf && running && visible && !document.hidden) raf = requestAnimationFrame(frame); }
    if ("IntersectionObserver" in window) {
      new IntersectionObserver(function (es) { visible = es[0].isIntersecting; kick(); })
        .observe(container);
    }
    document.addEventListener("visibilitychange", kick);
    kick();
    return { renderer: renderer, scene: scene, camera: camera, W: W, H: H,
             onTick: function (fn) { ticks.push(fn); },
             kick: kick,
             stop: function () { running = false; },
             start: function () { running = true; kick(); } };
  };

  /* minimal orbit: drag rotates, wheel zooms; no dependencies */
  WX.orbit = function (stage, opts) {
    opts = opts || {};
    var el = stage.renderer.domElement;
    var state = {
      theta: opts.theta !== undefined ? opts.theta : 0.6,
      phi: opts.phi !== undefined ? opts.phi : 1.15,
      dist: opts.dist || 4,
      target: opts.target || new THREE.Vector3(0, 0, 0),
      minDist: opts.minDist || 1.4, maxDist: opts.maxDist || 12,
      enabled: true
    };
    if (opts.minTheta !== undefined) state.minTheta = opts.minTheta;
    if (opts.maxTheta !== undefined) state.maxTheta = opts.maxTheta;
    var dragging = false, px = 0, py = 0;
    function apply() {
      state.phi = Math.max(0.08, Math.min(Math.PI - 0.08, state.phi));
      if (state.minTheta !== undefined)
        state.theta = Math.max(state.minTheta, Math.min(state.maxTheta, state.theta));
      var sp = Math.sin(state.phi), cp = Math.cos(state.phi);
      stage.camera.position.set(
        state.target.x + state.dist * sp * Math.cos(state.theta),
        state.target.y + state.dist * cp,
        state.target.z + state.dist * sp * Math.sin(state.theta));
      stage.camera.lookAt(state.target);
    }
    el.addEventListener("pointerdown", function (e) {
      if (!state.enabled) return;
      dragging = true; px = e.clientX; py = e.clientY;
      el.setPointerCapture && el.setPointerCapture(e.pointerId);
    });
    el.addEventListener("pointermove", function (e) {
      if (!dragging || !state.enabled) return;
      state.theta += (e.clientX - px) * 0.006;
      state.phi -= (e.clientY - py) * 0.006;
      px = e.clientX; py = e.clientY;
      apply(); stage.kick();
    });
    el.addEventListener("pointerup", function () { dragging = false; });
    el.addEventListener("pointercancel", function () { dragging = false; });
    el.addEventListener("wheel", function (e) {
      if (!state.enabled) return;
      e.preventDefault();
      state.dist *= (1 + Math.sign(e.deltaY) * 0.08);
      state.dist = Math.max(state.minDist, Math.min(state.maxDist, state.dist));
      apply(); stage.kick();
    }, { passive: false });
    state.apply = apply;
    apply();
    return state;
  };

  /* control builders (mirror the exercise instruments) */
  WX.button = function (label, onClick, alt) {
    var b = document.createElement("button");
    b.type = "button"; b.className = alt ? "wx alt" : "wx"; b.textContent = label;
    b.addEventListener("click", onClick);
    return b;
  };
  WX.slider = function (label, min, max, step, val, fmt, onInput) {
    var wrap = document.createElement("label"); wrap.className = "wx-slider";
    var span = document.createElement("span");
    span.appendChild(document.createTextNode(label + " "));
    var out = document.createElement("strong"); out.textContent = fmt(val);
    span.appendChild(out);
    var inp = document.createElement("input");
    inp.type = "range"; inp.min = min; inp.max = max; inp.step = step; inp.value = val;
    inp.addEventListener("input", function () {
      var v = parseFloat(inp.value); out.textContent = fmt(v); onInput(v);
    });
    wrap.appendChild(span); wrap.appendChild(inp);
    return { el: wrap, input: inp, out: out };
  };
  WX.readout = function (label) {
    var box = document.createElement("div"); box.className = "stat";
    var v = document.createElement("div"); v.className = "v"; v.textContent = "–";
    var k = document.createElement("div"); k.className = "k"; k.textContent = label;
    box.appendChild(v); box.appendChild(k);
    return { el: box, set: function (t) { v.textContent = t; },
             tone: function (cls) { v.className = "v" + (cls ? " " + cls : ""); } };
  };

  /* Plain-english gloss for a GQL statement. Same rules the tetmesh
     demo uses; kept here so every web extra has the identical mapping
     between verbs and reader-facing sentences. */
  WX.gqlGloss = function (q) {
    var s = String(q || "").trim().replace(/;$/, "").trim(), m;
    if (/^SHOW\s+BUNDLES$/i.test(s))
      return "SHOW BUNDLES — asks the engine: what bundles do you have?";
    if ((m = s.match(/^HEALTH\s+(\w+)$/i)))
      return "HEALTH — quick vitals on " + m[1] + ": record count, curvature (κ), confidence";
    if ((m = s.match(/^SECTION\s+(\w+)\s+AT\s+(\w+)\s*=\s*['"]?([^'"]+)['"]?$/i)))
      return "SECTION AT — the O(1) lookup: fetch " + m[1] + " where " + m[2] + "=" + m[3];
    if ((m = s.match(/^COVER\s+(\w+)\s+ON\s+(\w+)\s*=\s*([\w.]+)$/i)))
      return "COVER — the range scan: every " + m[1] + " record where " + m[2] + " = " + m[3];
    if ((m = s.match(/^COVER\s+(\w+)\s+WHERE\s+(.+)$/i)))
      return "COVER — the range scan: every " + m[1] + " record where " + m[2];
    if ((m = s.match(/^INTEGRATE\s+(\w+)\s+OVER\s+(\w+)\s+MEASURE\s+(.+)$/i)))
      return "INTEGRATE — group " + m[1] + " by " + m[2] + ", compute " + m[3] + " per group";
    if ((m = s.match(/^INTEGRATE\s+(\w+)\s+MEASURE\s+(.+)$/i)))
      return "INTEGRATE — aggregate across all of " + m[1] + ": " + m[2];
    if ((m = s.match(/^CURVATURE\s+(\w+)$/i)))
      return "CURVATURE — the bundle-wide κ readout for " + m[1];
    if ((m = s.match(/^BUNDLE\s+(\w+)/i)))
      return "BUNDLE — the schema declaration for " + m[1] + " (base + fiber shape)";
    return "a query GIGI understands";
  };

  var LEGEND_HTML =
    'Each query below is a real GQL statement GIGI just answered. ' +
    '<b>SHOW</b> = list what\'s stored · ' +
    '<b>HEALTH</b> = vitals · ' +
    '<b>SECTION AT</b> = O(1) lookup · ' +
    '<b>COVER</b> = range scan · ' +
    '<b>INTEGRATE</b> = group + aggregate.';

  /* GQL drawer wiring — same behavior as the exercise-page consoles */
  WX.wireConsole = function (root) {
    root.querySelectorAll("details.gql").forEach(function (box) {
      var run = box.querySelector(".run"), ep = box.querySelector(".ep"),
          q = box.querySelector("textarea"), out = box.querySelector("pre");
      if (!run) return;
      var saved = localStorage.getItem("gb-endpoint");
      if (saved) ep.value = saved;
      // one-time chrome: dashed legend up top + italic gloss above the
      // response. Idempotent — safe on re-wire.
      if (!box.querySelector(".gq-legend")) {
        var legend = document.createElement("div");
        legend.className = "gq-legend";
        legend.innerHTML = LEGEND_HTML;
        var summary = box.querySelector("summary");
        (summary && summary.nextSibling)
          ? box.insertBefore(legend, summary.nextSibling)
          : box.appendChild(legend);
      }
      var gloss = box.querySelector(".gloss");
      if (!gloss) {
        gloss = document.createElement("div");
        gloss.className = "gloss";
        gloss.setAttribute("aria-live", "polite");
        out.parentNode.insertBefore(gloss, out);
      }
      function refreshGloss() { gloss.textContent = WX.gqlGloss(q.value); }
      refreshGloss();
      q.addEventListener("input", refreshGloss);
      box.querySelectorAll(".chip").forEach(function (ch) {
        ch.addEventListener("click", function () {
          q.value = ch.getAttribute("data-q");
          refreshGloss();
        });
      });
      function go() {
        var base = ep.value.replace(/\/+$/, "");
        localStorage.setItem("gb-endpoint", ep.value);
        out.textContent = "running…";
        fetch(base + "/v1/public/gql", {
          method: "POST", headers: { "Content-Type": "application/json" },
          body: JSON.stringify({ query: q.value })
        }).then(function (r) {
          return r.text().then(function (t) { return { ok: r.ok, status: r.status, t: t }; });
        }).then(function (res) {
          if (!res.ok) { out.textContent = "error: HTTP " + res.status + "\n\n" + res.t; return; }
          try { out.textContent = JSON.stringify(JSON.parse(res.t), null, 2); }
          catch (e) { out.textContent = res.t; }
        }).catch(function (e) {
          out.textContent = "error: " + e +
            "\n\nIf you are pointing at your own engine, start it with GIGI_CORS_ORIGIN=* (dev only).";
        });
      }
      run.addEventListener("click", go);
      q.addEventListener("keydown", function (e) {
        if (e.key === "Enter" && (e.metaKey || e.ctrlKey)) go();
      });
    });
  };
})();
