import { useEffect, useState } from "react";
import gigiLogoUrl from "../assets/gigi-logo.svg";
import "./AboutModal.css";

export interface AboutModalProps {
  open: boolean;
  onClose: () => void;
}

type Tab = "engine" | "person";

export function AboutModal({ open, onClose }: AboutModalProps) {
  const [tab, setTab] = useState<Tab>("engine");

  useEffect(() => {
    if (!open) return;
    function onKey(e: KeyboardEvent) {
      if (e.key === "Escape") onClose();
    }
    document.addEventListener("keydown", onKey);
    return () => document.removeEventListener("keydown", onKey);
  }, [open, onClose]);

  if (!open) return null;

  return (
    <div
      className="about-bg"
      data-testid="about-bg"
      onClick={(e) => {
        if (e.target === e.currentTarget) onClose();
      }}
    >
      <div className="about-modal" data-testid="about-modal" role="dialog">
        <button
          type="button"
          className="about-close"
          onClick={onClose}
          aria-label="Close"
          data-testid="about-close"
        >
          ✕
        </button>

        <div className="about-hero">
          <img
            src={gigiLogoUrl}
            className="about-mark"
            alt="GIGI"
            data-testid="about-mark"
            draggable={false}
          />
          <div className="about-titles">
            <h1>GIGI</h1>
            <p className="about-tagline">
              Geometric Intrinsic Global Index —{" "}
              <span className="about-tagline-soft">a fiber-bundle database engine</span>
            </p>
            <p className="about-byline">
              Davis Geometric · 2026 · by Bee Rosa Davis
            </p>
          </div>
        </div>

        <div className="about-tabs" role="tablist">
          <button
            type="button"
            role="tab"
            aria-selected={tab === "engine"}
            className={`about-tab ${tab === "engine" ? "about-tab-active" : ""}`}
            onClick={() => setTab("engine")}
            data-testid="about-tab-engine"
          >
            The engine
          </button>
          <button
            type="button"
            role="tab"
            aria-selected={tab === "person"}
            className={`about-tab ${tab === "person" ? "about-tab-active" : ""}`}
            onClick={() => setTab("person")}
            data-testid="about-tab-person"
          >
            Gigi <span className="about-tab-emoji">✨</span>
          </button>
        </div>

        <div className="about-body">
          {tab === "engine" ? <EngineTab /> : <PersonTab />}
        </div>

        <footer className="about-foot">
          <span className="about-foot-sub">
            MIT-licensed. Mathematical content covered by provisional patents.
          </span>
          <button type="button" className="about-btn" onClick={onClose}>
            Close
          </button>
        </footer>
      </div>
    </div>
  );
}

function EngineTab() {
  return (
    <div className="about-tab-engine-body" data-testid="about-engine">
      <blockquote className="about-quote">
        Records are sections of a fiber bundle. Keys live on the base space; values
        live on the fiber. Curvature, spectral connectivity, holonomy, and confidence
        are properties of the bundle — they update incrementally with every
        insert and ride along on every query response.
      </blockquote>

      <section className="about-section">
        <h3>What makes it different</h3>
        <div className="about-grid">
          <div className="about-card">
            <h4>O(1) point queries</h4>
            <p>
              Composite-key hash <code>G: K₁ × … × Kₘ → ℤ₂⁶⁴</code> — empirically
              validated by <code>o1_proof</code> bench.
            </p>
          </div>
          <div className="about-card">
            <h4>Geometry, not analytics</h4>
            <p>
              Scalar curvature κ, spectral gap λ₁, and Betti numbers are
              recomputed incrementally per write — no separate pipeline.
            </p>
          </div>
          <div className="about-card">
            <h4>Gauge encryption</h4>
            <p>
              The structure group of the fiber bundle is the cipher.
              <strong> Encryption preserves κ, λ₁, holonomy, anomaly scores</strong>
              {" "}at native speed (no homomorphic slowdown).
            </p>
          </div>
          <div className="about-card">
            <h4>DHOOM wire</h4>
            <p>
              Geometric event protocol — every mutation broadcasts with κ /
              confidence over WebSocket. The grid reflects writes from other
              clients without polling.
            </p>
          </div>
        </div>
      </section>

      <section className="about-section">
        <h3>Geometric verbs</h3>
        <div className="about-verbs">
          {[
            { v: "SECTION", d: "point query" },
            { v: "INTEGRATE", d: "aggregate over a cover" },
            { v: "CURVATURE", d: "scalar κ over the bundle" },
            { v: "SPECTRAL", d: "Laplacian eigenvalues" },
            { v: "HOLONOMY", d: "loop integral over a fiber" },
            { v: "TRANSPORT", d: "parallel transport between sections" },
            { v: "BETTI", d: "sheaf cohomology" },
            { v: "GEODESIC", d: "shortest path in fiber space" },
          ].map((v) => (
            <div className="about-verb" key={v.v}>
              <code>{v.v}</code>
              <span>{v.d}</span>
            </div>
          ))}
        </div>
      </section>

      <section className="about-section">
        <h3>What runs on it</h3>
        <ul className="about-products">
          <li>
            <strong>KRAKEN</strong> — sensor fusion: DAS, sonar, SAT, SIGINT
            bundles plus operator-judgment audit, all on GIGI.
          </li>
          <li>
            <strong>Marcella</strong> — fiber-geometric reads of language
            corpora. <code>HOLONOMY</code> / <code>TRANSPORT</code> /{" "}
            <code>SPECTRAL ON FIBER</code> over tense circles.
          </li>
          <li>
            <strong>ICARUS</strong> — sprint deliverables across <code>Transport</code>,{" "}
            <code>Holonomy</code>, <code>GaugeTest</code>, <code>SpectralFiber</code>,{" "}
            <code>Divergence</code> verbs.
          </li>
          <li>
            <strong>GIGI Sheets</strong> — this app. Spreadsheet UI for fiber
            bundles, for humans.
          </li>
        </ul>
      </section>
    </div>
  );
}

function PersonTab() {
  return (
    <div className="about-tab-person-body" data-testid="about-person">
      <section className="about-section about-section-hero">
        <div className="about-avatar" aria-hidden="true">
          BD
        </div>
        <div>
          <h2>Bee Rosa Davis</h2>
          <p className="about-role">
            Founder · Davis Geometric · she ships fiber bundles for a living.
          </p>
        </div>
      </section>

      <section className="about-section">
        <p className="about-prose">
          Bee — <em>Gigi</em> to her friends — built a database engine that
          sees the world as fiber bundles, and the product line that runs on
          it. KRAKEN watches sensors. Marcella reads language. ICARUS ships
          sprints. Just-Gigi gives creators their stack. GIGI is the substrate
          underneath them all.
        </p>
        <p className="about-prose">
          The work is half mathematics and half engineering: every claim in the
          repo's spec docs maps to a passing test in <code>cargo test</code>,
          and every passing test maps to a real geometric property — curvature,
          connectivity, holonomy — that ships in production with no separate
          analytics pipeline.
        </p>
        <p className="about-prose about-prose-soft">
          The fact that the system is called GIGI and the founder is also Gigi
          is a happy accident she enjoys.
        </p>
      </section>

      <section className="about-section">
        <h3>Things she made</h3>
        <ul className="about-products">
          <li>
            <strong>GIGI</strong> — the fiber-bundle database engine. Single
            Rust crate, 700+ tests, ships gauge encryption out of the box.
          </li>
          <li>
            <strong>KRAKEN</strong> — sensor-fusion platform on GIGI. Multi-modal:
            DAS, sonar, SAT, SIGINT, plus audit log of operator judgments.
          </li>
          <li>
            <strong>Marcella</strong> — fiber-geometric language model. Tense,
            morphology, and discourse as bundle structure.
          </li>
          <li>
            <strong>ICARUS</strong> — sprint orchestration with TDD baked in.
          </li>
          <li>
            <strong>Just-Gigi</strong> — the creator stack. Bundles + sheets +
            commerce, for the people who actually ship.
          </li>
        </ul>
      </section>

      <section className="about-section">
        <p className="about-quote-small">
          "Geometry is not a plugin."
          <span className="about-quote-attr">— GIGI README, line 1.</span>
        </p>
      </section>
    </div>
  );
}
