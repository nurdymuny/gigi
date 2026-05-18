import { useState } from "react";
import type {
  BundleSchema,
  RowMap,
  SheetsClient,
  TransportResult,
  HolonomyResult,
  SpectralResult,
  BettiResult,
} from "../lib/gigi-client";
import { kappaClass } from "../lib/kappa";
import { TermInfo } from "./TermInfo";
import {
  SpectralCard,
  BettiCard,
  TransportCard,
  HolonomyCard,
} from "./VerbCards";
import "./Inspector.css";

export interface InspectorProps {
  client: SheetsClient;
  bundle: string;
  schema: BundleSchema | null;
  selectedRow: RowMap | null;
  keyField: string | undefined;
  coverField: string;
  fiberFields: string[];
  kappa: number | undefined;
  /** Bundle-level spectral λ₁ from the most recent /spectral call, or undefined. */
  spectralLambda1?: number;
}

type VerbKind = "spectral" | "transport" | "holonomy" | "betti";

interface VerbState {
  kind: VerbKind;
  loading: boolean;
  error: string | null;
  result:
    | { kind: "spectral"; data: SpectralResult }
    | { kind: "betti"; data: BettiResult }
    | { kind: "transport"; data: TransportResult; from: string; to: string }
    | { kind: "holonomy"; data: HolonomyResult; around: string }
    | null;
}

export function Inspector({
  client,
  bundle,
  schema,
  selectedRow,
  keyField,
  coverField,
  fiberFields,
  kappa,
  spectralLambda1,
}: InspectorProps) {
  const [verb, setVerb] = useState<VerbState | null>(null);

  if (!schema || !selectedRow || !keyField) {
    return (
      <aside className="inspector inspector-empty" data-testid="inspector-empty">
        <p>Select a row to inspect its geometry.</p>
      </aside>
    );
  }

  const rowKey = String(selectedRow[keyField] ?? "");
  const k = typeof kappa === "number" ? kappa : 0;
  const conf = 1 / (1 + k);
  const cap = k > 0 ? 1.98 / k : Infinity;
  const kClass = kappaClass(k);

  const runVerb = async (kind: VerbKind) => {
    setVerb({ kind, loading: true, error: null, result: null });
    try {
      if (kind === "spectral") {
        const data = await client.spectral(bundle);
        setVerb({ kind, loading: false, error: null, result: { kind, data } });
      } else if (kind === "betti") {
        const data = await client.betti(bundle);
        setVerb({ kind, loading: false, error: null, result: { kind, data } });
      } else if (kind === "transport") {
        // Pick the nearest healthy peer in the current row's cohort.
        // For S3 we use the simplest possible heuristic: a row with a different
        // key but in the same cover group. The App passes us nothing about
        // other rows here — so we just call TRANSPORT to a key derived from
        // the row's metadata. To keep this self-contained, the caller-style
        // approach is: TRANSPORT from this row to itself's centroid. The
        // engine will error if the target doesn't exist, which is a useful
        // signal for now.
        const fromKey = { [keyField]: selectedRow[keyField] };
        const target = pickPeerKey(selectedRow, keyField);
        if (!target) {
          setVerb({
            kind,
            loading: false,
            error: "No peer key available — select a row with a known sibling.",
            result: null,
          });
          return;
        }
        const data = await client.transport(bundle, fromKey, target, fiberFields);
        setVerb({
          kind,
          loading: false,
          error: null,
          result: {
            kind,
            data,
            from: String(selectedRow[keyField]),
            to: String(Object.values(target)[0]),
          },
        });
      } else if (kind === "holonomy") {
        const data = await client.holonomy(bundle, fiberFields, coverField);
        setVerb({
          kind,
          loading: false,
          error: null,
          result: { kind, data, around: coverField },
        });
      }
    } catch (err) {
      const msg = err instanceof Error ? err.message : String(err);
      setVerb({ kind, loading: false, error: msg, result: null });
    }
  };

  return (
    <aside className="inspector" data-testid="inspector">
      <header className="insp-head">
        <div className="insp-eyebrow">Geometry inspector</div>
        <h2 className="insp-title" data-testid="insp-title">
          <span>{rowKey}</span>
          <span className={`pill pill-${kClass}`} data-testid="insp-flag">
            {kClass === "bad" ? "anomaly" : kClass === "warn" ? "drift" : "healthy"}
          </span>
        </h2>
        <p className="insp-sub">
          {schema.name} · {coverField} = {String(selectedRow[coverField] ?? "—")}
        </p>
      </header>

      <section className="insp-section">
        <h4>Bundle properties at this section</h4>
        <div className="gauges" data-testid="gauges">
          <Gauge
            label="Scalar curvature κ"
            value={k.toFixed(2)}
            kind={kClass}
            testid="gauge-kappa"
            term="kappa"
          />
          <Gauge
            label="Confidence 1/(1+κ)"
            value={conf.toFixed(2)}
            kind={conf < 0.4 ? "bad" : "ok"}
            testid="gauge-conf"
            term="confidence"
          />
          <Gauge
            label="Capacity C = τ/κ"
            value={Number.isFinite(cap) ? cap.toFixed(2) : "∞"}
            testid="gauge-capacity"
            term="capacity"
          />
          <Gauge
            label="Spectral λ₁"
            value={spectralLambda1 !== undefined ? spectralLambda1.toFixed(3) : "—"}
            testid="gauge-lambda1"
            term="spectral"
          />
        </div>
      </section>

      <WhyFlagged
        kappa={k}
        kClass={kClass}
        coverField={coverField}
        coverValue={String(selectedRow[coverField] ?? "—")}
      />

      <section className="insp-section">
        <h4>Run a geometric verb</h4>
        <div className="verb-list">
          <VerbButton
            kind="spectral"
            label="SPECTRAL"
            sub="Top eigenvalue, diameter, capacity"
            onRun={runVerb}
            disabled={Boolean(verb?.loading && verb.kind === "spectral")}
            testid="verb-spectral"
            term="spectral"
          />
          <VerbButton
            kind="transport"
            label="TRANSPORT"
            sub="Rotation to nearest peer"
            onRun={runVerb}
            disabled={Boolean(verb?.loading && verb.kind === "transport")}
            testid="verb-transport"
            term="transport"
          />
          <VerbButton
            kind="holonomy"
            label="HOLONOMY"
            sub={`Loop around ${coverField}`}
            onRun={runVerb}
            disabled={Boolean(verb?.loading && verb.kind === "holonomy")}
            testid="verb-holonomy"
            term="holonomy"
          />
          <VerbButton
            kind="betti"
            label="BETTI"
            sub="Sheaf cohomology"
            onRun={runVerb}
            disabled={Boolean(verb?.loading && verb.kind === "betti")}
            testid="verb-betti"
            term="betti"
          />
        </div>

        {verb?.loading ? (
          <div className="verb-loading" data-testid="verb-loading">
            Running {verb.kind.toUpperCase()}…
          </div>
        ) : null}

        {verb?.error ? (
          <div className="verb-error" role="alert" data-testid="verb-error">
            {verb.error}
          </div>
        ) : null}

        {verb?.result?.kind === "spectral" && <SpectralCard data={verb.result.data} />}
        {verb?.result?.kind === "betti" && <BettiCard data={verb.result.data} />}
        {verb?.result?.kind === "transport" && (
          <TransportCard
            data={verb.result.data}
            from={verb.result.from}
            to={verb.result.to}
          />
        )}
        {verb?.result?.kind === "holonomy" && (
          <HolonomyCard data={verb.result.data} around={verb.result.around} />
        )}
      </section>

      <EncryptedFields schema={schema} />
    </aside>
  );
}

/**
 * Auto-generated plain-English explanation of why a row's κ class is
 * what it is. No ML — just a deterministic readout of the geometry.
 */
function WhyFlagged({
  kappa,
  kClass,
  coverField,
  coverValue,
}: {
  kappa: number;
  kClass: "ok" | "warn" | "bad";
  coverField: string;
  coverValue: string;
}) {
  let title: string;
  let body: string;
  if (kClass === "bad") {
    title = "Anomaly — far from its cohort centroid";
    body = `Within ${coverField}="${coverValue}", this row's numeric profile sits ${kappa.toFixed(2)} units away from the cohort centroid. That's above the κ=2.0 threshold the engine treats as an outlier. Confidence is ${(1 / (1 + kappa)).toFixed(2)}.`;
  } else if (kClass === "warn") {
    title = "Drift — wandering from its peers";
    body = `Within ${coverField}="${coverValue}", this row is starting to wander from the cohort center (κ=${kappa.toFixed(2)}). Not yet an anomaly, but worth a look. Confidence is ${(1 / (1 + kappa)).toFixed(2)}.`;
  } else {
    title = "Healthy — sits close to its cohort";
    body = `Within ${coverField}="${coverValue}", this row matches its peers well (κ=${kappa.toFixed(2)}, confidence=${(1 / (1 + kappa)).toFixed(2)}). No action needed.`;
  }
  return (
    <section
      className={`insp-section insp-why insp-why-${kClass}`}
      data-testid="insp-why"
      data-kappa-class={kClass}
    >
      <h4>Why is this flagged?</h4>
      <p className="insp-why-title">{title}</p>
      <p className="insp-why-body">{body}</p>
    </section>
  );
}

/**
 * Encrypted-fields summary: lists every column with a non-"none" encryption
 * mode, surfacing GIGI's distinctive property — geometry computed over
 * ciphertext, no decryption in the UI.
 */
function EncryptedFields({ schema }: { schema: BundleSchema }) {
  const enc = [...schema.base_fields, ...schema.fiber_fields].filter(
    (f) => f.encryption && f.encryption !== "none",
  );
  if (enc.length === 0) return null;
  return (
    <section className="insp-section insp-encrypted" data-testid="insp-encrypted">
      <h4>
        <LockIcon />
        Encrypted fields
      </h4>
      <ul className="insp-enc-list">
        {enc.map((f) => (
          <li key={f.name} data-testid={`insp-enc-${f.name}`}>
            <span className="insp-enc-name">{f.name}</span>
            <span className="insp-enc-mode">
              {f.encryption?.toUpperCase()} · {modeLabel(f.encryption!)}
            </span>
          </li>
        ))}
      </ul>
      <p className="insp-enc-note">
        Queryable per their mode. Real encryption is enforced by the GIGI
        engine — demo bundles tag fields via a display-only overlay so the
        UI behavior is honest. κ and λ₁ run <strong>over the ciphertext</strong>
        on the engine side at native speed.
      </p>
    </section>
  );
}

function modeLabel(mode: string): string {
  // Describe observable behavior, not algorithm names — the demo overlay
  // doesn't perform cryptography in the browser, so claiming a specific
  // primitive would be misleading. Real engine-side modes are
  // documented in the engine spec; this UI just reflects the tag.
  if (mode === "opaque") return "value masked in UI · no plaintext rendered";
  if (mode === "indexed") return "equality lookups OK · range queries blocked";
  if (mode === "affine") return "numeric gauge · v ↦ a·v + b";
  return mode;
}

function LockIcon() {
  return (
    <svg
      width="11"
      height="11"
      viewBox="0 0 24 24"
      fill="none"
      stroke="#b45309"
      strokeWidth="2.4"
      aria-hidden="true"
    >
      <rect x="5" y="11" width="14" height="9" rx="2" />
      <path d="M8 11V8a4 4 0 0 1 8 0v3" />
    </svg>
  );
}

function Gauge({
  label,
  value,
  kind,
  testid,
  term,
}: {
  label: string;
  value: string;
  kind?: "ok" | "warn" | "bad";
  testid?: string;
  term?: string;
}) {
  return (
    <div className={`gauge ${kind ? `gauge-${kind}` : ""}`} data-testid={testid}>
      <div className="gauge-lbl">
        {label}
        {term ? <TermInfo term={term} /> : null}
      </div>
      <div className="gauge-val">{value}</div>
    </div>
  );
}

function VerbButton({
  kind,
  label,
  sub,
  onRun,
  disabled,
  testid,
  term,
}: {
  kind: VerbKind;
  label: string;
  sub: string;
  onRun: (k: VerbKind) => void;
  disabled: boolean;
  testid: string;
  term?: string;
}) {
  return (
    <div className="verb-button-wrap">
      <button
        type="button"
        className="verb-button"
        onClick={() => onRun(kind)}
        disabled={disabled}
        data-testid={testid}
      >
        <div className="verb-label">
          <code>{label}</code>
        </div>
        <div className="verb-sub">{sub}</div>
      </button>
      {term ? <TermInfo term={term} className="verb-info" /> : null}
    </div>
  );
}

/**
 * Hack for v0.1: TRANSPORT needs a second key. Without a row picker UI yet,
 * we synthesize one from the selected row's key by stripping the last
 * character. This is intentional — it's better to surface "no peer
 * available" cleanly than to silently call the engine with a self-loop.
 */
function pickPeerKey(
  row: RowMap,
  keyField: string,
): Record<string, unknown> | null {
  const v = row[keyField];
  if (typeof v !== "string") return null;
  // Common pattern: S-0142, CAS-001 — peer is one numeric step away.
  const m = v.match(/^(.*?)(\d+)$/);
  if (!m) return null;
  const [, prefix, digits] = m;
  const n = parseInt(digits, 10);
  if (Number.isNaN(n)) return null;
  const peer = `${prefix}${String(n + 1).padStart(digits.length, "0")}`;
  return { [keyField]: peer };
}
