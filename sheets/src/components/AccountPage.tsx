import { useCallback, useEffect, useState } from "react";
import gigiIconUrl from "../assets/gigi-icon.svg";
import type { Account } from "../lib/use-account";
import { initialsFromEmail } from "../lib/use-account";
import "./AccountPage.css";

/**
 * Sheets-branded account dashboard. Lives at /gigi/sheets/account.
 *
 * Shows:
 *   - identity (email + initials avatar)
 *   - subscription tier
 *   - T&Cs status (version on file + when signed)
 *   - list of currently-open bundle tabs (click to jump)
 *   - sign out + back-to-picker controls
 *
 * Unauthenticated visitors see a "sign in to view your account" card
 * instead of the dashboard.
 */

export interface AccountPageProps {
  account: Account;
  openTabs: string[];
  onOpenBundle: (name: string) => void;
  onBackToPicker: () => void;
  onRequestSignIn: () => void;
}

interface AgreementInfo {
  hasSigned: boolean;
  termsVersion?: string;
  signedAt?: string;
  integrityValid?: boolean;
}

function resolveAuthBase(): string {
  const raw = (import.meta.env?.VITE_AUTH_BASE_URL ?? "") as string;
  return raw === "" ? "" : raw.replace(/\/$/, "");
}

function formatTier(tier?: string, status?: string): string {
  if (!tier) return "Free";
  const t = tier.trim();
  if (!status || status === "active") return t;
  return `${t} · ${status}`;
}

function formatDate(iso?: string): string {
  if (!iso) return "—";
  const d = new Date(iso);
  if (Number.isNaN(d.getTime())) return iso;
  return d.toLocaleString(undefined, {
    year: "numeric",
    month: "short",
    day: "numeric",
  });
}

export function AccountPage({
  account,
  openTabs,
  onOpenBundle,
  onBackToPicker,
  onRequestSignIn,
}: AccountPageProps) {
  const [agreement, setAgreement] = useState<AgreementInfo | null>(null);
  const [signingOut, setSigningOut] = useState(false);

  const loadAgreement = useCallback(async () => {
    if (account.state !== "user") return;
    const base = resolveAuthBase();
    try {
      const res = await fetch(`${base}/api/agreement/check`, {
        method: "GET",
        credentials: "include",
        headers: { accept: "application/json" },
      });
      if (!res.ok) return;
      const data = (await res.json().catch(() => ({}))) as AgreementInfo;
      setAgreement(data);
    } catch {
      /* silent — the card just falls back to the "unknown" state */
    }
  }, [account.state]);

  useEffect(() => {
    void loadAgreement();
  }, [loadAgreement]);

  const doSignOut = async () => {
    setSigningOut(true);
    await account.signOut();
    setSigningOut(false);
    onBackToPicker();
  };

  return (
    <div className="account-page-shell">
      <header className="account-page-topbar">
        <button
          type="button"
          className="account-page-brand-btn"
          onClick={onBackToPicker}
          aria-label="Back to bundles"
        >
          <img
            src={gigiIconUrl}
            className="account-page-brand-icon"
            alt="GIGI"
            draggable={false}
          />
          <span className="account-page-brand-name">GIGI Sheets</span>
          <span className="account-page-brand-sub">
            fiber bundles · for humans
          </span>
        </button>
        <button
          type="button"
          className="account-page-back"
          onClick={onBackToPicker}
        >
          ← All bundles
        </button>
      </header>

      <main className="account-page-main">
        {account.state === "loading" ? (
          <Card>
            <h1 className="account-page-h1">Loading your account…</h1>
          </Card>
        ) : null}

        {account.state === "guest" ? (
          <Card>
            <h1 className="account-page-h1">You're not signed in</h1>
            <p className="account-page-p">
              Sign in to manage your subscription, view your terms agreement,
              and jump between bundles.
            </p>
            <button
              type="button"
              className="account-page-btn account-page-btn-primary"
              onClick={onRequestSignIn}
            >
              Send sign-in link
            </button>
          </Card>
        ) : null}

        {account.state === "user" && account.email ? (
          <>
            <Card>
              <div className="account-page-identity">
                <div
                  className="account-page-avatar"
                  data-testid="account-page-avatar"
                >
                  {initialsFromEmail(account.email)}
                </div>
                <div className="account-page-identity-text">
                  <div className="account-page-eyebrow">Signed in as</div>
                  <div
                    className="account-page-email"
                    data-testid="account-page-email"
                  >
                    {account.email}
                  </div>
                  <div className="account-page-tier-row">
                    <span className="account-page-tier">
                      {formatTier(
                        account.subscription?.tier,
                        account.subscription?.status,
                      )}
                    </span>
                  </div>
                </div>
              </div>
              <div className="account-page-actions">
                <a
                  href="https://davisgeometric.com/members/"
                  className="account-page-btn"
                  target="_blank"
                  rel="noopener noreferrer"
                >
                  Manage subscription ↗
                </a>
                <button
                  type="button"
                  className="account-page-btn account-page-btn-danger"
                  onClick={() => void doSignOut()}
                  disabled={signingOut}
                >
                  {signingOut ? "Signing out…" : "Sign out"}
                </button>
              </div>
            </Card>

            <Card>
              <div className="account-page-section-head">
                <h2 className="account-page-h2">Terms &amp; conditions</h2>
              </div>
              {agreement?.hasSigned ? (
                <div className="account-page-terms-status account-page-terms-ok">
                  <div className="account-page-terms-line">
                    <span className="account-page-terms-dot account-page-terms-dot-ok" />
                    Signed
                    {agreement.signedAt
                      ? ` on ${formatDate(agreement.signedAt)}`
                      : ""}
                  </div>
                  <div className="account-page-terms-meta">
                    Version on file:{" "}
                    <code>{agreement.termsVersion ?? "—"}</code>
                    {agreement.integrityValid === false ? (
                      <span className="account-page-terms-warn">
                        · integrity check failed — contact support
                      </span>
                    ) : null}
                  </div>
                </div>
              ) : agreement && !agreement.hasSigned ? (
                <div className="account-page-terms-status">
                  <div className="account-page-terms-line">
                    <span className="account-page-terms-dot account-page-terms-dot-warn" />
                    Not signed yet
                  </div>
                  <a
                    href="/gigi/sheets/welcome"
                    className="account-page-btn account-page-btn-primary"
                  >
                    Read and sign the terms
                  </a>
                </div>
              ) : (
                <div className="account-page-terms-status">
                  <div className="account-page-terms-line">
                    <span className="account-page-terms-dot" />
                    Loading agreement status…
                  </div>
                </div>
              )}
            </Card>

            <Card>
              <div className="account-page-section-head">
                <h2 className="account-page-h2">Open bundles</h2>
                <button
                  type="button"
                  className="account-page-link-btn"
                  onClick={onBackToPicker}
                >
                  Browse all →
                </button>
              </div>
              {openTabs.length === 0 ? (
                <p className="account-page-p account-page-empty">
                  No bundles open right now. Head back to the picker to choose
                  one.
                </p>
              ) : (
                <ul className="account-page-tabs">
                  {openTabs.map((name) => (
                    <li key={name}>
                      <button
                        type="button"
                        className="account-page-tab"
                        onClick={() => onOpenBundle(name)}
                      >
                        <span className="account-page-tab-name">{name}</span>
                        <span className="account-page-tab-go">open →</span>
                      </button>
                    </li>
                  ))}
                </ul>
              )}
            </Card>
          </>
        ) : null}
      </main>

      <footer className="account-page-footer">
        GIGI Sheets · a Davis Geometric property ·{" "}
        <a
          href="https://davisgeometric.com"
          target="_blank"
          rel="noopener noreferrer"
        >
          davisgeometric.com
        </a>
      </footer>
    </div>
  );
}

function Card({ children }: { children: React.ReactNode }) {
  return <div className="account-page-card">{children}</div>;
}
