import type {
  SpectralResult,
  BettiResult,
  TransportResult,
  HolonomyResult,
} from "../lib/gigi-client";

/**
 * Verb result cards — one per geometric verb. Each is a pure component
 * driven by the typed response from gigi-client. The visual contract
 * lives in the mockup; these are the operationalized version.
 */

export function SpectralCard({ data }: { data: SpectralResult }) {
  const interp =
    data.lambda1 > 0.3
      ? "Well-connected cluster."
      : data.lambda1 > 0.1
      ? "Moderate connectivity."
      : "Loose / disconnected cover.";
  return (
    <div className="verb-result" data-testid="result-spectral">
      <h5>
        <code>SPECTRAL</code>
        <span className="verb-result-q">/v1/bundles/{"{name}"}/spectral</span>
      </h5>
      <Bars
        items={[
          { label: "λ₁", value: data.lambda1, scale: 120 },
          { label: "diam", value: data.diameter, scale: 4 },
          { label: "C", value: data.spectral_capacity, scale: 60 },
        ]}
      />
      <p className="verb-interp">{interp}</p>
    </div>
  );
}

export function BettiCard({ data }: { data: BettiResult }) {
  const chi = data.beta_0 - data.beta_1;
  return (
    <div className="verb-result" data-testid="result-betti">
      <h5>
        <code>BETTI</code>
        <span className="verb-result-q">/v1/bundles/{"{name}"}/betti</span>
      </h5>
      <div className="betti-row">
        <BettiNum value={data.beta_0} label="b₀" sub="components" />
        <BettiNum value={data.beta_1} label="b₁" sub="loops" />
        <BettiNum value={chi} label="χ" sub="Euler char" muted />
      </div>
      <p className="verb-interp">
        Euler characteristic χ = b₀ − b₁ ={" "}
        <span data-testid="betti-chi">{chi}</span>.
      </p>
    </div>
  );
}

export function TransportCard({
  data,
  from,
  to,
}: {
  data: TransportResult;
  from: string;
  to: string;
}) {
  const deg = (data.angle * 180) / Math.PI;
  return (
    <div className="verb-result" data-testid="result-transport">
      <h5>
        <code>TRANSPORT</code>
        <span className="verb-result-q">
          {from} → {to}
        </span>
      </h5>
      <Matrix matrix={data.matrix} dim={data.dim} />
      <div className="transport-angles">
        <span className="dial-num">
          {data.angle.toFixed(3)} <small>rad</small>
        </span>
        <span className="dial-num dial-num-secondary">
          {deg.toFixed(1)} <small>°</small>
        </span>
      </div>
      <p className="verb-interp">
        {Math.abs(data.angle) > 1.5
          ? "Large rotation — sections live on very different parts of the fiber."
          : "Small rotation — peers are geometrically close."}
      </p>
    </div>
  );
}

export function HolonomyCard({
  data,
  around,
}: {
  data: HolonomyResult;
  around: string;
}) {
  const deg = (data.angle * 180) / Math.PI;
  return (
    <div className="verb-result" data-testid="result-holonomy">
      <h5>
        <code>HOLONOMY</code>
        <span className="verb-result-q">around {around}</span>
      </h5>
      <div className="transport-angles">
        <span className="dial-num">
          δφ = {data.angle.toFixed(3)} <small>rad</small>
        </span>
        <span className="dial-num dial-num-secondary">
          {deg.toFixed(1)} <small>°</small>
        </span>
      </div>
      <p className="verb-interp">
        {data.trivial
          ? "Near-zero holonomy — flat connection. The fiber returns to itself around this loop."
          : `Non-trivial holonomy across ${data.centroids.length} cohorts. The bundle has measurable curvature on this loop.`}
      </p>
    </div>
  );
}

/* ── helpers ─────────────────────────────────────────────────────────── */

function Bars({
  items,
}: {
  items: Array<{ label: string; value: number; scale: number }>;
}) {
  return (
    <div className="bars">
      {items.map((it) => (
        <div className="bar-row" key={it.label}>
          <span className="bar-label">{it.label}</span>
          <span
            className="bar"
            style={{ width: `${Math.min(100, Math.max(2, it.value * it.scale))}%` }}
          />
          <span className="bar-val" data-testid={`bar-${it.label}`}>
            {Number.isInteger(it.value) ? it.value : it.value.toFixed(3)}
          </span>
        </div>
      ))}
    </div>
  );
}

function Matrix({ matrix, dim }: { matrix: number[]; dim: number }) {
  const cells: number[][] = [];
  for (let r = 0; r < dim; r++) {
    const row: number[] = [];
    for (let c = 0; c < dim; c++) {
      row.push(matrix[r * dim + c] ?? 0);
    }
    cells.push(row);
  }
  return (
    <div
      className="matrix"
      style={{ gridTemplateColumns: `repeat(${dim}, auto)` }}
      data-testid="matrix"
    >
      {cells.flatMap((row, r) =>
        row.map((v, c) => (
          <span className="mv" key={`${r}-${c}`}>
            {v.toFixed(3)}
          </span>
        )),
      )}
    </div>
  );
}

function BettiNum({
  value,
  label,
  sub,
  muted,
}: {
  value: number;
  label: string;
  sub: string;
  muted?: boolean;
}) {
  return (
    <div className={`betti-num ${muted ? "betti-num-muted" : ""}`}>
      <span className="betti-num-val">
        {value}
        <small>{label}</small>
      </span>
      <span className="betti-num-sub">{sub}</span>
    </div>
  );
}
