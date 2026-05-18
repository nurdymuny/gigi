import { useEffect } from "react";
import gigiIconUrl from "../assets/gigi-icon.svg";
import { type SheetsClient } from "../lib/gigi-client";
import { DemoBundles } from "./DemoBundles";
import { WorkflowPicker } from "./WorkflowPicker";
import "./LandingPage.css";

export interface LandingPageProps {
  client: SheetsClient;
  /** When provided, demo tiles + CTA navigate in-place instead of reloading. */
  onPickBundle?: (name: string) => void;
  /** Opens the magic-link sign-in modal in the parent. */
  onSignInClick?: () => void;
  /**
   * Launches the guided tour (Project Tracker walkthrough). The parent
   * owns the tour state because the tour spotlights elements inside the
   * bundle app, not the landing page.
   */
  onStartTour?: () => void;
}

/**
 * Public marketing landing. Shown when an unauthenticated visitor hits
 * `/gigi/sheets/` (no bundle in the path). Signed-in users skip this
 * entirely and land on the dashboard.
 *
 * Every feature listed here is treated as live — we don't ship "coming
 * soon" labels. The launch gate is in `FEATURE_PARITY.md`.
 */
export function LandingPage({
  client,
  onPickBundle,
  onSignInClick,
  onStartTour,
}: LandingPageProps) {
  // The app's global CSS pins html/body/#root to 100% height with
  // overflow:hidden so internal grid views can manage their own scrolling.
  // The landing page is a tall scrollable document, so opt out for as long
  // as this component is mounted.
  useEffect(() => {
    document.body.classList.add("landing-mounted");
    return () => document.body.classList.remove("landing-mounted");
  }, []);

  return (
    <div className="landing" data-testid="landing-page">
      {/* ── nav ─────────────────────────────────────────────────────── */}
      <nav className="landing-nav">
        <div className="landing-brand">
          <img src={gigiIconUrl} alt="GIGI" className="landing-brand-icon" />
          <span className="landing-brand-name">GIGI Sheets</span>
        </div>
        <div className="landing-nav-actions">
          <a href="#compare" className="landing-nav-link">Compare</a>
          <a href="#features" className="landing-nav-link">Features</a>
          <a href="#math" className="landing-nav-link">The math</a>
          <a href="#workflows" className="landing-nav-link">Workflows</a>
          <a href="#demos" className="landing-nav-link">Demos</a>
          <button
            type="button"
            className="landing-nav-cta"
            onClick={onSignInClick}
            data-testid="landing-signin"
          >
            Sign in
          </button>
        </div>
      </nav>

      {/* ── hero ────────────────────────────────────────────────────── */}
      <section className="landing-hero">
        <div className="landing-hero-inner">
          <div className="landing-eyebrow">
            <span className="landing-eyebrow-dot" />
            gigi sheets &gt; excel · airtable · numbers
          </div>
          <h1 className="landing-h1">
            Spreadsheets made with{" "}
            <span className="accent-gold">real math.</span>
          </h1>
          <p className="landing-hero-tagline">
            They <span className="accent-blue">know</span> when their data is{" "}
            <span className="accent-gold">lying</span> to you.
          </p>
          <p className="landing-sub">
            Excel is a grid. Airtable is a grid with API hooks. GIGI Sheets is a
            geometric data engine — every cell sits in a Davis bundle, every
            row has a curvature, anomalies surface on contact. Field-level
            encryption, real-time streams, and built-in ML come standard.
          </p>
          <div className="landing-hero-cta">
            {onStartTour ? (
              <button
                type="button"
                className="landing-btn landing-btn-primary"
                onClick={onStartTour}
                data-testid="landing-start-tour"
              >
                Take the 60-second tour
              </button>
            ) : (
              <a href="#demos" className="landing-btn landing-btn-primary">
                Try a live demo — no sign-up
              </a>
            )}
            <button
              type="button"
              className="landing-btn landing-btn-ghost"
              onClick={onSignInClick}
            >
              Sign in to your bundles
            </button>
          </div>
          <div className="landing-hero-meta">
            <span>
              <strong className="accent-gold">S = (1 + cosθ)/2</strong> · the
              Davis identity
            </span>
            <span className="landing-meta-sep">·</span>
            <span>sub-millisecond on millions of rows</span>
            <span className="landing-meta-sep">·</span>
            <span>fiber bundles for humans</span>
          </div>
        </div>
      </section>

      {/* ── value props ─────────────────────────────────────────────── */}
      <section className="landing-section landing-pillars">
        <h2 className="landing-h2">Four things no other spreadsheet does.</h2>
        <div className="landing-pillar-grid">
          <Pillar
            n="01"
            title="Davis math, native"
            body="Every cell carries a 448-dim embedding. Every row has a κ-curvature. Sort, filter, search, and join by sameness — not by string match."
            tag="S = (1 + cosθ)/2"
          />
          <Pillar
            n="02"
            title="Field-level encryption (engine-side)"
            body="Per-column encryption modes (indexed, affine, opaque) shipped by the GIGI engine. Search and aggregate over the ciphertext without decrypting it. This sheets client is a viewer — encryption is enforced server-side. Demo bundles use a display-only overlay to preview the UX."
            tag="indexed · affine · opaque"
          />
          <Pillar
            n="03"
            title="Real-time, not polled"
            body="Subscribe to a bundle and watch rows stream in. Airtable polls every 15 seconds. GIGI streams in microseconds."
            tag="GQL subscriptions"
          />
          <Pillar
            n="04"
            title="Prism built in"
            body="Dedup, Forecast, Monitor, Books — production-grade reconcile workflows run against any bundle in one click."
            tag="four workflows"
          />
        </div>
      </section>

      {/* ── comparison ──────────────────────────────────────────────── */}
      <section className="landing-section" id="compare">
        <h2 className="landing-h2">vs. the rest of the spreadsheet world.</h2>
        <p className="landing-sub-small">
          Every row is a feature GIGI ships today. No "soon" badges, no
          futures — this is live.
        </p>
        <div className="landing-table-wrap">
          <table className="landing-table" data-testid="landing-compare">
            <thead>
              <tr>
                <th>Feature</th>
                <th>Excel</th>
                <th>Airtable</th>
                <th className="landing-th-gigi">GIGI Sheets</th>
              </tr>
            </thead>
            <tbody>
              <CompareRow
                feature="Sort by column"
                excel="A→Z lexicographic"
                airtable="A→Z lexicographic"
                gigi="A→Z · κ-rank · sameness-to-pivot"
                win
              />
              <CompareRow
                feature="Column filter"
                excel="value / range"
                airtable="value / range"
                gigi="value · range · sameness ≥ τ · κ-class"
                win
              />
              <CompareRow
                feature="Find"
                excel="exact / regex"
                airtable="exact"
                gigi="exact · canonical · sameness"
                win
              />
              <CompareRow
                feature="Drag-fill"
                excel="last-two linear"
                airtable="basic"
                gigi="OLS over full selection + √step band"
                win
              />
              <CompareRow
                feature="Linked records"
                excel="VLOOKUP (exact only)"
                airtable="exact FK match"
                gigi="sameness-join — survives typos & drift"
                win
              />
              <CompareRow
                feature="Conditional format"
                excel="user rules"
                airtable="user rules"
                gigi="κ-overlay default + user rules + Davis-violation"
                win
              />
              <CompareRow
                feature="Formula primitives"
                excel="500+ funcs"
                airtable="full engine"
                gigi="80/20 set + =SAME, =K, =DIST, =COHORT"
                win
              />
              <CompareRow
                feature="Field-level encryption"
                excel="—"
                airtable="—"
                gigi="det · ored · opaque (search without decrypt)"
                win
              />
              <CompareRow
                feature="Real-time updates"
                excel="manual refresh"
                airtable="poll ~15s"
                gigi="GQL subscription · microsecond"
                win
              />
              <CompareRow
                feature="Anomaly detection"
                excel="—"
                airtable="—"
                gigi="κ-curvature on every row, always on"
                win
              />
              <CompareRow
                feature="Form view"
                excel="—"
                airtable="yes"
                gigi="yes + pre-insert κ check"
                win
              />
              <CompareRow
                feature="Calendar view"
                excel="—"
                airtable="yes"
                gigi="yes + κ-tint per day"
                win
              />
              <CompareRow
                feature="Gallery view"
                excel="—"
                airtable="yes"
                gigi="yes + sameness clusters"
                win
              />
              <CompareRow
                feature="Built-in analytics"
                excel="pivot table"
                airtable="bolt-on apps"
                gigi="Dedup · Forecast · Monitor · Books"
                win
              />
              <CompareRow
                feature="Workflow templates"
                excel="—"
                airtable="100s, mostly cosmetic"
                gigi="6 starters with κ overlay + Prism wired in"
                win
              />
              <CompareRow
                feature="Verifiable audit trail"
                excel="—"
                airtable="audit log (Enterprise)"
                gigi="signed bundle, every row, every plan"
                win
              />
            </tbody>
          </table>
        </div>
      </section>

      {/* ── the math ────────────────────────────────────────────────── */}
      <section className="landing-section landing-math" id="math">
        <div className="landing-math-inner">
          <div className="landing-eyebrow landing-eyebrow-center">
            <span className="landing-eyebrow-dot" />
            the math, in plain english
          </div>
          <h2 className="landing-h2">
            One equation does the work of an ML pipeline.
          </h2>
          <p className="landing-sub-small">
            Every row in your spreadsheet is turned into a list of numbers — a
            point in space. Once you have points, you can ask geometric
            questions: <em>how close are these two?</em>{" "}
            <em>which one is the odd one out?</em>{" "}
            <em>what's the center of this group?</em> All of GIGI's
            "smart" features — sort by similarity, filter by anomaly,
            sameness-join, drag-fill — are these geometric questions in
            disguise.
          </p>

          {/* ── The identity, hero treatment ────────────────────────── */}
          <div className="landing-equation">
            <code>S + d² = 1</code>
            <span className="landing-equation-label">
              the Davis double-cover identity
            </span>
          </div>

          {/* ── Plain-English breakdown ─────────────────────────────── */}
          <div className="landing-math-explain">
            <p className="landing-math-explain-p">
              Two rows have a <strong>sameness</strong>{" "}
              <code>S</code> between 0 and 1, and a <strong>distance</strong>{" "}
              <code>d</code> between 0 and 1. They always satisfy{" "}
              <code>S + d² = 1</code> — sameness and distance are two sides
              of the same coin. With <code>S = cos²(θ/2)</code> and{" "}
              <code>d = sin(θ/2)</code>, the identity is the half-angle
              Pythagorean rule.
            </p>
            <ul className="landing-math-cases">
              <li>
                <strong>S = 1, d = 0</strong> — identical rows. The dedup
                threshold lives just below this.
              </li>
              <li>
                <strong>S ≈ 0.85, d ≈ 0.39</strong> — clearly related; the
                cross-rail match threshold Prism uses for "same payment, two
                rails."
              </li>
              <li>
                <strong>S = 0.5, d ≈ 0.71</strong> — orthogonal; the rows
                share roughly nothing.
              </li>
              <li>
                <strong>S = 0, d = 1</strong> — perfectly opposite. Rare in
                real data; means the two rows disagree on every encoded axis.
              </li>
            </ul>
          </div>

          {/* ── Worked example ──────────────────────────────────────── */}
          <div className="landing-math-example">
            <div className="landing-math-example-head">
              <span className="landing-math-example-label">worked example</span>
              <h3 className="landing-math-example-h">
                Two payment rows from the demo bundle
              </h3>
            </div>
            <div className="landing-math-example-grid">
              <div className="landing-math-example-row">
                <span className="landing-math-example-key">Row A</span>
                <code className="landing-math-example-val">
                  P-100001 · CHAS→DBSS · $250,000 · USD · SWIFT · 2026-04-12 ·
                  ref "INV-2026-04823"
                </code>
              </div>
              <div className="landing-math-example-row">
                <span className="landing-math-example-key">Row B</span>
                <code className="landing-math-example-val">
                  P-100002 · CHAS→DBSS · $250,000 · USD · SWIFT · 2026-04-12 ·
                  ref "INV 2026 04823"
                </code>
              </div>
              <div className="landing-math-example-arrow">↓ embed both rows into 448-dim space</div>
              <div className="landing-math-example-row landing-math-example-out">
                <span className="landing-math-example-key">Result</span>
                <code className="landing-math-example-val">
                  S = 1.0000 · d = 0.0000 · κ = 0.0001 → <strong>duplicate</strong>
                </code>
              </div>
            </div>
            <p className="landing-math-example-note">
              Excel would treat these as different rows because the reference
              strings aren't byte-equal. GIGI canonicalizes both into{" "}
              <code>INV202604823</code>, the rest of the row is identical, so
              the embeddings line up and sameness lands at 1.0. The Davis
              identity guarantees distance is exactly 0 — no calibration knob
              to tune.
            </p>
          </div>

          {/* ── The four primitives ─────────────────────────────────── */}
          <h3 className="landing-math-h3">The four operators that do all the work</h3>
          <p className="landing-sub-small">
            Every parity feature on this page reduces to a combination of
            these four. They're small enough to fit on a chip; deep enough
            that nobody else's spreadsheet has them.
          </p>
          <div className="landing-math-strip">
            <MathChip
              label="sameness"
              expr="S(a,b) = (1 + cosθ)/2"
              plain="How alike are two rows? Cosine of the angle between their embedding vectors, rescaled to [0,1]. 1 is identical; 0.5 is orthogonal; 0 is opposite."
            />
            <MathChip
              label="distance"
              expr="d(a,b) = √(1 − S) = sin(θ/2)"
              plain="The Davis distance — half-angle sine, the natural geometric pair to sameness. Derived from S via the double-cover identity, so they can't drift apart."
            />
            <MathChip
              label="curvature"
              expr="κ(r) = 1 − S(r, μ)"
              plain="How far a row sits from its cohort's center. 0 = textbook example of its group; >0.3 = the row that doesn't belong."
            />
            <MathChip
              label="cohort centroid"
              expr="μ = normalize(Σ embed(rᵢ))"
              plain="The geometric center of a group of rows. Take the mean of their unit embedding vectors, then re-normalize. κ is measured against this point."
            />
          </div>

          {/* ── Why this matters ──────────────────────────────────── */}
          <h3 className="landing-math-h3">Why one equation beats a pipeline</h3>
          <div className="landing-math-why">
            <div className="landing-math-why-row">
              <span className="landing-math-why-task">
                "Find rows like this one"
              </span>
              <span className="landing-math-why-other">
                ML: train a similarity model, label data, deploy.
              </span>
              <span className="landing-math-why-gigi">
                GIGI: <code>S(row, pivot) ≥ τ</code>.
              </span>
            </div>
            <div className="landing-math-why-row">
              <span className="landing-math-why-task">
                "Flag anomalies"
              </span>
              <span className="landing-math-why-other">
                ML: isolation forest, tune contamination param.
              </span>
              <span className="landing-math-why-gigi">
                GIGI: <code>κ(row) &gt; 0.3</code>.
              </span>
            </div>
            <div className="landing-math-why-row">
              <span className="landing-math-why-task">
                "Match across two tables"
              </span>
              <span className="landing-math-why-other">
                SQL: JOIN ON exact key. Typos break it.
              </span>
              <span className="landing-math-why-gigi">
                GIGI: <code>S(a.key, b.key) ≥ 0.85</code>.
              </span>
            </div>
            <div className="landing-math-why-row">
              <span className="landing-math-why-task">
                "Project a trend"
              </span>
              <span className="landing-math-why-other">
                Stats: pick a model, fit, validate.
              </span>
              <span className="landing-math-why-gigi">
                GIGI: OLS over selection + √step band.
              </span>
            </div>
            <div className="landing-math-why-row">
              <span className="landing-math-why-task">
                "Audit who changed what"
              </span>
              <span className="landing-math-why-other">
                Custom logging + change-data-capture.
              </span>
              <span className="landing-math-why-gigi">
                GIGI: every row already signed. Done.
              </span>
            </div>
          </div>

          {/* ── Reading-list footer ─────────────────────────────────── */}
          <p className="landing-math-footer">
            For the full derivation — including why <code>cosθ</code>{" "}
            divided by 2 is the only metric that satisfies the double-cover
            identity, and how it generalizes to fiber bundles — see the{" "}
            <a
              href="https://davisgeometric.com/papers"
              className="landing-math-link"
            >
              Davis Geometric papers
            </a>
            .
          </p>
        </div>
      </section>

      {/* ── feature grid ────────────────────────────────────────────── */}
      <section className="landing-section" id="features">
        <h2 className="landing-h2">Sixteen features, every one geometric.</h2>
        <p className="landing-sub-small">
          Each feature does what its Excel counterpart does — then layers Davis
          math on top.
        </p>
        <div className="landing-feature-grid">
          <Feature name="Sort" pitch="Click a header. Or pick a row and sort by sameness to it." />
          <Feature name="Filter" pitch="Text, range, sameness ≥ τ, or κ-class. Stack them." />
          <Feature name="Find & replace" pitch="Exact, regex, canonical (the Dedup trick), or sameness." />
          <Feature name="Range select" pitch="Drag a rect. Shift-G extends by κ-neighborhood." />
          <Feature name="Copy / paste" pitch="TSV out, bundle-JSON in. Outliers get flagged before commit." />
          <Feature name="Drag-fill" pitch="OLS trend over the selection, not a naive two-point line." />
          <Feature name="Freeze cols" pitch="Pin N columns. Curvature column auto-pins with overlay." />
          <Feature name="Conditional fmt" pitch="κ-overlay default. Add rules referencing S(row, pivot)." />
          <Feature name="Number / date fmt" pitch="Schema-driven defaults. Per-column override strings." />
          <Feature name="Multi-select" pitch="Chips are points in φ_ent space. Cluster by tag, not list." />
          <Feature name="Linked records" pitch="Sameness-join across bundles. No more typo-breaks-FK." />
          <Feature name="Per-view state" pitch="Filter + sort + κ-bracket save with the view." />
          <Feature name="Calendar" pitch="Month grid tinted by mean κ per day." />
          <Feature name="Gallery" pitch="Cards laid out by PCA on the embedding matrix." />
          <Feature name="Form view" pitch="Submit fires a pre-insert κ check. Outliers ask first." />
          <Feature name="Formula bar" pitch="80/20 set plus =SAME(A1,B1), =K(A1), =DIST(A1,B1)." />
        </div>
      </section>

      {/* ── workflows ────────────────────────────────────────────────── */}
      <section
        className="landing-section landing-workflows-section"
        id="workflows"
      >
        <h2 className="landing-h2">Or start with a workflow.</h2>
        <p className="landing-sub-small">
          Six pre-baked starters — project tracker, content calendar, CRM,
          event planning, inventory, recruiting. Same shapes you know from
          Airtable, with κ-overlay, field encryption, and Prism wired in on
          day one.
        </p>
        <div className="landing-workflows-wrap">
          <WorkflowPicker
            client={client}
            onApplied={(bundleName) => onPickBundle?.(bundleName)}
          />
        </div>
      </section>

      {/* ── demos ───────────────────────────────────────────────────── */}
      <section className="landing-section landing-demos-section" id="demos">
        <h2 className="landing-h2">Or try a demo dataset.</h2>
        <p className="landing-sub-small">
          Real datasets, one-click load. Run Prism Dedup against the payments
          one and watch reference-drift duplicates surface in milliseconds.
        </p>
        <div className="landing-demos-wrap">
          <DemoBundles
            client={client}
            existing={new Set()}
            onPickBundle={onPickBundle}
          />
        </div>
      </section>

      {/* ── final CTA ───────────────────────────────────────────────── */}
      <section className="landing-section landing-final-cta">
        <div className="landing-cta-card">
          <h2 className="landing-cta-h">Bring your own data.</h2>
          <p className="landing-cta-sub">
            Sign in with email, upload a CSV, and your spreadsheet ships with
            geometry, encryption, and ML — out of the box.
          </p>
          <button
            type="button"
            className="landing-btn landing-btn-primary landing-btn-lg"
            onClick={onSignInClick}
            data-testid="landing-signin-final"
          >
            Get started — free
          </button>
          <p className="landing-cta-fine">
            Magic-link sign-in. No credit card. Bundles up to 100k rows on the
            free tier.
          </p>
        </div>
      </section>

      {/* ── footer ──────────────────────────────────────────────────── */}
      <footer className="landing-footer">
        <div className="landing-footer-inner">
          <span>
            GIGI Sheets · part of{" "}
            <a href="https://davisgeometric.com" className="landing-footer-link">
              Davis Geometric
            </a>
          </span>
          <span className="landing-footer-sep">·</span>
          <span>Built on the Davis identity</span>
        </div>
      </footer>
    </div>
  );
}

function Pillar({
  n,
  title,
  body,
  tag,
}: {
  n: string;
  title: string;
  body: string;
  tag: string;
}) {
  return (
    <div className="landing-pillar">
      <span className="landing-pillar-n">{n}</span>
      <h3 className="landing-pillar-title">{title}</h3>
      <p className="landing-pillar-body">{body}</p>
      <span className="landing-pillar-tag">{tag}</span>
    </div>
  );
}

function CompareRow({
  feature,
  excel,
  airtable,
  gigi,
  win,
}: {
  feature: string;
  excel: string;
  airtable: string;
  gigi: string;
  win?: boolean;
}) {
  return (
    <tr>
      <th scope="row">{feature}</th>
      <td className="landing-td-other">{excel}</td>
      <td className="landing-td-other">{airtable}</td>
      <td className={`landing-td-gigi ${win ? "landing-td-win" : ""}`}>
        {gigi}
      </td>
    </tr>
  );
}

function MathChip({
  label,
  expr,
  plain,
}: {
  label: string;
  expr: string;
  plain?: string;
}) {
  return (
    <div className="landing-math-chip">
      <span className="landing-math-chip-label">{label}</span>
      <code className="landing-math-chip-expr">{expr}</code>
      {plain ? (
        <span className="landing-math-chip-plain">{plain}</span>
      ) : null}
    </div>
  );
}

function Feature({ name, pitch }: { name: string; pitch: string }) {
  return (
    <div className="landing-feature">
      <h4 className="landing-feature-name">{name}</h4>
      <p className="landing-feature-pitch">{pitch}</p>
    </div>
  );
}
