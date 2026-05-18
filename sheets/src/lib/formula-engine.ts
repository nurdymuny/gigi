/**
 * Reactive formula engine — composes evaluate + collectDeps + FormulaGraph.
 *
 * The engine is the **demo / reference composition** of the Phase 2
 * primitives. App.tsx mirrors this pattern at the bundle level, with
 * the formula sidecar + bundle row store standing in for the in-memory
 * `cells` map here. Pinning the composition in a single class also
 * lets the test suite exercise the full cascade + #CIRC! behavior
 * without dragging in React or the bundle layer.
 *
 * Wire-up:
 *
 *   setValue(ref, v)     → write v; cascade to formulas reading ref
 *   setFormula(ref, txt) → parse, extract deps, register in graph,
 *                          evaluate, write result, cascade
 *   clearFormula(ref)    → drop the formula text; cell value is
 *                          preserved as the last evaluated result
 *
 * Cycles detected by `FormulaGraph.affected` cause `#CIRC!` to be
 * written to every cell in the cycle; downstream readers then get the
 * sentinel and aggregate poisoning (Phase 1.C) takes over.
 */

import {
  collectDeps,
  evaluate,
  type FormulaContext,
  type FormulaValue,
} from "./formula";
import { FormulaGraph } from "./formula-graph";

export interface FormulaEngineOptions {
  /**
   * Named-field resolver for `=SUM(temperature)` style refs. Same
   * contract as `FormulaContext.resolveField`; if omitted, named fields
   * resolve to `#NAME!`.
   */
  resolveField?: (name: string) => string[] | null;
  /** Same contract as `FormulaContext.fieldRowRef` (Phase 1.D). */
  fieldRowRef?: (name: string, row: number) => string | null;
  /** Same contract as `FormulaContext.today` (Phase 1.5.D). */
  today?: () => number;
}

export class FormulaEngine {
  private graph = new FormulaGraph();
  private cells: Map<string, FormulaValue> = new Map();
  private formulas: Map<string, string> = new Map();
  private opts: FormulaEngineOptions;

  constructor(opts: FormulaEngineOptions = {}) {
    this.opts = opts;
  }

  /** Current value at a cell, or null if unset. */
  get(ref: string): FormulaValue {
    return this.cells.get(ref) ?? null;
  }

  /** Raw formula text at a cell, or null if it's a plain value. */
  getFormula(ref: string): string | null {
    return this.formulas.get(ref) ?? null;
  }

  /**
   * Write a plain value. Any prior formula at this cell is removed —
   * "typing a literal over a formula replaces it" (matches Excel/Sheets).
   */
  setValue(ref: string, value: FormulaValue): void {
    if (this.formulas.has(ref)) {
      this.formulas.delete(ref);
      this.graph.removeFormula(ref);
    }
    this.cells.set(ref, value);
    this.cascade([ref]);
  }

  /**
   * Install or replace a formula at `ref`. `text` must start with `=`;
   * anything else is rejected (callers should use `setValue` for plain
   * values).
   */
  setFormula(ref: string, text: string): void {
    if (!text.startsWith("=")) {
      throw new Error(`FormulaEngine.setFormula: '${text}' is not a formula`);
    }
    const dep = collectDeps(text, { resolveField: this.opts.resolveField });
    // Dep-extraction errors (#ERROR! / #NAME!) get written as the cell
    // value AND we register an empty dep set so the graph stays
    // consistent. The user fixes the formula by re-calling setFormula.
    if (dep.error) {
      this.formulas.set(ref, text);
      this.graph.setFormula(ref, new Set());
      this.cells.set(ref, dep.error);
      this.cascade([ref]);
      return;
    }
    this.formulas.set(ref, text);
    this.graph.setFormula(ref, dep.deps);
    // Evaluate the new formula first, then cascade.
    this.evalAndWrite(ref);
    this.cascade([ref]);
  }

  /**
   * Remove the formula at `ref` but keep its last-evaluated value
   * around. The cell becomes a regular leaf in the graph.
   */
  clearFormula(ref: string): void {
    if (!this.formulas.has(ref)) return;
    this.formulas.delete(ref);
    this.graph.removeFormula(ref);
    // Downstream still depends on `ref` only via the graph's reverse
    // index; the value is frozen at whatever it last computed to, so
    // no recompute is needed — dependents already saw that value.
  }

  /** Internal: build a FormulaContext that reads the engine's cell store. */
  private buildContext(): FormulaContext {
    return {
      cell: (ref) => this.cells.get(ref) ?? null,
      sameness: () => 0.5,
      kappa: () => 0,
      cohort: () => "",
      resolveField: this.opts.resolveField,
      fieldRowRef: this.opts.fieldRowRef,
      today: this.opts.today,
    };
  }

  /** Evaluate one formula and write its result (or error) into `cells`. */
  private evalAndWrite(ref: string): void {
    const text = this.formulas.get(ref);
    if (!text) return;
    const r = evaluate(text, this.buildContext());
    this.cells.set(ref, r.error ?? r.value);
  }

  /**
   * Propagate a change set: write `#CIRC!` to any cycle members, then
   * re-evaluate every dependent formula in topological order. Sources
   * already-written by the caller are not in `order`.
   */
  private cascade(changedRefs: string[]): void {
    const r = this.graph.affected(changedRefs);
    if (r.cycle) {
      for (const c of r.cycle) {
        this.cells.set(c, "#CIRC!");
      }
    }
    for (const f of r.order) {
      this.evalAndWrite(f);
    }
  }
}
