import { useEffect, useState } from "react";
import "./Tutorial.css";

export interface TutorialStep {
  /** A short title shown at the top of the tooltip. */
  title: string;
  /** Body content — plain text or a small React tree. */
  body: React.ReactNode;
  /**
   * A `data-testid` value to spotlight. The element gets highlighted
   * and the tooltip points at it. Pass `null` for steps that don't
   * pin to a specific element (welcome / done screens).
   */
  target?: string | null;
  /**
   * Optional async action to perform when this step becomes active.
   * Useful for stepping a guided walkthrough through state changes —
   * e.g. "switch to the Kanban tab" or "click this button."
   */
  action?: () => Promise<void> | void;
  /**
   * Optional delay (ms) after the action completes before measuring
   * the spotlight target. Some actions trigger DOM mounts that need
   * a tick to settle.
   */
  settleMs?: number;
}

export interface TutorialProps {
  /** Whether the tutorial is mounted + visible. */
  open: boolean;
  /** Steps to run, in order. */
  steps: TutorialStep[];
  /** Called when the user finishes or skips. */
  onClose: () => void;
  /** Optional title displayed in the tooltip header. */
  title?: string;
}

interface SpotRect {
  top: number;
  left: number;
  width: number;
  height: number;
}

/**
 * Generic guided-tour overlay. Pass a list of `TutorialStep`s; the
 * component manages step navigation, spotlight rendering, and the
 * tooltip card. Caller is responsible for shaping each step's `action`
 * (e.g. triggering a tab switch) — the overlay just runs it and re-
 * measures the spotlight.
 *
 * Spotlight: a translucent backdrop with a transparent "hole" cut over
 * the target element so the user can still see (and reach) the UI
 * under focus. Implemented with a four-pane box-shadow trick so there's
 * no Canvas / SVG mask gymnastics.
 */
export function Tutorial({ open, steps, onClose, title }: TutorialProps) {
  const [idx, setIdx] = useState(0);
  const [spot, setSpot] = useState<SpotRect | null>(null);
  const [busy, setBusy] = useState(false);

  useEffect(() => {
    if (!open) {
      setIdx(0);
      setSpot(null);
    }
  }, [open]);

  // Run the current step's action, then measure the spotlight target.
  useEffect(() => {
    if (!open) return;
    const step = steps[idx];
    if (!step) return;

    let cancelled = false;
    const settle = step.settleMs ?? 120;

    async function run() {
      setBusy(true);
      try {
        if (step.action) await step.action();
      } catch (err) {
        // Don't crash the tour on action failure — just continue.
        console.warn("[Tutorial] step action failed:", err);
      }
      // Allow DOM to settle (tab switches, modal mounts, etc.)
      await new Promise((r) => setTimeout(r, settle));
      if (cancelled) return;
      setBusy(false);
      if (!step.target) {
        setSpot(null);
        return;
      }
      const el = document.querySelector<HTMLElement>(
        `[data-testid="${step.target}"]`,
      );
      if (!el) {
        setSpot(null);
        return;
      }
      const r = el.getBoundingClientRect();
      setSpot({ top: r.top, left: r.left, width: r.width, height: r.height });
      // Scroll the target into view if it's offscreen.
      const viewportH = window.innerHeight;
      if (r.top < 60 || r.bottom > viewportH - 60) {
        el.scrollIntoView({ behavior: "smooth", block: "center" });
        // Re-measure after the scroll settles.
        setTimeout(() => {
          const r2 = el.getBoundingClientRect();
          if (!cancelled) {
            setSpot({
              top: r2.top,
              left: r2.left,
              width: r2.width,
              height: r2.height,
            });
          }
        }, 320);
      }
    }
    run();
    return () => {
      cancelled = true;
    };
  }, [open, idx, steps]);

  // Re-measure on window resize so the spotlight tracks layout changes.
  useEffect(() => {
    if (!open) return;
    function reflow() {
      const step = steps[idx];
      if (!step?.target) return;
      const el = document.querySelector<HTMLElement>(
        `[data-testid="${step.target}"]`,
      );
      if (!el) return;
      const r = el.getBoundingClientRect();
      setSpot({ top: r.top, left: r.left, width: r.width, height: r.height });
    }
    window.addEventListener("resize", reflow);
    window.addEventListener("scroll", reflow, true);
    return () => {
      window.removeEventListener("resize", reflow);
      window.removeEventListener("scroll", reflow, true);
    };
  }, [open, idx, steps]);

  if (!open) return null;
  const step = steps[idx];
  if (!step) return null;

  const isLast = idx === steps.length - 1;
  const next = () => (isLast ? onClose() : setIdx((i) => i + 1));
  const prev = () => setIdx((i) => Math.max(0, i - 1));

  // Tooltip position: centered horizontally when no spot, otherwise
  // placed under the spotlight (or above if there's no room).
  const tooltipStyle = computeTooltipPosition(spot);

  return (
    <div
      className="tutorial-root"
      data-testid="tutorial-root"
      role="dialog"
      aria-label={title ?? "Tutorial"}
    >
      {/* Backdrop with cut-out spotlight. The four-pane trick: render
          one absolutely-positioned div per side of the spotlight, each
          dark. When there's no spot, render a single full backdrop. */}
      {spot ? (
        <SpotlightCutout spot={spot} />
      ) : (
        <div className="tutorial-backdrop" />
      )}

      {/* Spotlight outline — draws the focus ring around the target. */}
      {spot ? (
        <div
          className="tutorial-spotlight-ring"
          style={{
            top: spot.top - 6,
            left: spot.left - 6,
            width: spot.width + 12,
            height: spot.height + 12,
          }}
        />
      ) : null}

      {/* Tooltip card. */}
      <div
        className="tutorial-card"
        style={tooltipStyle}
        data-testid="tutorial-card"
      >
        <header className="tutorial-card-head">
          <span className="tutorial-card-step">
            Step {idx + 1} of {steps.length}
          </span>
          {title ? <span className="tutorial-card-tour">{title}</span> : null}
          <button
            type="button"
            className="tutorial-card-skip"
            onClick={onClose}
            data-testid="tutorial-skip"
            aria-label="Skip tour"
          >
            Skip
          </button>
        </header>
        <h3 className="tutorial-card-title">{step.title}</h3>
        <div className="tutorial-card-body">{step.body}</div>
        <footer className="tutorial-card-foot">
          <button
            type="button"
            className="tutorial-card-back"
            onClick={prev}
            disabled={idx === 0 || busy}
            data-testid="tutorial-back"
          >
            Back
          </button>
          <div className="tutorial-card-dots">
            {steps.map((_, i) => (
              <span
                key={i}
                className={`tutorial-dot ${i === idx ? "tutorial-dot-active" : ""}`}
                aria-hidden="true"
              />
            ))}
          </div>
          <button
            type="button"
            className="tutorial-card-next"
            onClick={next}
            disabled={busy}
            data-testid="tutorial-next"
          >
            {busy ? "Loading…" : isLast ? "Finish" : "Next →"}
          </button>
        </footer>
      </div>
    </div>
  );
}

/** Cut a transparent rectangle out of the dark backdrop using four
 *  overlapping dim panes (top / right / bottom / left of the spot). */
function SpotlightCutout({ spot }: { spot: SpotRect }) {
  const pad = 6; // visual breathing room around the spotlit element
  return (
    <>
      <div
        className="tutorial-pane"
        style={{ top: 0, left: 0, right: 0, height: Math.max(0, spot.top - pad) }}
      />
      <div
        className="tutorial-pane"
        style={{
          top: spot.top + spot.height + pad,
          left: 0,
          right: 0,
          bottom: 0,
        }}
      />
      <div
        className="tutorial-pane"
        style={{
          top: Math.max(0, spot.top - pad),
          left: 0,
          width: Math.max(0, spot.left - pad),
          height: spot.height + pad * 2,
        }}
      />
      <div
        className="tutorial-pane"
        style={{
          top: Math.max(0, spot.top - pad),
          left: spot.left + spot.width + pad,
          right: 0,
          height: spot.height + pad * 2,
        }}
      />
    </>
  );
}

/** Decide where to place the tooltip card relative to the spotlight. */
function computeTooltipPosition(spot: SpotRect | null): React.CSSProperties {
  const card = { w: 360, h: 200 };
  if (!spot) {
    // Centered, slightly above middle.
    return {
      top: "30vh",
      left: "50%",
      transform: "translateX(-50%)",
    };
  }
  const margin = 16;
  const viewportW = typeof window !== "undefined" ? window.innerWidth : 1024;
  const viewportH = typeof window !== "undefined" ? window.innerHeight : 768;

  // Prefer below the spot if there's room; else above.
  const belowSpace = viewportH - (spot.top + spot.height) - margin;
  const aboveSpace = spot.top - margin;
  const placeBelow = belowSpace >= card.h || belowSpace >= aboveSpace;

  let top: number;
  if (placeBelow) {
    top = spot.top + spot.height + margin;
  } else {
    top = Math.max(margin, spot.top - card.h - margin);
  }

  // Center horizontally on the spot, clamped to viewport.
  let left = spot.left + spot.width / 2 - card.w / 2;
  if (left < margin) left = margin;
  if (left + card.w > viewportW - margin) left = viewportW - card.w - margin;

  return { top, left };
}
