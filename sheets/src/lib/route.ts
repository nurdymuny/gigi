/**
 * URL ↔ bundle / system-page parser. The app lives under
 * `/gigi/sheets/` (vite `base`).
 *
 * URL scheme:
 *   /gigi/sheets/             → picker
 *   /gigi/sheets/#sensors     → bundle: "sensors"          (HASH-based)
 *   /gigi/sheets/welcome      → system: "welcome"          (static stub)
 *   /gigi/sheets/account      → system: "account"          (static stub)
 *   /gigi/sheets/sensors      → legacy path-based bundle URL (back-compat
 *                                read-only — Vercel's rewrite engine
 *                                drops sub-paths under directories with
 *                                index.html, so we can't reliably serve
 *                                bundle URLs in the path. New navigation
 *                                always uses hash. Legacy URLs still
 *                                parse if a user happens to land on one
 *                                via the SPA still being warm.)
 *
 * Bundle names: `[A-Za-z_][A-Za-z0-9_-]*` (same as GIGI's identifier
 * grammar). Anything else returns null so we never pass garbage to
 * the engine. System page names are reserved out of that namespace
 * — a bundle literally called "welcome" would be unreachable via
 * the URL, but bundle names are user-controlled and we're comfortable
 * claiming this small set.
 */

const BASE = "/gigi/sheets/";
const BUNDLE_RE = /^[A-Za-z_][A-Za-z0-9_-]*$/;

export type SystemPage = "welcome" | "account";
const SYSTEM_PAGES: ReadonlySet<SystemPage> = new Set(["welcome", "account"]);

function firstSegment(pathname: string): string {
  let p = pathname;
  if (p.startsWith(BASE)) p = p.slice(BASE.length);
  else if (p === BASE.replace(/\/$/, "")) return "";
  if (p.startsWith("/")) p = p.slice(1);
  return p.split("/")[0] ?? "";
}

function parseHash(hash: string): string {
  // Drop leading '#' if present, then URL-decode in case the bundle
  // name has any URI-encoded chars (it shouldn't, given BUNDLE_RE,
  // but defensive).
  const raw = hash.startsWith("#") ? hash.slice(1) : hash;
  try {
    return decodeURIComponent(raw);
  } catch {
    return raw;
  }
}

/**
 * Resolve the active bundle from either window.location.hash (the
 * canonical source post-Phase-B) or, as a legacy fallback, the
 * pathname. Hash wins when both are present.
 */
export function bundleFromLocation(
  pathname: string,
  hash: string,
): string | null {
  const fromHash = parseHash(hash);
  if (fromHash) {
    if ((SYSTEM_PAGES as ReadonlySet<string>).has(fromHash)) return null;
    return BUNDLE_RE.test(fromHash) ? fromHash : null;
  }
  return bundleFromPath(pathname);
}

/**
 * Legacy path-only bundle parser. Kept exported for code paths that
 * read from a static pathname (tests, server-side rendering helpers).
 * Production navigation uses `bundleFromLocation`.
 */
export function bundleFromPath(pathname: string): string | null {
  const first = firstSegment(pathname);
  if (!first) return null;
  // Reserve system-page names; the bundle hook should not load them.
  if ((SYSTEM_PAGES as ReadonlySet<string>).has(first)) return null;
  if (!BUNDLE_RE.test(first)) return null;
  return first;
}

export function systemPageFromPath(pathname: string): SystemPage | null {
  const first = firstSegment(pathname);
  if (!first) return null;
  return (SYSTEM_PAGES as ReadonlySet<string>).has(first)
    ? (first as SystemPage)
    : null;
}

/**
 * Canonical URL for a bundle. Returns a HASH-based URL so the page
 * loads reliably on hard-refresh — see the file header note about
 * Vercel's rewrite engine dropping sub-paths under a directory that
 * has its own index.html.
 */
export function pathForBundle(bundle: string): string {
  return BASE + "#" + encodeURIComponent(bundle);
}

export function pathForSystem(page: SystemPage): string {
  return BASE + page;
}
