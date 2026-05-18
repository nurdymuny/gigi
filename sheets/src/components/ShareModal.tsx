import { useEffect, useMemo, useState } from "react";
import { buildAiPrompt, buildBundleSummary } from "../lib/bundle-summary";
import type { BundleSchema, RowMap } from "../lib/gigi-client";
import { encodeView, type ViewSpec } from "../lib/view";
import "./ShareModal.css";

export interface ShareModalProps {
  open: boolean;
  bundle: string;
  /** The current working-state snapshot. We slice it per the user's choices. */
  currentSpec: ViewSpec;
  /** Schema + rows + κ for building the AI-readable summary. */
  schema?: BundleSchema | null;
  rows?: RowMap[];
  kappaMap?: Map<string, number>;
  hiddenFields?: Set<string>;
  onClose: () => void;
  /** Export hooks: same handlers used by the File › Export menu. */
  onExportCsv: () => void;
  onExportJson: () => void;
  onExportGql: () => void;
}

type ShareKind = "link" | "export" | "ai";

interface IncludeFlags {
  coverField: boolean;
  overlayOn: boolean;
  activeView: boolean;
  gqlQuery: boolean;
}

const DEFAULT_INCLUDES: IncludeFlags = {
  coverField: true,
  overlayOn: true,
  activeView: true,
  gqlQuery: false,
};

export function ShareModal({
  open,
  bundle,
  currentSpec,
  schema,
  rows,
  kappaMap,
  hiddenFields,
  onClose,
  onExportCsv,
  onExportJson,
  onExportGql,
}: ShareModalProps) {
  const [kind, setKind] = useState<ShareKind>("link");
  const [copied, setCopied] = useState<
    "url" | "embed" | "summary" | "prompt" | null
  >(null);
  const [includes, setIncludes] = useState<IncludeFlags>(DEFAULT_INCLUDES);

  // Reset transient state every time the modal opens.
  useEffect(() => {
    if (open) {
      setKind("link");
      setCopied(null);
      setIncludes(DEFAULT_INCLUDES);
    }
  }, [open]);

  useEffect(() => {
    if (!open) return;
    function onKey(e: KeyboardEvent) {
      if (e.key === "Escape") onClose();
    }
    document.addEventListener("keydown", onKey);
    return () => document.removeEventListener("keydown", onKey);
  }, [open, onClose]);

  /** Build the URL with only the included slice of state. */
  const shareUrl = useMemo(() => {
    if (typeof window === "undefined") return "";
    const sliced: ViewSpec = { v: 1 };
    if (includes.coverField) sliced.coverField = currentSpec.coverField;
    if (includes.overlayOn) sliced.overlayOn = currentSpec.overlayOn;
    if (includes.activeView) sliced.activeView = currentSpec.activeView;
    if (includes.gqlQuery) sliced.gqlQuery = currentSpec.gqlQuery;
    const encoded = encodeView(sliced);
    const base = `${window.location.origin}${window.location.pathname}`;
    return `${base}?view=${encoded}`;
  }, [currentSpec, includes]);

  /** The embed snippet — an iframe pointing at the same URL.
   *
   *  Defense-in-depth: HTML-escape `bundle` and `shareUrl` before
   *  interpolating into attribute values. Bundle names are already
   *  validated at the route layer (`route.ts`: `[A-Za-z_][A-Za-z0-9_-]*`)
   *  but this snippet is generated for paste, so we escape regardless —
   *  if the regex ever loosens, the snippet stays safe. */
  const embedSnippet = useMemo(() => {
    const safeBundle = escapeHtmlAttr(bundle);
    const safeUrl = escapeHtmlAttr(shareUrl);
    return `<iframe src="${safeUrl}" width="100%" height="640" frameborder="0" allow="clipboard-write" title="GIGI Sheets · ${safeBundle}"></iframe>`;
  }, [shareUrl, bundle]);

  // AI-readable summary of the bundle. Built lazily from the data the
  // grid already has. See CRAWLABILITY_SPEC.md for the format contract.
  const aiSummary = useMemo(() => {
    if (!schema) return "";
    return buildBundleSummary({
      bundle,
      schema,
      rows: rows ?? [],
      kappaMap: kappaMap ?? new Map(),
      hiddenFields,
      coverField: currentSpec.coverField ?? null,
      url:
        typeof window !== "undefined"
          ? `${window.location.origin}${window.location.pathname}`
          : undefined,
    });
  }, [schema, rows, kappaMap, hiddenFields, currentSpec.coverField, bundle]);

  const aiPrompt = useMemo(
    () => (aiSummary ? buildAiPrompt(aiSummary, bundle) : ""),
    [aiSummary, bundle],
  );

  const copy = async (
    text: string,
    label: "url" | "embed" | "summary" | "prompt",
  ) => {
    try {
      await navigator.clipboard.writeText(text);
      setCopied(label);
      setTimeout(() => setCopied(null), 1800);
    } catch {
      // Fall back: select the text in the input so the user can ⌘C
      setCopied(null);
    }
  };

  if (!open) return null;

  return (
    <div
      className="share-bg"
      data-testid="share-bg"
      onClick={(e) => {
        if (e.target === e.currentTarget) onClose();
      }}
    >
      <div className="share-modal" data-testid="share-modal" role="dialog" aria-labelledby="share-title">
        <header className="share-head">
          <div>
            <h2 id="share-title">Share this view</h2>
            <p className="share-sub">
              <code>{bundle}</code> · the link encodes your current view state
              so the recipient lands on the same setup.
            </p>
          </div>
          <button
            type="button"
            className="share-close"
            onClick={onClose}
            aria-label="Close"
            data-testid="share-close"
          >
            ×
          </button>
        </header>

        <nav className="share-tabs" role="tablist">
          <button
            type="button"
            role="tab"
            aria-selected={kind === "link"}
            className={`share-tab ${kind === "link" ? "share-tab-active" : ""}`}
            onClick={() => setKind("link")}
            data-testid="share-tab-link"
          >
            Link
          </button>
          <button
            type="button"
            role="tab"
            aria-selected={kind === "export"}
            className={`share-tab ${kind === "export" ? "share-tab-active" : ""}`}
            onClick={() => setKind("export")}
            data-testid="share-tab-export"
          >
            Download
          </button>
          <button
            type="button"
            role="tab"
            aria-selected={kind === "ai"}
            className={`share-tab ${kind === "ai" ? "share-tab-active" : ""}`}
            onClick={() => setKind("ai")}
            data-testid="share-tab-ai"
          >
            AI
          </button>
        </nav>

        <div className="share-body">
          {kind === "ai" ? (
            <section data-testid="share-ai-panel" className="share-ai">
              <p className="share-ai-hint">
                A high-fidelity, AI-readable summary of this bundle —
                schema, distributions, a stratified sample, and a Davis-math
                snapshot. Paste this into Claude, ChatGPT, or any AI tool
                and it can answer questions about the data with real
                specifics. See{" "}
                <code>CRAWLABILITY_SPEC.md</code> for the format contract.
              </p>
              <textarea
                className="share-ai-textarea"
                value={aiSummary}
                readOnly
                onFocus={(e) => e.currentTarget.select()}
                data-testid="share-ai-summary"
                aria-label="Bundle summary"
              />
              <div className="share-ai-actions">
                <button
                  type="button"
                  className="share-btn share-btn-primary"
                  onClick={() => copy(aiSummary, "summary")}
                  data-testid="share-ai-copy-summary"
                >
                  {copied === "summary" ? "✓ Copied" : "Copy summary"}
                </button>
                <button
                  type="button"
                  className="share-btn"
                  onClick={() => copy(aiPrompt, "prompt")}
                  data-testid="share-ai-copy-prompt"
                  title="Copies the summary wrapped in a one-line prompt prefix"
                >
                  {copied === "prompt" ? "✓ Copied" : "Copy as AI prompt"}
                </button>
              </div>
              <p className="share-ai-note">
                <strong>Privacy:</strong> OPAQUE-tagged columns are masked
                as <code>••••••••</code> in the summary. Hidden fields are
                omitted entirely. The mask is enforced by the engine on
                real schemas; on demo bundles it's a display-only overlay
                — so don't share the summary if the demo bundle contains
                data you wouldn't put in a public document.
              </p>
            </section>
          ) : kind === "link" ? (
            <section data-testid="share-link-panel">
              <div className="share-url-row">
                <input
                  type="text"
                  className="share-url-input"
                  value={shareUrl}
                  readOnly
                  onFocus={(e) => e.currentTarget.select()}
                  data-testid="share-url"
                  aria-label="Shareable URL"
                />
                <button
                  type="button"
                  className="share-btn share-btn-primary"
                  onClick={() => copy(shareUrl, "url")}
                  data-testid="share-copy-url"
                >
                  {copied === "url" ? "✓ Copied" : "Copy link"}
                </button>
              </div>

              <h4 className="share-section-title">Include in this link</h4>
              <ul className="share-include-list">
                <IncludeRow
                  testid="share-include-cover"
                  label="Cover field"
                  hint={`Recipient will land with cover = ${currentSpec.coverField ? `"${currentSpec.coverField}"` : "(none)"}`}
                  checked={includes.coverField}
                  onChange={(v) => setIncludes((s) => ({ ...s, coverField: v }))}
                />
                <IncludeRow
                  testid="share-include-view"
                  label="Active view tab"
                  hint={`Will open on the ${currentSpec.activeView ?? "grid"} tab`}
                  checked={includes.activeView}
                  onChange={(v) => setIncludes((s) => ({ ...s, activeView: v }))}
                />
                <IncludeRow
                  testid="share-include-overlay"
                  label="Geometry overlay state"
                  hint={`Overlay will be ${currentSpec.overlayOn ? "on" : "off"}`}
                  checked={includes.overlayOn}
                  onChange={(v) => setIncludes((s) => ({ ...s, overlayOn: v }))}
                />
                <IncludeRow
                  testid="share-include-gql"
                  label="GQL draft"
                  hint="Their GQL editor will be seeded with your current query"
                  checked={includes.gqlQuery}
                  onChange={(v) => setIncludes((s) => ({ ...s, gqlQuery: v }))}
                />
              </ul>

              <h4 className="share-section-title">Quick send</h4>
              <div className="share-quick">
                <a
                  href={`mailto:?subject=${encodeURIComponent(`GIGI Sheets · ${bundle}`)}&body=${encodeURIComponent(`Take a look at this bundle:\n\n${shareUrl}`)}`}
                  className="share-btn"
                  data-testid="share-email"
                >
                  ✉ Email
                </a>
                <a
                  href={shareUrl}
                  target="_blank"
                  rel="noopener noreferrer"
                  className="share-btn"
                  data-testid="share-open-new-tab"
                >
                  ↗ Open in new tab
                </a>
              </div>

              <details className="share-embed">
                <summary>Embed in a page</summary>
                <div className="share-embed-body">
                  <textarea
                    className="share-embed-snippet"
                    value={embedSnippet}
                    readOnly
                    onFocus={(e) => e.currentTarget.select()}
                    data-testid="share-embed-snippet"
                    aria-label="Embed snippet"
                  />
                  <button
                    type="button"
                    className="share-btn"
                    onClick={() => copy(embedSnippet, "embed")}
                    data-testid="share-copy-embed"
                  >
                    {copied === "embed" ? "✓ Copied" : "Copy HTML"}
                  </button>
                </div>
              </details>
            </section>
          ) : (
            <section data-testid="share-export-panel" className="share-export">
              <p className="share-export-hint">
                Download the data itself, not just a link. The file will only
                contain rows currently visible in the grid.
              </p>
              <div className="share-export-grid">
                <button
                  type="button"
                  className="share-export-card"
                  onClick={() => {
                    onExportCsv();
                    onClose();
                  }}
                  data-testid="share-export-csv"
                >
                  <strong>CSV</strong>
                  <span>Open in Excel, Numbers, Sheets, etc.</span>
                </button>
                <button
                  type="button"
                  className="share-export-card"
                  onClick={() => {
                    onExportJson();
                    onClose();
                  }}
                  data-testid="share-export-json"
                >
                  <strong>JSON</strong>
                  <span>Structured records, one per row.</span>
                </button>
                <button
                  type="button"
                  className="share-export-card"
                  onClick={() => {
                    onExportGql();
                    onClose();
                  }}
                  data-testid="share-export-gql"
                >
                  <strong>GQL script</strong>
                  <span>
                    <code>CREATE BUNDLE</code> + <code>SECTION</code>s. Run it
                    against any GIGI engine to recreate this bundle.
                  </span>
                </button>
              </div>
            </section>
          )}
        </div>

        <footer className="share-foot">
          <span className="share-foot-note">
            Recipients see whatever your engine exposes — the link doesn't bypass
            its auth or encryption.
          </span>
          <button type="button" className="share-btn" onClick={onClose}>
            Close
          </button>
        </footer>
      </div>
    </div>
  );
}

function IncludeRow({
  label,
  hint,
  checked,
  onChange,
  testid,
}: {
  label: string;
  hint: string;
  checked: boolean;
  onChange: (next: boolean) => void;
  testid: string;
}) {
  return (
    <li>
      <label className="share-include-row">
        <input
          type="checkbox"
          checked={checked}
          onChange={(e) => onChange(e.target.checked)}
          data-testid={testid}
        />
        <span>
          <strong>{label}</strong>
          <small>{hint}</small>
        </span>
      </label>
    </li>
  );
}

/** Escape a value for safe interpolation into an HTML attribute string.
 *  Used by the embed-snippet generator above. */
function escapeHtmlAttr(s: string): string {
  return s
    .replace(/&/g, "&amp;")
    .replace(/"/g, "&quot;")
    .replace(/'/g, "&#39;")
    .replace(/</g, "&lt;")
    .replace(/>/g, "&gt;");
}
