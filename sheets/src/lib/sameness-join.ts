/**
 * Sameness-join — cross-bundle row matching by S(a.key, b.key) ≥ τ.
 *
 * Standard SQL INNER JOIN matches on exact key equality. Two rows with
 * "INV-2026-04823" and "INV 2026 04823" stay unjoined even though they
 * refer to the same payment. Our version takes a threshold τ and a
 * sameness function: any pair where S ≥ τ becomes a match.
 *
 * For `useCanonical: true` the matching is exact on the canonicalized
 * key — the same trick Prism Dedup uses. This is the fast path; it
 * doesn't need a sameness function and scales as a hash join. For
 * sameness functions other than canonical, we fall back to O(L · R)
 * pairwise scoring (acceptable for the row counts a spreadsheet
 * handles; for millions of rows the upstream Prism stack pre-buckets).
 */

import { canonicalize } from "./canon";

export interface SamenessJoinOptions {
  /** Threshold τ — pairs with S ≥ τ are joined. Default 0.85. */
  threshold?: number;
  /**
   * Pair-sameness function. Returns S ∈ [0, 1]. Ignored when
   * `useCanonical` is true.
   */
  samenessOf?: (a: string, b: string) => number;
  /**
   * Fast path: match keys after canonicalize() (strip punctuation,
   * uppercase). Two equal canonical forms count as S = 1.
   */
  useCanonical?: boolean;
  /** When true, attach `.orphansLeft` and `.orphansRight` to the result. */
  includeOrphans?: boolean;
}

export interface JoinPair<L, R> {
  left: L;
  right: R;
  sameness: number;
}

export interface JoinResult<L, R> extends Array<JoinPair<L, R>> {
  orphansLeft: L[];
  orphansRight: R[];
}

export function samenessJoin<
  L extends Record<string, unknown>,
  R extends Record<string, unknown>,
>(
  left: L[],
  right: R[],
  keyField: string,
  opts: SamenessJoinOptions = {},
): JoinResult<L, R> {
  const threshold = opts.threshold ?? 0.85;
  const pairs: JoinPair<L, R>[] = [];
  const matchedLeft = new Set<number>();
  const matchedRight = new Set<number>();

  if (opts.useCanonical) {
    // Hash-join on canonicalized keys.
    const byCanon = new Map<string, number[]>();
    for (let j = 0; j < right.length; j++) {
      const c = canonicalize(String(right[j][keyField] ?? ""));
      if (!c) continue;
      const arr = byCanon.get(c) ?? [];
      arr.push(j);
      byCanon.set(c, arr);
    }
    for (let i = 0; i < left.length; i++) {
      const c = canonicalize(String(left[i][keyField] ?? ""));
      if (!c) continue;
      const matches = byCanon.get(c);
      if (!matches) continue;
      for (const j of matches) {
        pairs.push({ left: left[i], right: right[j], sameness: 1 });
        matchedLeft.add(i);
        matchedRight.add(j);
      }
    }
  } else if (opts.samenessOf) {
    // O(L · R) pairwise — acceptable up to the few-thousand-row range
    // a spreadsheet typically holds.
    for (let i = 0; i < left.length; i++) {
      const a = String(left[i][keyField] ?? "");
      for (let j = 0; j < right.length; j++) {
        const b = String(right[j][keyField] ?? "");
        const s = opts.samenessOf(a, b);
        if (s >= threshold) {
          pairs.push({ left: left[i], right: right[j], sameness: s });
          matchedLeft.add(i);
          matchedRight.add(j);
        }
      }
    }
  }

  const result = pairs as JoinResult<L, R>;
  if (opts.includeOrphans) {
    result.orphansLeft = left.filter((_, i) => !matchedLeft.has(i));
    result.orphansRight = right.filter((_, j) => !matchedRight.has(j));
  } else {
    result.orphansLeft = [];
    result.orphansRight = [];
  }
  return result;
}
