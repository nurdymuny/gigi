/**
 * Micro-predicate language for the *IF family.
 *
 * Surface (FORMULAS_SPEC §"Conditional aggregates"):
 *
 *   ">N"   "<N"   ">=N"   "<=N"   "<>N"   "=N"   N
 *   "value"   "prefix*"   "*suffix"
 *
 * That's the whole grammar. No regex, no AND/OR — multi-criterion lives
 * at the `*IFS` layer (every pair AND'd). Keeping the surface tiny means
 * `SUMIF`/`COUNTIF` stay readable in formulas, and the parser stays
 * ~50 lines of code (the spec's commitment).
 *
 * Equality on strings is case-insensitive (Excel parity). Ordering on
 * strings uses default `localeCompare` (sensitivity=base, so accent +
 * case-insensitive). Numbers compare numerically.
 */

export type PredicateOp =
  | "="
  | "<>"
  | ">"
  | ">="
  | "<"
  | "<="
  | "starts"
  | "ends";

export interface Predicate {
  op: PredicateOp;
  operand: number | string | boolean | null;
}

/**
 * Parse a raw predicate value (string or scalar) into the normalized
 * `Predicate` form. Numbers and booleans become `=` predicates. Strings
 * are scanned for an operator prefix and a numeric operand; if the
 * operand looks numeric we coerce, otherwise it's compared stringwise.
 *
 * The `*` wildcard is only honored at one end of the string and only as
 * the entire prefix/suffix marker — `f*o` is a literal three-char string,
 * not a glob.
 */
export function parsePredicate(raw: unknown): Predicate {
  if (typeof raw === "number") return { op: "=", operand: raw };
  if (typeof raw === "boolean") return { op: "=", operand: raw };
  if (raw == null) return { op: "=", operand: null };
  const s = String(raw);

  // Quoted empty string `""` → match null/empty. This is the only escape
  // path for "is this cell blank" — `=""` as a SUMIF criterion.
  if (s === '=""' || s === '""') return { op: "=", operand: "" };

  // Wildcards: `prefix*` or `*suffix`. A bare `*` is starts-with-empty
  // (matches anything that's a string).
  if (s.length > 0 && s !== "*" && s[0] === "*" && !s.slice(1).includes("*")) {
    return { op: "ends", operand: s.slice(1) };
  }
  if (s.length > 1 && s[s.length - 1] === "*" && !s.slice(0, -1).includes("*")) {
    return { op: "starts", operand: s.slice(0, -1) };
  }
  if (s === "*") return { op: "starts", operand: "" };

  // Operator prefix scan. Order matters: check the two-char ops first.
  let op: PredicateOp = "=";
  let rest = s;
  if (s.startsWith(">=")) { op = ">="; rest = s.slice(2); }
  else if (s.startsWith("<=")) { op = "<="; rest = s.slice(2); }
  else if (s.startsWith("<>")) { op = "<>"; rest = s.slice(2); }
  else if (s.startsWith(">")) { op = ">"; rest = s.slice(1); }
  else if (s.startsWith("<")) { op = "<"; rest = s.slice(1); }
  else if (s.startsWith("=")) { op = "="; rest = s.slice(1); }

  // Try to coerce the operand to a number. If the remaining string is
  // empty or non-numeric, keep it as a string operand.
  const trimmed = rest.trim();
  if (trimmed !== "") {
    const n = Number(trimmed);
    if (Number.isFinite(n) && /^-?\d*\.?\d+([eE][+-]?\d+)?$/.test(trimmed)) {
      return { op, operand: n };
    }
  }
  return { op, operand: rest };
}

/**
 * Test a single cell value against a parsed predicate.
 *
 * Null/undefined values never match anything except an explicit empty-
 * string equality (`=""`) — matching FORMULAS_SPEC §"Predicate vs
 * column type": "null values never match any predicate other than
 * explicit `=` against empty/null."
 *
 * Numeric operand + non-numeric value → coerce to number, fail (false)
 * if the value isn't finite. Numeric operand + numeric value → numeric
 * compare. String operand → case-insensitive string compare for `=`/`<>`,
 * `localeCompare` for ordering.
 */
export function matchesPredicate(value: unknown, p: Predicate): boolean {
  // Null handling: only =""/="" / =null matches.
  if (value == null) {
    if (p.op === "=" && (p.operand === "" || p.operand == null)) return true;
    return false;
  }
  // Starts/ends are always string ops, always case-insensitive.
  if (p.op === "starts" || p.op === "ends") {
    const v = String(value).toLowerCase();
    const o = String(p.operand ?? "").toLowerCase();
    return p.op === "starts" ? v.startsWith(o) : v.endsWith(o);
  }

  if (typeof p.operand === "number") {
    const n = typeof value === "number"
      ? value
      : typeof value === "boolean"
        ? value ? 1 : 0
        : Number(value);
    if (!Number.isFinite(n)) return false;
    switch (p.op) {
      case "=":  return n === p.operand;
      case "<>": return n !== p.operand;
      case ">":  return n > p.operand;
      case ">=": return n >= p.operand;
      case "<":  return n < p.operand;
      case "<=": return n <= p.operand;
    }
  }

  // String / boolean operand path.
  if (p.op === "=" || p.op === "<>") {
    const a = String(value).toLowerCase();
    const b = String(p.operand ?? "").toLowerCase();
    const eq = a === b;
    return p.op === "=" ? eq : !eq;
  }
  // Ordering — locale-aware base-sensitivity (case-insensitive).
  const cmp = String(value).localeCompare(String(p.operand ?? ""), undefined, {
    sensitivity: "base",
  });
  switch (p.op) {
    case ">":  return cmp > 0;
    case ">=": return cmp >= 0;
    case "<":  return cmp < 0;
    case "<=": return cmp <= 0;
  }
  return false;
}
