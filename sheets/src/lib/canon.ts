/**
 * Canonical form for reference / identifier strings.
 *
 * Real-world reference fields drift: "INV-2026-04823" and "INV 2026 04823"
 * and "INV/2026/04823" are all the same payment booked three different
 * ways. The canonical form is the upper-cased, punctuation-stripped string
 * — when two references canonicalize to the same value, they're the same
 * reference for matching purposes.
 *
 * Used by:
 *   - Prism Dedup (the Reference subblock of the embedding)
 *   - Find & replace (canonical-match mode)
 *   - Sort (canonical-aware sort)
 *   - Sameness-join across bundles (reference-key alignment)
 */

const STRIP = /[\s\-/_.,]+/g;

/** Uppercase + strip whitespace/dashes/slashes/underscores/dots/commas. */
export function canonicalize(s: string): string {
  if (!s) return "";
  return s.toUpperCase().replace(STRIP, "");
}

/** True iff two strings canonicalize to the same non-empty form, or both
 *  canonicalize to empty (covers the "both empty" edge case symmetrically). */
export function canonicalMatches(a: string, b: string): boolean {
  return canonicalize(a) === canonicalize(b);
}

/**
 * Overlapping 3-character windows over the canonicalized string. Used by
 * the embedding layer to hash references into the φ_sem subblock — small
 * variations in formatting collapse to the same trigram set, so the dot
 * product after hashing comes out high.
 */
export function trigrams(s: string): string[] {
  const c = canonicalize(s);
  if (c.length < 3) return [];
  const out: string[] = [];
  for (let i = 0; i <= c.length - 3; i++) {
    out.push(c.slice(i, i + 3));
  }
  return out;
}
