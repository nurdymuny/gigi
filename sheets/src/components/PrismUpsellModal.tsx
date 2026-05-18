import { useEffect } from "react";
import "./PrismUpsellModal.css";

export interface PrismUpsellModalProps {
  open: boolean;
  onClose: () => void;
  /** Opens the sign-in flow (magic link → Prism account check). */
  onSignIn: () => void;
  /** Resets the free-run counter back to 0. Optional — when omitted, the
   *  "Reset trial" link is hidden. */
  onResetTrial?: () => void;
}

/**
 * Shown when a guest runs out of free Prism workflow runs. Navy + gold
 * pitch-deck styling so it reads as Prism's marketing, not GIGI's.
 */
export function PrismUpsellModal({
  open,
  onClose,
  onSignIn,
  onResetTrial,
}: PrismUpsellModalProps) {
  useEffect(() => {
    if (!open) return;
    function onKey(e: KeyboardEvent) {
      if (e.key === "Escape") onClose();
    }
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [open, onClose]);

  if (!open) return null;

  return (
    <div
      className="prism-upsell-bg"
      data-testid="prism-upsell-bg"
      onClick={(e) => {
        if (e.target === e.currentTarget) onClose();
      }}
    >
      <div
        className="prism-upsell-modal"
        data-testid="prism-upsell-modal"
        role="dialog"
        aria-labelledby="prism-upsell-title"
      >
        <button
          type="button"
          className="prism-upsell-close"
          onClick={onClose}
          aria-label="Close"
          data-testid="prism-upsell-close"
        >
          ×
        </button>
        <div className="prism-upsell-mark">◇ PRISM</div>
        <h2 id="prism-upsell-title">You've used your 3 free Prism runs</h2>
        <p className="prism-upsell-sub">
          Prism is the production payment-reconciliation engine — geometric
          AI for cross-format matching, sanctions screening, behavioral
          surveillance, and books reconciliation. It's normally licensed by
          banks.
        </p>

        <div className="prism-upsell-tiers">
          <div className="prism-upsell-tier">
            <div className="prism-upsell-tier-name">Pilot</div>
            <div className="prism-upsell-tier-price">$50K / yr</div>
            <ul>
              <li>1M transactions</li>
              <li>Single format pair</li>
              <li>90-day trial option</li>
            </ul>
          </div>
          <div className="prism-upsell-tier prism-upsell-tier-featured">
            <div className="prism-upsell-tier-name">Growth</div>
            <div className="prism-upsell-tier-price">$200K / yr</div>
            <ul>
              <li>50M transactions</li>
              <li>All format pairs</li>
              <li>API integration</li>
            </ul>
          </div>
          <div className="prism-upsell-tier">
            <div className="prism-upsell-tier-name">Enterprise</div>
            <div className="prism-upsell-tier-price">$500K+ / yr</div>
            <ul>
              <li>Unlimited volume</li>
              <li>SLAs + on-prem</li>
              <li>Custom rails</li>
            </ul>
          </div>
        </div>

        <div className="prism-upsell-actions">
          <a
            href="mailto:bee_davis@alumni.brown.edu?subject=Prism%20%E2%80%94%20talk%20to%20sales&body=Hi%20Bee%2C%0A%0AI%20was%20using%20GIGI%20Sheets%20and%20tried%20Prism's%20demo%20workflows.%20I'd%20like%20to%20learn%20more%20about%20a%20Prism%20pilot.%0A%0A"
            className="prism-upsell-btn prism-upsell-btn-primary"
            data-testid="prism-upsell-contact"
          >
            Talk to sales →
          </a>
          <button
            type="button"
            className="prism-upsell-btn"
            onClick={onSignIn}
            data-testid="prism-upsell-signin"
          >
            I have a Prism account — sign in
          </button>
        </div>

        <p className="prism-upsell-fine">
          Or close this dialog and keep using GIGI Sheets — everything other
          than Prism workflows is free forever.
          {onResetTrial ? (
            <>
              {" · "}
              <button
                type="button"
                className="prism-upsell-reset"
                onClick={() => {
                  onResetTrial();
                  onClose();
                }}
                data-testid="prism-upsell-reset"
              >
                Reset trial counter
              </button>
            </>
          ) : null}
        </p>
      </div>
    </div>
  );
}
