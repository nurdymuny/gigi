import { useEffect } from "react";
import type { Subscription } from "../lib/use-account";
import "./AccountMenu.css";

export interface AccountMenuProps {
  open: boolean;
  email: string;
  subscription: Subscription | null | undefined;
  onClose: () => void;
  onSignOut: () => Promise<void>;
}

/**
 * Tiny popover anchored to the topbar avatar. Shows who's signed in,
 * any subscription tier, and a Sign-out button.
 */
export function AccountMenu({
  open,
  email,
  subscription,
  onClose,
  onSignOut,
}: AccountMenuProps) {
  useEffect(() => {
    if (!open) return;
    function onKey(e: KeyboardEvent) {
      if (e.key === "Escape") onClose();
    }
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [open, onClose]);

  if (!open) return null;

  const signOut = async () => {
    await onSignOut();
    onClose();
  };

  return (
    <div
      className="account-menu-bg"
      data-testid="account-menu-bg"
      onClick={(e) => {
        if (e.target === e.currentTarget) onClose();
      }}
    >
      <div className="account-menu" data-testid="account-menu" role="dialog">
        <div className="account-menu-head">
          <div className="account-eyebrow">Signed in as</div>
          <div className="account-email" data-testid="account-email">
            {email}
          </div>
          {subscription?.tier ? (
            <div className="account-tier" data-testid="account-tier">
              {subscription.tier}
              {subscription.status && subscription.status !== "active"
                ? ` · ${subscription.status}`
                : null}
            </div>
          ) : null}
        </div>
        <div className="account-menu-body">
          <a
            href="https://davisgeometric.com/members/"
            className="account-link"
            target="_blank"
            rel="noopener noreferrer"
          >
            Manage account ↗
          </a>
          <button
            type="button"
            className="account-signout"
            onClick={() => void signOut()}
            data-testid="account-signout"
          >
            Sign out
          </button>
        </div>
      </div>
    </div>
  );
}
