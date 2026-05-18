/**
 * Formula dependency graph + topological recompute order.
 *
 * Each formula is keyed by the A1 ref of the cell it lives in
 * (e.g. `"B5"`). The graph stores:
 *
 *   forward : Map<formulaRef, Set<sourceRef>>   the deps this formula reads
 *   reverse : Map<sourceRef,  Set<formulaRef>>  formulas that read this ref
 *
 * The reverse index is what makes "X changed, who needs recompute?" a
 * cheap lookup. Cycles are detected by Kahn's-algorithm-style topological
 * sort: any node that can't be drained means it sits on a cycle.
 *
 * Naming: this layer is bundle-scoped — caller maintains one
 * `FormulaGraph` per bundle. Formula refs are the bundle's A1 refs,
 * computed elsewhere from `(rowKey, field)`.
 */

export interface AffectedResult {
  /**
   * Formulas that need recomputation, in dependency-respecting order:
   * a formula's sources appear earlier in the list. Excludes the
   * originally-changed refs themselves (the caller has just written
   * those values; re-evaluating them is the caller's job, not this
   * cascade's). When a cycle is present, members of the cycle are
   * **omitted** from `order` and listed in `cycle` instead.
   */
  order: string[];
  /**
   * Refs participating in any cycle reachable from the change set, or
   * null if no cycle was hit. The caller is expected to write `#CIRC!`
   * to each cell in this list — they can't be evaluated.
   */
  cycle: string[] | null;
}

export class FormulaGraph {
  private forward: Map<string, Set<string>> = new Map();
  private reverse: Map<string, Set<string>> = new Map();

  /** True if a formula is registered at this cell ref. */
  has(formulaRef: string): boolean {
    return this.forward.has(formulaRef);
  }

  /** All registered formula refs (used by tooling / debugging). */
  formulas(): string[] {
    return [...this.forward.keys()];
  }

  /**
   * Register (or replace) a formula at `formulaRef` with the given
   * source dep set. Reverse index is kept consistent automatically —
   * old back-edges are dropped and new ones inserted.
   */
  setFormula(formulaRef: string, deps: Set<string>): void {
    this.removeFormula(formulaRef);
    // Snapshot to a fresh Set so caller mutations don't bleed in.
    const owned = new Set(deps);
    this.forward.set(formulaRef, owned);
    for (const d of owned) {
      let s = this.reverse.get(d);
      if (!s) {
        s = new Set();
        this.reverse.set(d, s);
      }
      s.add(formulaRef);
    }
  }

  /** Drop a formula and clean its reverse-index entries. */
  removeFormula(formulaRef: string): void {
    const old = this.forward.get(formulaRef);
    if (!old) return;
    for (const d of old) {
      const s = this.reverse.get(d);
      if (!s) continue;
      s.delete(formulaRef);
      if (s.size === 0) this.reverse.delete(d);
    }
    this.forward.delete(formulaRef);
  }

  /** Direct dependents of a single ref. Empty Set if none. */
  dependents(ref: string): Set<string> {
    return this.reverse.get(ref) ?? new Set();
  }

  /**
   * Given a set of changed source refs, return the transitive closure
   * of formulas that need recomputation, in topological order (sources
   * before dependents).
   *
   * Algorithm: BFS from the changed refs to collect every reachable
   * formula. Build the sub-graph restricted to those nodes plus their
   * inter-formula edges. Run Kahn's algorithm. Any node that can't be
   * drained is on a cycle — return those in `cycle`, the rest in
   * dependency order.
   */
  affected(changedRefs: string[]): AffectedResult {
    // 1. Collect the closure of dependent formulas (BFS via reverse).
    const closure = new Set<string>();
    const queue: string[] = [];
    for (const r of changedRefs) {
      for (const f of this.dependents(r)) {
        if (!closure.has(f)) { closure.add(f); queue.push(f); }
      }
    }
    while (queue.length > 0) {
      const f = queue.shift()!;
      for (const g of this.dependents(f)) {
        if (!closure.has(g)) { closure.add(g); queue.push(g); }
      }
    }
    if (closure.size === 0) return { order: [], cycle: null };

    // 2. Build the in-degree map restricted to edges *inside* the closure.
    // A formula's "in-edges" are its deps that are *also* formula nodes
    // in the closure (i.e. cells whose values are themselves about to
    // be recomputed). Deps outside the closure are leaves — already-
    // written source cells the caller just changed.
    const inDeg = new Map<string, number>();
    const inDeps = new Map<string, string[]>();
    for (const f of closure) {
      const deps: string[] = [];
      const forward = this.forward.get(f);
      if (forward) {
        for (const d of forward) {
          if (closure.has(d)) deps.push(d);
        }
      }
      inDeps.set(f, deps);
      inDeg.set(f, deps.length);
    }

    // 3. Kahn's: repeatedly drain nodes with in-degree 0.
    const order: string[] = [];
    const ready: string[] = [];
    for (const [f, n] of inDeg) if (n === 0) ready.push(f);
    while (ready.length > 0) {
      const f = ready.shift()!;
      order.push(f);
      for (const g of this.dependents(f)) {
        if (!closure.has(g)) continue;
        const n = (inDeg.get(g) ?? 0) - 1;
        inDeg.set(g, n);
        if (n === 0) ready.push(g);
      }
    }

    // 4. Anything still with in-degree > 0 sits on a cycle.
    const cycle: string[] = [];
    for (const [f, n] of inDeg) if (n > 0) cycle.push(f);
    return { order, cycle: cycle.length === 0 ? null : cycle };
  }
}
