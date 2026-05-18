/**
 * Tiny formula engine for GIGI Sheets.
 *
 * Scope (intentionally narrow):
 *   - Literals: numbers, strings ("...")
 *   - Cell refs: A1, B12 (single only — no $ABS or sheet qualifiers)
 *   - Ranges:    A1:A4 (used inside function args)
 *   - Operators: + − * / and unary − with conventional precedence
 *   - Functions: SUM, AVG, AVERAGE, MIN, MAX, COUNT, IF
 *   - GIGI primitives: SAME(a,b), DIST(a,b), K(ref), COHORT("col")
 *
 * Anything else returns a typed error sentinel (#NAME!, #ERROR!, #DIV0!).
 * Full Excel breadth is out of scope (see FEATURE_PARITY.md).
 *
 * Architecture: tokenizer → recursive-descent parser → tree walk evaluator.
 * The evaluator never throws; errors propagate as result objects.
 */

// (formula evaluation derives DIST from SAME via the Davis identity inline,
//  so we don't import davisDistance here.)
import { matchesPredicate, parsePredicate } from "./formula-predicate";

export type FormulaValue = number | string | boolean | null;
export type FormulaError = "#ERROR!" | "#NAME!" | "#DIV0!" | "#REF!" | "#CIRC!" | "#VALUE!";

export interface FormulaResult {
  value: FormulaValue;
  error: FormulaError | null;
}

export interface FormulaContext {
  /** Lookup a cell value by its A1-style reference. Returns null for unknown. */
  cell: (ref: string) => FormulaValue;
  /** Davis sameness between the rows containing two cells. */
  sameness: (refA: string, refB: string) => number;
  /** Davis curvature κ for the row containing this cell. */
  kappa: (ref: string) => number;
  /** Cohort name for a given column. */
  cohort: (col: string) => string;
  /**
   * Named-field resolver. Given a field name, return the list of A1-style
   * cell refs for that column (in bundle row order). Return null if the
   * name is not a known field — caller then emits `#NAME!`.
   *
   * Recommended idiom for `=SUM(temperature)`, `=MEDIAN(amount_usd)`, etc.
   * Resolution is **case-sensitive** for field names (matches the engine
   * schema). Reserved function names always win — see `RESERVED_NAMES`.
   */
  resolveField?: (name: string) => string[] | null;
  /**
   * Single-row named-field lookup for the `temperature[5]` notation.
   * Returns the A1 ref for that row, or null if `row` is out of bounds.
   * Bounds-checked by the caller; null → `#REF!`.
   */
  fieldRowRef?: (name: string, row: number) => string | null;
  /**
   * Dense-rank of a row by κ descending — 1 is the highest κ. Used by
   * `=KAPPA_RANK(ref)` (FORMULAS_SPEC §"GIGI primitives"). Optional;
   * `null` return means the ref doesn't correspond to a known row, and
   * the evaluator surfaces `#REF!`.
   */
  kappaRank?: (ref: string) => number | null;
  /**
   * Dense-rank of a row by Davis sameness against a pivot ref, desc.
   * The pivot itself always ranks 1 (S=1 with itself). `null` return
   * means the pivot or the row is unresolvable → `#REF!`.
   */
  samenessRank?: (pivotRef: string, ref: string) => number | null;
  /**
   * Source for `=TODAY()`. Returns the current date as a **serial day**
   * (days since Unix epoch, UTC). Optional so older callers keep working;
   * if absent, TODAY falls back to `Math.floor(Date.now() / 86400000)`.
   *
   * "Deterministic per evaluation" (FORMULAS_SPEC) means the host is
   * free to capture a single timestamp at bundle-load and reuse it for
   * every formula in that view — never time-of-day-volatile.
   */
  today?: () => number;
}

/**
 * Reserved function names — these ALWAYS parse as function calls, never
 * as field refs. Per FORMULAS_SPEC §"Identifier disambiguation rule": a
 * bundle with a field literally named `median` cannot access it as a
 * bare identifier in a formula; the function wins. Access via A1.
 *
 * Comparison is case-insensitive on the formula side — `=MEDIAN(…)`,
 * `=median(…)`, `=Median(…)` are all the same call.
 */
export const RESERVED_NAMES: Set<string> = new Set([
  "SUM", "AVG", "AVERAGE", "MIN", "MAX", "COUNT", "COUNTA", "IF",
  "MOD", "ABS", "ROUND", "CONCAT",
  "MEDIAN", "STDEV", "STDEVP", "VAR", "VARP", "PERCENTILE", "QUARTILE",
  "SUMIF", "COUNTIF", "AVERAGEIF",
  "SUMIFS", "COUNTIFS", "AVERAGEIFS", "MINIFS", "MAXIFS",
  "LEN", "LOWER", "UPPER", "TRIM",
  "YEAR", "MONTH", "DAY", "DATEDIF", "TO_DATE", "TODAY",
  "SAME", "DIST", "K", "COHORT", "KAPPA_RANK", "SAMENESS_RANK",
]);

const ok = (value: FormulaValue): FormulaResult => ({ value, error: null });
const err = (e: FormulaError): FormulaResult => ({ value: null, error: e });

/** Public entry point. Accepts a raw cell value; if it starts with `=` it's
 *  treated as a formula, otherwise the input is returned verbatim. */
export function evaluate(input: string, ctx: FormulaContext): FormulaResult {
  if (input === "") return ok(null);
  if (!input.startsWith("=")) return ok(input);
  try {
    const tokens = tokenize(input.slice(1));
    const parser = new Parser(tokens);
    const ast = parser.parseExpression();
    if (!parser.eof()) return err("#ERROR!");
    return evalNode(ast, ctx);
  } catch (e) {
    if (e instanceof FormulaNameError) return err("#NAME!");
    if (e instanceof FormulaParseError) return err("#ERROR!");
    return err("#ERROR!");
  }
}

// ── tokenizer ──────────────────────────────────────────────────────────

type TokenKind =
  | "number"
  | "string"
  | "ident"
  | "ref"
  | "lparen"
  | "rparen"
  | "lbracket" // [   (row index on a named field: temperature[3])
  | "rbracket" // ]
  | "comma"
  | "colon"
  | "plus"
  | "minus"
  | "star"
  | "slash"
  | "caret"   // ^   (power, right-assoc)
  | "percent" // %   (postfix only — 5% = 0.05)
  | "amp"     // &   (string concat)
  | "eq"      // =   (equality)
  | "neq"     // <>  (inequality)
  | "lt"      // <
  | "lte"     // <=
  | "gt"      // >
  | "gte";    // >=

interface Token {
  kind: TokenKind;
  text: string;
}

class FormulaParseError extends Error {}
/**
 * Thrown by the parser when a reserved function name appears outside a
 * call context (e.g. `=MEDIAN` with no parens). Translated to `#NAME!`
 * by `evaluate()`. Kept separate from `FormulaParseError` (→ `#ERROR!`)
 * because the user-visible diagnostic is different: this is "you wrote
 * a function name where a value was expected," not "syntax broken."
 */
class FormulaNameError extends Error {}

function tokenize(src: string): Token[] {
  const out: Token[] = [];
  let i = 0;
  while (i < src.length) {
    const c = src[i];
    if (c === " " || c === "\t") {
      i++;
      continue;
    }
    if (c === "(") { out.push({ kind: "lparen", text: "(" }); i++; continue; }
    if (c === ")") { out.push({ kind: "rparen", text: ")" }); i++; continue; }
    if (c === "[") { out.push({ kind: "lbracket", text: "[" }); i++; continue; }
    if (c === "]") { out.push({ kind: "rbracket", text: "]" }); i++; continue; }
    if (c === ",") { out.push({ kind: "comma", text: "," }); i++; continue; }
    if (c === ":") { out.push({ kind: "colon", text: ":" }); i++; continue; }
    if (c === "+") { out.push({ kind: "plus", text: "+" }); i++; continue; }
    if (c === "-") { out.push({ kind: "minus", text: "-" }); i++; continue; }
    if (c === "*") { out.push({ kind: "star", text: "*" }); i++; continue; }
    if (c === "/") { out.push({ kind: "slash", text: "/" }); i++; continue; }
    if (c === "^") { out.push({ kind: "caret", text: "^" }); i++; continue; }
    if (c === "%") { out.push({ kind: "percent", text: "%" }); i++; continue; }
    if (c === "&") { out.push({ kind: "amp", text: "&" }); i++; continue; }
    if (c === "=") { out.push({ kind: "eq", text: "=" }); i++; continue; }
    if (c === "<") {
      const next = src[i + 1];
      if (next === "=") { out.push({ kind: "lte", text: "<=" }); i += 2; continue; }
      if (next === ">") { out.push({ kind: "neq", text: "<>" }); i += 2; continue; }
      out.push({ kind: "lt", text: "<" }); i++; continue;
    }
    if (c === ">") {
      const next = src[i + 1];
      if (next === "=") { out.push({ kind: "gte", text: ">=" }); i += 2; continue; }
      out.push({ kind: "gt", text: ">" }); i++; continue;
    }
    if (c === '"') {
      // String literal with "" escape (Excel convention): two adjacent
      // quotes inside a string literal produce a single quote.
      let j = i + 1;
      let buf = "";
      while (j < src.length) {
        if (src[j] === '"') {
          if (src[j + 1] === '"') {
            // Escaped quote — emit one quote, advance past both.
            buf += '"';
            j += 2;
            continue;
          }
          // Closing quote.
          break;
        }
        buf += src[j++];
      }
      if (j >= src.length) throw new FormulaParseError("unterminated string");
      out.push({ kind: "string", text: buf });
      i = j + 1;
      continue;
    }
    if (/[0-9.]/.test(c)) {
      let j = i;
      while (j < src.length && /[0-9.]/.test(src[j])) j++;
      out.push({ kind: "number", text: src.slice(i, j) });
      i = j;
      continue;
    }
    if (/[A-Za-z_]/.test(c)) {
      let j = i;
      while (j < src.length && /[A-Za-z0-9_]/.test(src[j])) j++;
      const word = src.slice(i, j);
      // Cell ref pattern: one-or-more letters followed by one-or-more digits.
      // Anything else is an identifier (function name).
      if (/^[A-Za-z]+[0-9]+$/.test(word)) {
        out.push({ kind: "ref", text: word.toUpperCase() });
      } else {
        out.push({ kind: "ident", text: word });
      }
      i = j;
      continue;
    }
    throw new FormulaParseError(`unexpected character: ${c}`);
  }
  return out;
}

// ── AST ────────────────────────────────────────────────────────────────

type BinaryOp =
  | "+" | "-" | "*" | "/" | "^" | "&"
  | "=" | "<>" | "<" | "<=" | ">" | ">=";

type Node =
  | { kind: "number"; value: number }
  | { kind: "string"; value: string }
  | { kind: "ref"; ref: string }
  | { kind: "range"; from: string; to: string }
  | { kind: "fieldRef"; field: string }
  | { kind: "fieldRowRef"; field: string; rowArg: Node }
  | { kind: "fieldRangeRef"; field: string; fromArg: Node; toArg: Node }
  | { kind: "unary"; op: "-"; arg: Node }
  | { kind: "postfix"; op: "%"; arg: Node }
  | { kind: "binary"; op: BinaryOp; left: Node; right: Node }
  | { kind: "call"; name: string; args: Node[] };

// ── parser (recursive descent, conventional precedence) ────────────────

/**
 * Hard ceiling on parse-time recursion depth. A maliciously-deep
 * `=((((…))))` would otherwise blow the JS stack — sample bundles
 * could embed a 50k-deep formula via the sidecar, and the page would
 * hang before any user input gated it. 256 is well past any real
 * formula a user would write (the deepest realistic case is a
 * 4-5 level nested aggregate, e.g. `=IF(A>0, SUM(B1:B5), AVG(C:C))`).
 */
const MAX_PARSE_DEPTH = 256;

class Parser {
  private pos = 0;
  private depth = 0;
  constructor(private readonly tokens: Token[]) {}

  eof(): boolean {
    return this.pos >= this.tokens.length;
  }

  peek(): Token | null {
    return this.tokens[this.pos] ?? null;
  }

  consume(kind: TokenKind): Token {
    const t = this.peek();
    if (!t || t.kind !== kind) {
      throw new FormulaParseError(`expected ${kind}, got ${t?.kind ?? "EOF"}`);
    }
    this.pos++;
    return t;
  }

  /** Bumps the depth counter; every `parseExpression()` entry calls this
   *  to refuse pathologically nested inputs before they crash the runtime. */
  private enter(): void {
    this.depth++;
    if (this.depth > MAX_PARSE_DEPTH) {
      throw new FormulaParseError(
        `formula exceeds maximum nesting depth of ${MAX_PARSE_DEPTH}`,
      );
    }
  }
  private leave(): void {
    this.depth--;
  }

  parseExpression(): Node {
    this.enter();
    try {
      return this.parseComparison();
    } finally {
      this.leave();
    }
  }

  /** Lowest precedence after literals — = <> < <= > >=. Left-assoc. */
  parseComparison(): Node {
    let left = this.parseConcat();
    while (true) {
      const t = this.peek();
      if (
        !t ||
        (t.kind !== "eq" &&
          t.kind !== "neq" &&
          t.kind !== "lt" &&
          t.kind !== "lte" &&
          t.kind !== "gt" &&
          t.kind !== "gte")
      ) {
        break;
      }
      this.pos++;
      const op: BinaryOp =
        t.kind === "eq"
          ? "="
          : t.kind === "neq"
            ? "<>"
            : t.kind === "lt"
              ? "<"
              : t.kind === "lte"
                ? "<="
                : t.kind === "gt"
                  ? ">"
                  : ">=";
      const right = this.parseConcat();
      left = { kind: "binary", op, left, right };
    }
    return left;
  }

  /** & string concatenation. Left-assoc. */
  parseConcat(): Node {
    let left = this.parseAdditive();
    while (true) {
      const t = this.peek();
      if (!t || t.kind !== "amp") break;
      this.pos++;
      const right = this.parseAdditive();
      left = { kind: "binary", op: "&", left, right };
    }
    return left;
  }

  parseAdditive(): Node {
    let left = this.parseMultiplicative();
    while (true) {
      const t = this.peek();
      if (!t || (t.kind !== "plus" && t.kind !== "minus")) break;
      this.pos++;
      const right = this.parseMultiplicative();
      left = { kind: "binary", op: t.kind === "plus" ? "+" : "-", left, right };
    }
    return left;
  }

  parseMultiplicative(): Node {
    let left = this.parsePower();
    while (true) {
      const t = this.peek();
      if (!t || (t.kind !== "star" && t.kind !== "slash")) break;
      this.pos++;
      const right = this.parsePower();
      left = { kind: "binary", op: t.kind === "star" ? "*" : "/", left, right };
    }
    return left;
  }

  /** Power ^ — right-associative. */
  parsePower(): Node {
    const left = this.parseUnary();
    const t = this.peek();
    if (t && t.kind === "caret") {
      this.pos++;
      const right = this.parsePower(); // right-assoc
      return { kind: "binary", op: "^", left, right };
    }
    return left;
  }

  parseUnary(): Node {
    const t = this.peek();
    if (t && t.kind === "minus") {
      this.pos++;
      return { kind: "unary", op: "-", arg: this.parseUnary() };
    }
    if (t && t.kind === "plus") {
      this.pos++;
      return this.parseUnary();
    }
    return this.parsePostfix();
  }

  /** Postfix % — turns N into N/100. */
  parsePostfix(): Node {
    const inner = this.parsePrimary();
    const t = this.peek();
    if (t && t.kind === "percent") {
      this.pos++;
      return { kind: "postfix", op: "%", arg: inner };
    }
    return inner;
  }

  parsePrimary(): Node {
    const t = this.peek();
    if (!t) throw new FormulaParseError("unexpected end of input");
    if (t.kind === "number") {
      this.pos++;
      return { kind: "number", value: Number(t.text) };
    }
    if (t.kind === "string") {
      this.pos++;
      return { kind: "string", value: t.text };
    }
    if (t.kind === "ref") {
      this.pos++;
      // Range A1:A4 — only legal in function args, but cheapest to handle here.
      const next = this.peek();
      if (next && next.kind === "colon") {
        this.pos++;
        const to = this.consume("ref");
        return { kind: "range", from: t.text, to: to.text };
      }
      return { kind: "ref", ref: t.text };
    }
    if (t.kind === "ident") {
      // Identifier disambiguation (FORMULAS_SPEC §"Identifier disambiguation"):
      //
      //   ident '('         → function call (uppercased name)
      //   reserved name     → MUST be a function. Bare `=MEDIAN` (no parens)
      //                       is `#NAME!`; reserved names never shadow as
      //                       field refs, even if the bundle has a field
      //                       literally named "median".
      //   ident '[' expr ']' → row-indexed field ref (`temperature[3]`)
      //   ident             → whole-column field ref (acts like a range
      //                       inside aggregates; collapses to first cell
      //                       elsewhere). Unknown field → `#NAME!` at eval.
      this.pos++;
      const upper = t.text.toUpperCase();
      const isReserved = RESERVED_NAMES.has(upper);
      const next = this.peek();
      if (next && next.kind === "lparen") {
        this.consume("lparen");
        const args: Node[] = [];
        const peeked = this.peek();
        if (peeked && peeked.kind !== "rparen") {
          args.push(this.parseExpression());
          while (this.peek()?.kind === "comma") {
            this.pos++;
            args.push(this.parseExpression());
          }
        }
        this.consume("rparen");
        return { kind: "call", name: upper, args };
      }
      if (isReserved) {
        throw new FormulaNameError(
          `reserved function name '${t.text}' used as value`,
        );
      }
      if (next && next.kind === "lbracket") {
        this.pos++;
        const first = this.parseExpression();
        // `temperature[1:5]` — sliced field range. Both bounds required;
        // open-ended slices (`[3:]` / `[:5]`) are explicitly out of v1.
        const sep = this.peek();
        if (sep && sep.kind === "colon") {
          this.pos++;
          const second = this.parseExpression();
          this.consume("rbracket");
          return { kind: "fieldRangeRef", field: t.text, fromArg: first, toArg: second };
        }
        this.consume("rbracket");
        return { kind: "fieldRowRef", field: t.text, rowArg: first };
      }
      return { kind: "fieldRef", field: t.text };
    }
    if (t.kind === "lparen") {
      this.pos++;
      const e = this.parseExpression();
      this.consume("rparen");
      return e;
    }
    throw new FormulaParseError(`unexpected token: ${t.kind}`);
  }
}

// ── evaluator ──────────────────────────────────────────────────────────

function evalNode(n: Node, ctx: FormulaContext): FormulaResult {
  if (n.kind === "number") return ok(n.value);
  if (n.kind === "string") {
    // A literal string sentinel is just a string, not an error — only
    // *cell-resolved* sentinels poison aggregates. So `=SUM("#REF!", 1)`
    // would still try to coerce and yield 1 (toNumber("#REF!") = 0).
    return ok(n.value);
  }
  if (n.kind === "ref") {
    const v = ctx.cell(n.ref);
    const e = asError(v);
    if (e) return err(e);
    return ok(v ?? 0);
  }
  if (n.kind === "range") {
    // Ranges only make sense inside aggregates. If one slips out here,
    // collapse to the first cell's value.
    const v = ctx.cell(n.from);
    const e = asError(v);
    if (e) return err(e);
    return ok(v ?? 0);
  }
  if (n.kind === "fieldRef") {
    // Bare field ref outside an aggregate. Symmetric with `range` above:
    // collapse to the first cell. Unknown field → `#NAME!`.
    if (!ctx.resolveField) return err("#NAME!");
    const refs = ctx.resolveField(n.field);
    if (!refs) return err("#NAME!");
    if (refs.length === 0) return err("#REF!");
    const v = ctx.cell(refs[0]);
    const e = asError(v);
    if (e) return err(e);
    return ok(v ?? 0);
  }
  if (n.kind === "fieldRangeRef") {
    // Outside of an aggregate, collapse to the first cell — symmetric
    // with how `range` and `fieldRef` behave when used as a scalar.
    const resolved = resolveFieldRange(n, ctx);
    if (resolved.error) return err(resolved.error);
    if (resolved.refs.length === 0) return ok(null);
    const v = ctx.cell(resolved.refs[0]);
    const e = asError(v);
    if (e) return err(e);
    return ok(v ?? 0);
  }
  if (n.kind === "fieldRowRef") {
    if (!ctx.resolveField) return err("#NAME!");
    if (ctx.resolveField(n.field) == null) return err("#NAME!");
    const rr = evalNode(n.rowArg, ctx);
    if (rr.error) return rr;
    const row = Math.trunc(toNumber(rr.value));
    if (!Number.isFinite(row)) return err("#REF!");
    if (!ctx.fieldRowRef) return err("#REF!");
    const ref = ctx.fieldRowRef(n.field, row);
    if (!ref) return err("#REF!");
    const v = ctx.cell(ref);
    const e = asError(v);
    if (e) return err(e);
    return ok(v ?? 0);
  }
  if (n.kind === "unary") {
    const r = evalNode(n.arg, ctx);
    if (r.error) return r;
    return ok(-toNumber(r.value));
  }
  if (n.kind === "postfix") {
    // Postfix % — divide by 100. The only postfix op for now.
    const r = evalNode(n.arg, ctx);
    if (r.error) return r;
    return ok(toNumber(r.value) / 100);
  }
  if (n.kind === "binary") {
    const lr = evalNode(n.left, ctx);
    if (lr.error) return lr;
    const rr = evalNode(n.right, ctx);
    if (rr.error) return rr;
    // String concat: coerce both sides to string (no number coercion).
    if (n.op === "&") {
      const ls = lr.value == null ? "" : String(lr.value);
      const rs = rr.value == null ? "" : String(rr.value);
      return ok(ls + rs);
    }
    // Equality / inequality: string-aware (case-insensitive for strings).
    if (n.op === "=" || n.op === "<>") {
      const eq = compareEqual(lr.value, rr.value);
      return ok(n.op === "=" ? eq : !eq);
    }
    // Ordering comparisons: numeric if both coerce; otherwise locale-string.
    if (n.op === "<" || n.op === "<=" || n.op === ">" || n.op === ">=") {
      const c = compareOrder(lr.value, rr.value);
      switch (n.op) {
        case "<":  return ok(c < 0);
        case "<=": return ok(c <= 0);
        case ">":  return ok(c > 0);
        case ">=": return ok(c >= 0);
      }
    }
    const l = toNumber(lr.value);
    const r = toNumber(rr.value);
    switch (n.op) {
      case "+": return ok(l + r);
      case "-": return ok(l - r);
      case "*": return ok(l * r);
      case "/":
        if (r === 0) return err("#DIV0!");
        return ok(l / r);
      case "^": return ok(Math.pow(l, r));
    }
  }
  if (n.kind === "call") {
    return evalCall(n, ctx);
  }
  return err("#ERROR!");
}

function evalCall(n: { name: string; args: Node[] }, ctx: FormulaContext): FormulaResult {
  const name = n.name;
  // Helper: expand ranges + collect numeric arg values. Cell-resolved
  // error sentinels (e.g. `"#REF!"` literally stored in the bundle row)
  // **poison** the aggregate — return the first error encountered, in
  // argument-order. Matches Excel semantics.
  const numericArgs = (): { values: number[]; error: FormulaError | null } => {
    const values: number[] = [];
    for (const a of n.args) {
      if (a.kind === "range") {
        for (const ref of expandRange(a.from, a.to)) {
          const v = ctx.cell(ref);
          const e = asError(v);
          if (e) return { values: [], error: e };
          if (typeof v === "number") values.push(v);
        }
      } else if (a.kind === "fieldRef") {
        if (!ctx.resolveField) return { values: [], error: "#NAME!" };
        const refs = ctx.resolveField(a.field);
        if (!refs) return { values: [], error: "#NAME!" };
        for (const ref of refs) {
          const v = ctx.cell(ref);
          const e = asError(v);
          if (e) return { values: [], error: e };
          if (typeof v === "number") values.push(v);
        }
      } else if (a.kind === "fieldRangeRef") {
        const resolved = resolveFieldRange(a, ctx);
        if (resolved.error) return { values: [], error: resolved.error };
        for (const ref of resolved.refs) {
          const v = ctx.cell(ref);
          const e = asError(v);
          if (e) return { values: [], error: e };
          if (typeof v === "number") values.push(v);
        }
      } else {
        const r = evalNode(a, ctx);
        if (r.error) return { values: [], error: r.error };
        if (typeof r.value === "number") values.push(r.value);
      }
    }
    return { values, error: null };
  };

  switch (name) {
    case "SUM": {
      const r = numericArgs();
      if (r.error) return err(r.error);
      return ok(r.values.reduce((s, x) => s + x, 0));
    }
    case "AVG":
    case "AVERAGE": {
      const r = numericArgs();
      if (r.error) return err(r.error);
      if (r.values.length === 0) return err("#DIV0!");
      return ok(r.values.reduce((s, x) => s + x, 0) / r.values.length);
    }
    case "MIN": {
      const r = numericArgs();
      if (r.error) return err(r.error);
      if (r.values.length === 0) return ok(0);
      return ok(Math.min(...r.values));
    }
    case "MAX": {
      const r = numericArgs();
      if (r.error) return err(r.error);
      if (r.values.length === 0) return ok(0);
      return ok(Math.max(...r.values));
    }
    case "COUNT": {
      let count = 0;
      for (const a of n.args) {
        if (a.kind === "range") {
          for (const ref of expandRange(a.from, a.to)) {
            const v = ctx.cell(ref);
            const e = asError(v);
            if (e) return err(e);
            if (typeof v === "number") count++;
          }
        } else if (a.kind === "fieldRef") {
          if (!ctx.resolveField) return err("#NAME!");
          const refs = ctx.resolveField(a.field);
          if (!refs) return err("#NAME!");
          for (const ref of refs) {
            const v = ctx.cell(ref);
            const e = asError(v);
            if (e) return err(e);
            if (typeof v === "number") count++;
          }
        } else if (a.kind === "fieldRangeRef") {
          const resolved = resolveFieldRange(a, ctx);
          if (resolved.error) return err(resolved.error);
          for (const ref of resolved.refs) {
            const v = ctx.cell(ref);
            const e = asError(v);
            if (e) return err(e);
            if (typeof v === "number") count++;
          }
        } else {
          const r = evalNode(a, ctx);
          if (r.error) return r;
          if (typeof r.value === "number") count++;
        }
      }
      return ok(count);
    }
    case "COUNTA": {
      // Counts non-empty cells. Bare refs are checked against the raw
      // cell value (bypassing the evaluator's null→0 coercion) so an
      // empty cell isn't double-counted as a numeric zero. Error
      // sentinels poison the count (Excel parity).
      let count = 0;
      for (const a of n.args) {
        if (a.kind === "range") {
          for (const ref of expandRange(a.from, a.to)) {
            const v = ctx.cell(ref);
            const e = asError(v);
            if (e) return err(e);
            if (v != null && v !== "") count++;
          }
        } else if (a.kind === "fieldRef") {
          if (!ctx.resolveField) return err("#NAME!");
          const refs = ctx.resolveField(a.field);
          if (!refs) return err("#NAME!");
          for (const ref of refs) {
            const v = ctx.cell(ref);
            const e = asError(v);
            if (e) return err(e);
            if (v != null && v !== "") count++;
          }
        } else if (a.kind === "fieldRangeRef") {
          const resolved = resolveFieldRange(a, ctx);
          if (resolved.error) return err(resolved.error);
          for (const ref of resolved.refs) {
            const v = ctx.cell(ref);
            const e = asError(v);
            if (e) return err(e);
            if (v != null && v !== "") count++;
          }
        } else if (a.kind === "ref") {
          const v = ctx.cell(a.ref);
          const e = asError(v);
          if (e) return err(e);
          if (v != null && v !== "") count++;
        } else {
          const r = evalNode(a, ctx);
          if (r.error) return r;
          if (r.value != null && r.value !== "") count++;
        }
      }
      return ok(count);
    }
    case "ABS": {
      if (n.args.length !== 1) return err("#ERROR!");
      const r = evalNode(n.args[0], ctx);
      if (r.error) return r;
      return ok(Math.abs(toNumber(r.value)));
    }
    case "MOD": {
      if (n.args.length !== 2) return err("#ERROR!");
      const ar = evalNode(n.args[0], ctx);
      if (ar.error) return ar;
      const br = evalNode(n.args[1], ctx);
      if (br.error) return br;
      const a = toNumber(ar.value);
      const b = toNumber(br.value);
      if (b === 0) return err("#DIV0!");
      // Excel MOD: sign of result matches sign of divisor. JS `%` has
      // sign-of-dividend semantics, so we adjust: ((a % b) + b) % b.
      return ok(((a % b) + b) % b);
    }
    case "ROUND": {
      if (n.args.length < 1 || n.args.length > 2) return err("#ERROR!");
      const valR = evalNode(n.args[0], ctx);
      if (valR.error) return valR;
      let digits = 0;
      if (n.args.length === 2) {
        const dR = evalNode(n.args[1], ctx);
        if (dR.error) return dR;
        digits = Math.trunc(toNumber(dR.value));
      }
      return ok(roundHalfAwayFromZero(toNumber(valR.value), digits));
    }
    case "CONCAT": {
      // Bare refs check the raw cell so a null cell concatenates as
      // empty, not "0" (which would be the evaluator's null→0 default).
      let s = "";
      for (const a of n.args) {
        if (a.kind === "ref") {
          const v = ctx.cell(a.ref);
          if (v == null || v === "") continue;
          s += String(v);
          continue;
        }
        if (a.kind === "range") {
          for (const ref of expandRange(a.from, a.to)) {
            const v = ctx.cell(ref);
            const e = asError(v);
            if (e) return err(e);
            if (v == null || v === "") continue;
            s += String(v);
          }
          continue;
        }
        if (a.kind === "fieldRef") {
          if (!ctx.resolveField) return err("#NAME!");
          const refs = ctx.resolveField(a.field);
          if (!refs) return err("#NAME!");
          for (const ref of refs) {
            const v = ctx.cell(ref);
            const e = asError(v);
            if (e) return err(e);
            if (v == null || v === "") continue;
            s += String(v);
          }
          continue;
        }
        if (a.kind === "fieldRangeRef") {
          const resolved = resolveFieldRange(a, ctx);
          if (resolved.error) return err(resolved.error);
          for (const ref of resolved.refs) {
            const v = ctx.cell(ref);
            const e = asError(v);
            if (e) return err(e);
            if (v == null || v === "") continue;
            s += String(v);
          }
          continue;
        }
        const r = evalNode(a, ctx);
        if (r.error) return r;
        if (r.value == null) continue;
        s += String(r.value);
      }
      return ok(s);
    }
    case "MEDIAN": {
      const r = numericArgs();
      if (r.error) return err(r.error);
      if (r.values.length === 0) return err("#DIV0!");
      const sorted = r.values.slice().sort((a, b) => a - b);
      const m = sorted.length;
      return ok(
        m % 2 === 1
          ? sorted[(m - 1) / 2]
          : (sorted[m / 2 - 1] + sorted[m / 2]) / 2,
      );
    }
    case "STDEV": {
      const r = numericArgs();
      if (r.error) return err(r.error);
      if (r.values.length < 2) return err("#DIV0!");
      return ok(Math.sqrt(variance(r.values, true)));
    }
    case "STDEVP": {
      const r = numericArgs();
      if (r.error) return err(r.error);
      if (r.values.length === 0) return err("#DIV0!");
      return ok(Math.sqrt(variance(r.values, false)));
    }
    case "VAR": {
      const r = numericArgs();
      if (r.error) return err(r.error);
      if (r.values.length < 2) return err("#DIV0!");
      return ok(variance(r.values, true));
    }
    case "VARP": {
      const r = numericArgs();
      if (r.error) return err(r.error);
      if (r.values.length === 0) return err("#DIV0!");
      return ok(variance(r.values, false));
    }
    case "PERCENTILE": {
      // PERCENTILE(range, k) — Excel PERCENTILE.INC convention.
      // Linear interpolation between bracketing samples.
      if (n.args.length !== 2) return err("#ERROR!");
      const collected = collectRangeValues(n.args[0], ctx);
      if (collected.error) return err(collected.error);
      if (collected.values.length === 0) return err("#DIV0!");
      const kR = evalNode(n.args[1], ctx);
      if (kR.error) return kR;
      const k = toNumber(kR.value);
      if (k < 0 || k > 1) return err("#VALUE!");
      return ok(percentileInc(collected.values, k));
    }
    case "QUARTILE": {
      if (n.args.length !== 2) return err("#ERROR!");
      const qR = evalNode(n.args[1], ctx);
      if (qR.error) return qR;
      const q = Math.trunc(toNumber(qR.value));
      if (q < 0 || q > 4) return err("#VALUE!");
      const collected = collectRangeValues(n.args[0], ctx);
      if (collected.error) return err(collected.error);
      if (collected.values.length === 0) return err("#DIV0!");
      return ok(percentileInc(collected.values, q / 4));
    }
    case "SUMIF":
    case "COUNTIF":
    case "AVERAGEIF": {
      // SUMIF(range, predicate [, sum_range])
      // COUNTIF(range, predicate) — sum_range is meaningless
      // AVERAGEIF(range, predicate [, avg_range])
      if (n.args.length < 2 || n.args.length > 3) return err("#ERROR!");
      const crit = collectRangeCells(n.args[0], ctx);
      if (crit.error) return err(crit.error);
      // The predicate operand is an evaluated scalar (a string literal,
      // a number, or anything else that reduces to a value).
      const predR = evalNode(n.args[1], ctx);
      if (predR.error) return predR;
      const predicate = parsePredicate(predR.value);
      // Sum/avg range — defaults to the criteria range itself.
      const target =
        name === "COUNTIF" || n.args.length < 3
          ? crit
          : collectRangeCells(n.args[2], ctx);
      if (target.error) return err(target.error);
      if (target.values.length !== crit.values.length) return err("#VALUE!");
      let sum = 0;
      let count = 0;
      for (let i = 0; i < crit.values.length; i++) {
        if (!matchesPredicate(crit.values[i], predicate)) continue;
        if (name === "COUNTIF") {
          count++;
          continue;
        }
        const v = target.values[i];
        if (typeof v === "number") {
          sum += v;
          count++;
        }
      }
      if (name === "COUNTIF") return ok(count);
      if (name === "AVERAGEIF") {
        if (count === 0) return err("#DIV0!");
        return ok(sum / count);
      }
      return ok(sum); // SUMIF
    }
    case "SUMIFS":
    case "COUNTIFS":
    case "AVERAGEIFS":
    case "MINIFS":
    case "MAXIFS": {
      // SUMIFS(sum_range, range1, pred1, range2, pred2, …)
      // COUNTIFS(range1, pred1, range2, pred2, …) — no sum_range
      // AVERAGEIFS / MINIFS / MAXIFS mirror SUMIFS.
      const hasTarget = name !== "COUNTIFS";
      const firstPair = hasTarget ? 1 : 0;
      const argc = n.args.length;
      if (argc < firstPair + 2 || (argc - firstPair) % 2 !== 0) return err("#ERROR!");
      // Resolve target (sum/avg/min/max range) first, if present.
      let target: { values: FormulaValue[]; error: FormulaError | null } | null = null;
      if (hasTarget) {
        target = collectRangeCells(n.args[0], ctx);
        if (target.error) return err(target.error);
      }
      // Resolve each (range_i, pred_i) pair.
      const ranges: FormulaValue[][] = [];
      const preds: ReturnType<typeof parsePredicate>[] = [];
      for (let i = firstPair; i < argc; i += 2) {
        const range = collectRangeCells(n.args[i], ctx);
        if (range.error) return err(range.error);
        const predR = evalNode(n.args[i + 1], ctx);
        if (predR.error) return predR;
        ranges.push(range.values);
        preds.push(parsePredicate(predR.value));
      }
      // All criteria ranges (and the sum_range) must have the same length.
      const len = ranges[0].length;
      for (const r of ranges) if (r.length !== len) return err("#VALUE!");
      if (target && target.values.length !== len) return err("#VALUE!");
      // Walk the rows; keep those matching every predicate.
      let sum = 0;
      let count = 0;
      let minV = Number.POSITIVE_INFINITY;
      let maxV = Number.NEGATIVE_INFINITY;
      for (let i = 0; i < len; i++) {
        let matches = true;
        for (let p = 0; p < preds.length; p++) {
          if (!matchesPredicate(ranges[p][i], preds[p])) { matches = false; break; }
        }
        if (!matches) continue;
        if (name === "COUNTIFS") { count++; continue; }
        const v = target!.values[i];
        if (typeof v !== "number") continue;
        sum += v;
        count++;
        if (v < minV) minV = v;
        if (v > maxV) maxV = v;
      }
      switch (name) {
        case "COUNTIFS": return ok(count);
        case "SUMIFS":   return ok(sum);
        case "AVERAGEIFS":
          if (count === 0) return err("#DIV0!");
          return ok(sum / count);
        case "MINIFS":   return ok(count === 0 ? 0 : minV);
        case "MAXIFS":   return ok(count === 0 ? 0 : maxV);
      }
      return err("#ERROR!");
    }
    case "LEN": {
      // LEN(value) → number of UTF-16 code units in the string form.
      // Null/empty → 0; numbers stringify before measuring.
      if (n.args.length !== 1) return err("#ERROR!");
      const a = n.args[0];
      let v: FormulaValue;
      if (a.kind === "ref") {
        v = ctx.cell(a.ref);
        const e = asError(v);
        if (e) return err(e);
      } else {
        const r = evalNode(a, ctx);
        if (r.error) return r;
        v = r.value;
      }
      if (v == null) return ok(0);
      return ok(String(v).length);
    }
    case "LOWER":
    case "UPPER":
    case "TRIM": {
      if (n.args.length !== 1) return err("#ERROR!");
      const a = n.args[0];
      let v: FormulaValue;
      if (a.kind === "ref") {
        v = ctx.cell(a.ref);
        const e = asError(v);
        if (e) return err(e);
      } else {
        const r = evalNode(a, ctx);
        if (r.error) return r;
        v = r.value;
      }
      const s = v == null ? "" : String(v);
      if (name === "LOWER") return ok(s.toLowerCase());
      if (name === "UPPER") return ok(s.toUpperCase());
      // TRIM (Excel): trim leading/trailing whitespace AND collapse any
      // internal whitespace run to a single space. Treat tabs/newlines
      // as whitespace, matching JS \s semantics.
      return ok(s.replace(/^\s+|\s+$/g, "").replace(/\s+/g, " "));
    }
    case "TO_DATE": {
      if (n.args.length !== 1) return err("#ERROR!");
      const r = evalNode(n.args[0], ctx);
      if (r.error) return r;
      const d = toSerialDay(r.value);
      if (d == null) return err("#VALUE!");
      return ok(d);
    }
    case "TODAY": {
      if (n.args.length !== 0) return err("#ERROR!");
      const t = ctx.today ? ctx.today() : Math.floor(Date.now() / MS_PER_DAY);
      return ok(t);
    }
    case "YEAR":
    case "MONTH":
    case "DAY": {
      if (n.args.length !== 1) return err("#ERROR!");
      const r = evalNode(n.args[0], ctx);
      if (r.error) return r;
      // Strings get parsed through TO_DATE; numbers are already serial days.
      const serial = typeof r.value === "number" ? r.value : toSerialDay(r.value);
      if (serial == null) return err("#VALUE!");
      const date = new Date(serial * MS_PER_DAY);
      if (name === "YEAR") return ok(date.getUTCFullYear());
      if (name === "MONTH") return ok(date.getUTCMonth() + 1);
      return ok(date.getUTCDate()); // DAY
    }
    case "DATEDIF": {
      if (n.args.length !== 3) return err("#ERROR!");
      const aR = evalNode(n.args[0], ctx);
      if (aR.error) return aR;
      const bR = evalNode(n.args[1], ctx);
      if (bR.error) return bR;
      const uR = evalNode(n.args[2], ctx);
      if (uR.error) return uR;
      const start = typeof aR.value === "number" ? aR.value : toSerialDay(aR.value);
      const end = typeof bR.value === "number" ? bR.value : toSerialDay(bR.value);
      if (start == null || end == null) return err("#VALUE!");
      const unit = typeof uR.value === "string" ? uR.value : null;
      if (unit !== "d" && unit !== "w" && unit !== "m" && unit !== "y") {
        // Uppercase units (Excel's "Y", "M", "D", "MD", …) deliberately
        // rejected — see FORMULAS_SPEC §"DATEDIF unit divergence".
        return err("#VALUE!");
      }
      return ok(datediff(start, end, unit));
    }
    case "IF": {
      if (n.args.length < 2 || n.args.length > 3) return err("#ERROR!");
      const cond = evalNode(n.args[0], ctx);
      if (cond.error) return cond;
      const truthy = !!toNumber(cond.value);
      if (truthy) return evalNode(n.args[1], ctx);
      if (n.args.length === 3) return evalNode(n.args[2], ctx);
      return ok(null);
    }
    case "SAME": {
      if (n.args.length !== 2) return err("#ERROR!");
      const a = refOf(n.args[0], ctx);
      const b = refOf(n.args[1], ctx);
      if (!a || !b) return err("#REF!");
      return ok(ctx.sameness(a, b));
    }
    case "DIST": {
      if (n.args.length !== 2) return err("#ERROR!");
      const a = refOf(n.args[0], ctx);
      const b = refOf(n.args[1], ctx);
      if (!a || !b) return err("#REF!");
      // Use the Davis double-cover identity rather than a separate
      // distance lookup so SAME + DIST² = 1 holds exactly (modulo float
      // precision). With S = cos²(θ/2), d = sin(θ/2), the identity is
      // the half-angle Pythagorean identity.
      const S = ctx.sameness(a, b);
      const sq = 1 - S;
      return ok(sq <= 0 ? 0 : Math.sqrt(sq));
    }
    case "K": {
      if (n.args.length !== 1) return err("#ERROR!");
      const r = refOf(n.args[0], ctx);
      if (!r) return err("#REF!");
      return ok(ctx.kappa(r));
    }
    case "COHORT": {
      // Phase 3 fix: COHORT takes a cell ref (not a column name string)
      // and returns the row's cover-field value. Backwards-compat for
      // the Phase 1 stub: if the arg is a string literal, fall through
      // to ctx.cohort(string) which the old App.tsx echoed back.
      if (n.args.length !== 1) return err("#ERROR!");
      const a = n.args[0];
      if (a.kind === "ref") return ok(ctx.cohort(a.ref));
      const r = evalNode(a, ctx);
      if (r.error) return r;
      if (typeof r.value !== "string") return err("#ERROR!");
      return ok(ctx.cohort(r.value));
    }
    case "KAPPA_RANK": {
      if (n.args.length !== 1) return err("#ERROR!");
      const r = refOf(n.args[0], ctx);
      if (!r) return err("#REF!");
      if (!ctx.kappaRank) return err("#REF!");
      const rank = ctx.kappaRank(r);
      if (rank == null) return err("#REF!");
      return ok(rank);
    }
    case "SAMENESS_RANK": {
      if (n.args.length !== 2) return err("#ERROR!");
      const pivot = refOf(n.args[0], ctx);
      const ref = refOf(n.args[1], ctx);
      if (!pivot || !ref) return err("#REF!");
      if (!ctx.samenessRank) return err("#REF!");
      const rank = ctx.samenessRank(pivot, ref);
      if (rank == null) return err("#REF!");
      return ok(rank);
    }
    default:
      return err("#NAME!");
  }
}

/**
 * Resolve a node to a single A1 cell ref, if it represents one. Handles:
 *   - bare `ref` nodes (`A1`)
 *   - `fieldRowRef` with a literal-number index (`temperature[3]`) —
 *     looked up via `ctx.fieldRowRef`
 *
 * Anything else (range, expression, dynamic index) returns null →
 * caller emits `#REF!`. Used by SAME/DIST/K/COHORT/KAPPA_RANK/
 * SAMENESS_RANK which all need to point at a *specific* row, not a
 * range.
 */
function refOf(n: Node, ctx: FormulaContext): string | null {
  if (n.kind === "ref") return n.ref;
  if (n.kind === "fieldRowRef" && n.rowArg.kind === "number" && ctx.fieldRowRef) {
    return ctx.fieldRowRef(n.field, Math.trunc(n.rowArg.value));
  }
  return null;
}

function toNumber(v: FormulaValue): number {
  if (typeof v === "number") return v;
  if (typeof v === "boolean") return v ? 1 : 0;
  if (typeof v === "string") {
    const n = Number(v);
    return Number.isFinite(n) ? n : 0;
  }
  return 0;
}

/**
 * Recognize cell values that are error sentinels (produced by an
 * upstream formula whose result was written back to the bundle).
 * Returns the matching `FormulaError` or null.
 *
 * Aggregates and operators use this to **poison** on error inputs,
 * matching Excel's behavior — `=SUM(A1:A10)` returns `#REF!` if any
 * cell in the range holds the sentinel.
 */
export function asError(v: unknown): FormulaError | null {
  if (typeof v !== "string") return null;
  switch (v) {
    case "#ERROR!":
    case "#NAME!":
    case "#REF!":
    case "#DIV0!":
    case "#VALUE!":
    case "#CIRC!":
      return v;
    default:
      return null;
  }
}

/**
 * Equality with Excel-ish semantics: numbers compare numerically;
 * strings compare case-insensitively; null is equal to null and "".
 */
function compareEqual(a: FormulaValue, b: FormulaValue): boolean {
  // Both nullish → equal.
  if (a == null && b == null) return true;
  if (a == null || b == null) {
    // null equals empty string per Excel convention.
    return a === "" || b === "";
  }
  if (typeof a === "string" || typeof b === "string") {
    return String(a).toLowerCase() === String(b).toLowerCase();
  }
  return toNumber(a) === toNumber(b);
}

/**
 * Ordering: numeric if both coerce sensibly, else locale-aware string
 * comparison. Returns negative, zero, or positive (compare convention).
 */
function compareOrder(a: FormulaValue, b: FormulaValue): number {
  // Prefer numeric ordering when both sides look numeric (incl. numeric
  // booleans).
  if (
    (typeof a === "number" || typeof a === "boolean") &&
    (typeof b === "number" || typeof b === "boolean")
  ) {
    return toNumber(a) - toNumber(b);
  }
  // Otherwise lexicographic.
  return String(a ?? "").localeCompare(String(b ?? ""), undefined, {
    sensitivity: "base",
  });
}

function expandRange(from: string, to: string): string[] {
  const fromParsed = parseRef(from);
  const toParsed = parseRef(to);
  if (!fromParsed || !toParsed) return [];
  // Only support same-column or same-row ranges in v1.
  if (fromParsed.col !== toParsed.col && fromParsed.row !== toParsed.row) {
    return [];
  }
  const out: string[] = [];
  if (fromParsed.col === toParsed.col) {
    const r1 = Math.min(fromParsed.row, toParsed.row);
    const r2 = Math.max(fromParsed.row, toParsed.row);
    for (let r = r1; r <= r2; r++) out.push(`${fromParsed.col}${r}`);
  } else {
    // Same row, different columns — single-row range.
    // For v1 we only emit the endpoint cells; a future revision can
    // expand across alphabetical columns.
    out.push(from, to);
  }
  return out;
}

function parseRef(ref: string): { col: string; row: number } | null {
  const m = ref.match(/^([A-Z]+)([0-9]+)$/);
  if (!m) return null;
  return { col: m[1], row: Number(m[2]) };
}

/**
 * Collect numeric values from a node that's expected to be a range
 * (but can also be a single value). Poisons on any cell-resolved
 * error sentinel encountered along the way.
 */
/**
 * Resolve a `fieldRangeRef` (e.g. `temperature[1:5]`) into the concrete
 * list of A1 refs it covers. Bounds are 1-based and inclusive; the
 * upper bound clamps to the available row count (`temperature[1:99]`
 * over a 7-row column returns 7 refs, not `#REF!`). A lower bound below
 * 1 still errors — zero is treated as "you meant something else."
 */
function resolveFieldRange(
  n: { field: string; fromArg: Node; toArg: Node },
  ctx: FormulaContext,
): { refs: string[]; error: FormulaError | null } {
  if (!ctx.resolveField) return { refs: [], error: "#NAME!" };
  const all = ctx.resolveField(n.field);
  if (!all) return { refs: [], error: "#NAME!" };
  const fromR = evalNode(n.fromArg, ctx);
  if (fromR.error) return { refs: [], error: fromR.error };
  const toR = evalNode(n.toArg, ctx);
  if (toR.error) return { refs: [], error: toR.error };
  const fromI = Math.trunc(toNumber(fromR.value));
  const toI = Math.trunc(toNumber(toR.value));
  if (!Number.isFinite(fromI) || !Number.isFinite(toI)) return { refs: [], error: "#REF!" };
  if (fromI < 1 || toI < 1) return { refs: [], error: "#REF!" };
  // Empty range on inverted bounds returns []; caller decides what an
  // empty aggregate means (SUM → 0, AVG → #DIV0!, etc.).
  if (toI < fromI) return { refs: [], error: null };
  const hi = Math.min(toI, all.length);
  const out: string[] = [];
  for (let r = fromI; r <= hi; r++) out.push(all[r - 1]);
  return { refs: out, error: null };
}

/**
 * Resolve a node that's expected to be a range (or fieldRef) into a
 * list of raw cell values — strings, numbers, nulls all preserved.
 * Used by the *IF family which needs to apply the predicate against
 * the original value (a category string, a number, etc.), not a
 * numeric-coerced view.
 *
 * Poisons on any error sentinel encountered in the range.
 */
function collectRangeCells(
  node: Node,
  ctx: FormulaContext,
): { values: FormulaValue[]; error: FormulaError | null } {
  const values: FormulaValue[] = [];
  if (node.kind === "range") {
    for (const ref of expandRange(node.from, node.to)) {
      const v = ctx.cell(ref);
      const e = asError(v);
      if (e) return { values: [], error: e };
      values.push(v ?? null);
    }
    return { values, error: null };
  }
  if (node.kind === "fieldRef") {
    if (!ctx.resolveField) return { values: [], error: "#NAME!" };
    const refs = ctx.resolveField(node.field);
    if (!refs) return { values: [], error: "#NAME!" };
    for (const ref of refs) {
      const v = ctx.cell(ref);
      const e = asError(v);
      if (e) return { values: [], error: e };
      values.push(v ?? null);
    }
    return { values, error: null };
  }
  if (node.kind === "fieldRangeRef") {
    const resolved = resolveFieldRange(node, ctx);
    if (resolved.error) return { values: [], error: resolved.error };
    for (const ref of resolved.refs) {
      const v = ctx.cell(ref);
      const e = asError(v);
      if (e) return { values: [], error: e };
      values.push(v ?? null);
    }
    return { values, error: null };
  }
  // A non-range arg gets treated as a single-cell range.
  const r = evalNode(node, ctx);
  if (r.error) return { values: [], error: r.error };
  values.push(r.value);
  return { values, error: null };
}

function collectRangeValues(
  node: Node,
  ctx: FormulaContext,
): { values: number[]; error: FormulaError | null } {
  const values: number[] = [];
  if (node.kind === "range") {
    for (const ref of expandRange(node.from, node.to)) {
      const v = ctx.cell(ref);
      const e = asError(v);
      if (e) return { values: [], error: e };
      if (typeof v === "number") values.push(v);
    }
  } else if (node.kind === "fieldRef") {
    if (!ctx.resolveField) return { values: [], error: "#NAME!" };
    const refs = ctx.resolveField(node.field);
    if (!refs) return { values: [], error: "#NAME!" };
    for (const ref of refs) {
      const v = ctx.cell(ref);
      const e = asError(v);
      if (e) return { values: [], error: e };
      if (typeof v === "number") values.push(v);
    }
  } else if (node.kind === "fieldRangeRef") {
    const resolved = resolveFieldRange(node, ctx);
    if (resolved.error) return { values: [], error: resolved.error };
    for (const ref of resolved.refs) {
      const v = ctx.cell(ref);
      const e = asError(v);
      if (e) return { values: [], error: e };
      if (typeof v === "number") values.push(v);
    }
  } else {
    const r = evalNode(node, ctx);
    if (r.error) return { values: [], error: r.error };
    if (typeof r.value === "number") values.push(r.value);
  }
  return { values, error: null };
}

/**
 * Round-half-away-from-zero (Excel parity).
 *
 * JS `Math.round(-0.5)` returns 0; Excel returns -1. `Math.round` uses
 * round-half-toward-positive-infinity. This implementation uses the
 * Excel convention: 0.5 rounds up, -0.5 rounds down, 2.5 rounds to 3,
 * -2.5 rounds to -3.
 */
export function roundHalfAwayFromZero(value: number, digits: number = 0): number {
  if (!Number.isFinite(value)) return value;
  const m = Math.pow(10, digits);
  const x = value * m;
  // Branch on sign so 0.5 and -0.5 both round outward.
  const r = x >= 0 ? Math.floor(x + 0.5) : -Math.floor(-x + 0.5);
  return r / m;
}

/**
 * Sample variance (`sample=true`, n-1 denominator) or population
 * variance (n denominator). Caller guarantees `values.length >= 1` for
 * population and `>= 2` for sample.
 */
export function variance(values: number[], sample: boolean): number {
  const n = values.length;
  if (n === 0) return 0;
  let mean = 0;
  for (const v of values) mean += v;
  mean /= n;
  let sq = 0;
  for (const v of values) sq += (v - mean) * (v - mean);
  const denom = sample ? n - 1 : n;
  return denom > 0 ? sq / denom : 0;
}

/**
 * PERCENTILE.INC (Excel) — linear interpolation between bracketing samples.
 *
 * For k ∈ [0, 1] and a sorted array of length n, position = k·(n−1),
 * which gives PERCENTILE(values, 0) = min, PERCENTILE(values, 1) = max,
 * PERCENTILE(values, 0.5) = median.
 */
export function percentileInc(values: number[], k: number): number {
  if (values.length === 0) return NaN;
  const sorted = values.slice().sort((a, b) => a - b);
  if (k <= 0) return sorted[0];
  if (k >= 1) return sorted[sorted.length - 1];
  const pos = k * (sorted.length - 1);
  const lo = Math.floor(pos);
  const hi = Math.ceil(pos);
  if (lo === hi) return sorted[lo];
  const frac = pos - lo;
  return sorted[lo] * (1 - frac) + sorted[hi] * frac;
}

// ── static dependency extraction ───────────────────────────────────────

/**
 * Subset of `FormulaContext` needed for static dep extraction. Only
 * `resolveField` matters; cell values and Davis primitives don't
 * factor in — we never evaluate the formula here.
 */
export interface DepsContext {
  resolveField?: (name: string) => string[] | null;
}

/**
 * Walk a formula's AST and return the set of A1 refs it reads. This is
 * the dependency edge in the recompute graph (FORMULAS_SPEC §"Recompute
 * model"): when any returned ref's value changes, this formula must
 * re-evaluate.
 *
 * Static resolution:
 *   - `A1`, `A1:A4` → exact set
 *   - `field`, `field[N]`, `field[N:M]` with literal N/M → exact set
 *   - `field[expr]` / `field[expr:expr]` with non-literal indices →
 *     **conservatively** take the whole field column (over-approximate)
 *     plus any deps inside `expr`. This is safe — recompute fires more
 *     often than strictly necessary, never less.
 *
 * `error` mirrors the eval-time error sentinels: unknown field name →
 * `#NAME!`, parse failure → `#ERROR!`. An empty / non-formula input
 * (no leading `=`) returns an empty dep set with no error.
 */
export function collectDeps(
  input: string,
  ctx: DepsContext,
): { deps: Set<string>; error: FormulaError | null } {
  const deps = new Set<string>();
  if (input === "" || !input.startsWith("=")) return { deps, error: null };
  let ast: Node;
  try {
    const tokens = tokenize(input.slice(1));
    const parser = new Parser(tokens);
    ast = parser.parseExpression();
    if (!parser.eof()) return { deps, error: "#ERROR!" };
  } catch (e) {
    if (e instanceof FormulaNameError) return { deps, error: "#NAME!" };
    return { deps, error: "#ERROR!" };
  }
  return walkDeps(ast, ctx, deps);
}

function walkDeps(
  node: Node,
  ctx: DepsContext,
  deps: Set<string>,
): { deps: Set<string>; error: FormulaError | null } {
  switch (node.kind) {
    case "number":
    case "string":
      return { deps, error: null };
    case "ref":
      deps.add(node.ref);
      return { deps, error: null };
    case "range":
      for (const ref of expandRange(node.from, node.to)) deps.add(ref);
      return { deps, error: null };
    case "fieldRef": {
      if (!ctx.resolveField) return { deps, error: "#NAME!" };
      const refs = ctx.resolveField(node.field);
      if (!refs) return { deps, error: "#NAME!" };
      for (const r of refs) deps.add(r);
      return { deps, error: null };
    }
    case "fieldRowRef": {
      if (!ctx.resolveField) return { deps, error: "#NAME!" };
      const refs = ctx.resolveField(node.field);
      if (!refs) return { deps, error: "#NAME!" };
      // Literal row index → exact dep. Otherwise over-approximate to
      // the full column and recurse into the index expression to pick
      // up any cell refs it depends on too.
      if (node.rowArg.kind === "number") {
        const r = Math.trunc(node.rowArg.value);
        if (r >= 1 && r <= refs.length) deps.add(refs[r - 1]);
        return { deps, error: null };
      }
      for (const r of refs) deps.add(r);
      return walkDeps(node.rowArg, ctx, deps);
    }
    case "fieldRangeRef": {
      if (!ctx.resolveField) return { deps, error: "#NAME!" };
      const refs = ctx.resolveField(node.field);
      if (!refs) return { deps, error: "#NAME!" };
      if (node.fromArg.kind === "number" && node.toArg.kind === "number") {
        const lo = Math.max(1, Math.trunc(node.fromArg.value));
        const hi = Math.min(refs.length, Math.trunc(node.toArg.value));
        for (let r = lo; r <= hi; r++) deps.add(refs[r - 1]);
        return { deps, error: null };
      }
      // Dynamic endpoints → conservative whole-column + index deps.
      for (const r of refs) deps.add(r);
      const a = walkDeps(node.fromArg, ctx, deps);
      if (a.error) return a;
      return walkDeps(node.toArg, ctx, deps);
    }
    case "unary":
    case "postfix":
      return walkDeps(node.arg, ctx, deps);
    case "binary": {
      const l = walkDeps(node.left, ctx, deps);
      if (l.error) return l;
      return walkDeps(node.right, ctx, deps);
    }
    case "call":
      for (const a of node.args) {
        const r = walkDeps(a, ctx, deps);
        if (r.error) return r;
      }
      return { deps, error: null };
  }
}

// ── date helpers (Excel serial-day model) ──────────────────────────────

const MS_PER_DAY = 86400000;

/**
 * Convert an arbitrary value into a serial day (days since 1970-01-01
 * UTC), or null if no reasonable interpretation exists. Accepts:
 *
 *   - number    serial day (passes through unchanged)
 *   - string    ISO 8601 / RFC 3339 date or date-time
 *   - 10 digits epoch SECONDS (Unix convention)
 *   - 13 digits epoch MILLISECONDS (JS convention)
 *
 * Ambiguous numeric inputs (anything that isn't 10 or 13 digits) return
 * null so the caller can emit `#VALUE!` rather than producing nonsense.
 */
export function toSerialDay(v: FormulaValue): number | null {
  if (v == null) return null;
  if (typeof v === "number") {
    if (!Number.isFinite(v)) return null;
    // The heuristic from FORMULAS_SPEC §"TO_DATE epoch heuristic":
    // 10 digits → seconds, 13 digits → milliseconds. Anything else
    // is too ambiguous to guess (could be a serial day, could be a
    // garbage timestamp), so we refuse.
    const abs = Math.abs(Math.trunc(v));
    const digits = abs === 0 ? 1 : Math.floor(Math.log10(abs)) + 1;
    if (digits === 10) return Math.floor((v * 1000) / MS_PER_DAY);
    if (digits === 13) return Math.floor(v / MS_PER_DAY);
    return null;
  }
  if (typeof v !== "string") return null;
  const s = v.trim();
  if (s === "") return null;
  // Accept anything `Date.parse` understands as ISO/RFC 3339. To keep
  // the heuristic tight, require a leading 4-digit year + dash so we
  // don't accidentally accept things like "Tomorrow" or "April".
  if (!/^\d{4}-\d{2}-\d{2}/.test(s)) return null;
  const ms = Date.parse(s);
  if (!Number.isFinite(ms)) return null;
  return Math.floor(ms / MS_PER_DAY);
}

/**
 * DATEDIF body — whole-unit difference between two serial days. Units
 * `"d"` and `"w"` are arithmetic; `"m"` and `"y"` walk the Gregorian
 * calendar and round toward zero (the "have we passed the same day of
 * the month/year?" rule).
 */
export function datediff(start: number, end: number, unit: "d" | "w" | "m" | "y"): number {
  if (unit === "d") return end - start;
  if (unit === "w") return Math.trunc((end - start) / 7);
  const a = new Date(start * MS_PER_DAY);
  const b = new Date(end * MS_PER_DAY);
  if (unit === "y") {
    let years = b.getUTCFullYear() - a.getUTCFullYear();
    // Subtract one if `b` hasn't yet reached the anniversary of `a` —
    // e.g. 2020-06-01 → 2024-05-31 is 3y, not 4y. Compare on (month, day).
    const aKey = a.getUTCMonth() * 100 + a.getUTCDate();
    const bKey = b.getUTCMonth() * 100 + b.getUTCDate();
    if (end >= start && bKey < aKey) years--;
    else if (end < start && bKey > aKey) years++;
    return years;
  }
  // unit === "m"
  let months =
    (b.getUTCFullYear() - a.getUTCFullYear()) * 12 +
    (b.getUTCMonth() - a.getUTCMonth());
  if (end >= start && b.getUTCDate() < a.getUTCDate()) months--;
  else if (end < start && b.getUTCDate() > a.getUTCDate()) months++;
  return months;
}

// Re-export for the broader library so callers don't have to dual-import.
export { sameness } from "./davis";
