import { useCallback, useState } from "react";

/**
 * Prism workflow free-run counter. Anonymous users get FREE_RUN_LIMIT
 * total runs across all workflows (not per workflow); after that the
 * upsell modal appears. Signed-in users with a Prism subscription get
 * unlimited runs.
 *
 * Backed by localStorage so the counter survives reloads — same browser,
 * same identity, same allowance. The counter is intentionally NOT
 * synced server-side (it's not security; it's a soft upsell trigger).
 */

export const FREE_RUN_LIMIT = 3;
const STORAGE_KEY = "gigi.sheets.prism_credits_used";

export interface PrismCredits {
  /** Number of runs consumed so far. */
  used: number;
  /** Total allowance (FREE_RUN_LIMIT for guests, Infinity for subscribers). */
  limit: number;
  /** Runs left. Infinity when subscribed. */
  remaining: number;
  /** True when subscribed (limit lifted). */
  unlimited: boolean;
  /** True when the user can run at least one more workflow. */
  canRun: boolean;
  /** Consume a credit. No-op when at limit and not subscribed. */
  consume: () => void;
  /** Wipe the counter back to 0 (debug / "reset trial" affordance). */
  reset: () => void;
}

function readUsed(): number {
  if (typeof localStorage === "undefined") return 0;
  const raw = localStorage.getItem(STORAGE_KEY);
  const n = raw ? parseInt(raw, 10) : 0;
  return Number.isFinite(n) && n >= 0 ? n : 0;
}

function writeUsed(n: number): void {
  if (typeof localStorage === "undefined") return;
  localStorage.setItem(STORAGE_KEY, String(n));
}

export function usePrismCredits(opts: { subscribed: boolean }): PrismCredits {
  const { subscribed } = opts;
  const [used, setUsed] = useState<number>(() => readUsed());

  const consume = useCallback(() => {
    if (subscribed) {
      // Still count subscribed-user runs for analytics — never gate them.
      setUsed((u) => {
        const next = u + 1;
        writeUsed(next);
        return next;
      });
      return;
    }
    setUsed((u) => {
      if (u >= FREE_RUN_LIMIT) return u;
      const next = u + 1;
      writeUsed(next);
      return next;
    });
  }, [subscribed]);

  const reset = useCallback(() => {
    setUsed(0);
    writeUsed(0);
  }, []);

  const limit = subscribed ? Infinity : FREE_RUN_LIMIT;
  const remaining = subscribed ? Infinity : Math.max(0, FREE_RUN_LIMIT - used);
  const canRun = subscribed || used < FREE_RUN_LIMIT;

  return {
    used,
    limit,
    remaining,
    unlimited: subscribed,
    canRun,
    consume,
    reset,
  };
}
