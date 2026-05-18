/**
 * Tiny path → bundle parser. The app lives under `/gigi/sheets/` (vite
 * `base`), and the first path segment after the base is the bundle name.
 *
 *   /gigi/sheets/             → null     (caller picks a default)
 *   /gigi/sheets/sensors      → "sensors"
 *   /gigi/sheets/sensors/foo  → "sensors" (anything past the first segment is ignored)
 *
 * Bundle names are constrained the same way GIGI itself constrains
 * identifiers: `[A-Za-z_][A-Za-z0-9_-]*`. Anything else returns null so
 * the app doesn't pass garbage to the engine.
 */

const BASE = "/gigi/sheets/";
const BUNDLE_RE = /^[A-Za-z_][A-Za-z0-9_-]*$/;

export function bundleFromPath(pathname: string): string | null {
  let p = pathname;
  if (p.startsWith(BASE)) p = p.slice(BASE.length);
  else if (p === BASE.replace(/\/$/, "")) return null;
  // Strip leading slash if any (in case base is missing).
  if (p.startsWith("/")) p = p.slice(1);
  const first = p.split("/")[0] ?? "";
  if (!first) return null;
  if (!BUNDLE_RE.test(first)) return null;
  return first;
}

export function pathForBundle(bundle: string): string {
  return BASE + encodeURIComponent(bundle);
}
