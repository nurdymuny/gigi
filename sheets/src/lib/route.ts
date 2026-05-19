/**
 * Tiny path → bundle / system-page parser. The app lives under
 * `/gigi/sheets/` (vite `base`).
 *
 *   /gigi/sheets/             → bundle: null, system: null   (picker)
 *   /gigi/sheets/sensors      → bundle: "sensors"
 *   /gigi/sheets/welcome      → system: "welcome"            (post-magic-link landing)
 *   /gigi/sheets/account      → system: "account"            (account dashboard)
 *
 * Bundle names are constrained the same way GIGI itself constrains
 * identifiers: `[A-Za-z_][A-Za-z0-9_-]*`. Anything else returns null so
 * the app doesn't pass garbage to the engine. The reserved system page
 * names below are taken out of the bundle namespace — a bundle literally
 * called `welcome` would be unreachable via the URL, but bundle names
 * are user-controlled so we're comfortable claiming this small set.
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

export function pathForBundle(bundle: string): string {
  return BASE + encodeURIComponent(bundle);
}

export function pathForSystem(page: SystemPage): string {
  return BASE + page;
}
