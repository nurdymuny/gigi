/**
 * Function metadata registry — powers the Excel-style FormulaPicker.
 *
 * One entry per function the engine supports (RESERVED_NAMES in
 * formula.ts is the source of truth for what's wired). A test enforces
 * that every reserved name has a doc, so the picker never silently
 * omits a function the engine accepts.
 *
 * Categories follow what the user mentally groups by, not the engine's
 * internal switch order:
 *
 *   aggregate    SUM / AVERAGE / MIN / MAX / COUNT / COUNTA / MEDIAN
 *   math         MOD / ABS / ROUND
 *   stats        STDEV / STDEVP / VAR / VARP / PERCENTILE / QUARTILE
 *   logic        IF
 *   text         CONCAT / LEN / LOWER / UPPER / TRIM
 *   date         TODAY / YEAR / MONTH / DAY / DATEDIF / TO_DATE
 *   conditional  SUMIF family + IFS variants
 *   geometry     SAME / DIST / K / COHORT / KAPPA_RANK / SAMENESS_RANK
 */

export type FormulaCategory =
  | "aggregate"
  | "math"
  | "stats"
  | "logic"
  | "text"
  | "date"
  | "conditional"
  | "geometry";

export interface ArgDoc {
  name: string;
  description: string;
  /** Optional args are surfaced with `[brackets]` in the signature. */
  optional?: boolean;
  /** Hint text shown as the input's placeholder. */
  placeholder?: string;
}

export interface FormulaDoc {
  name: string;
  category: FormulaCategory;
  /** Excel-style signature shown verbatim in the picker header. */
  signature: string;
  /** One-line summary for the list view. */
  description: string;
  /** Optional long-form usage detail shown under the args. */
  details?: string;
  args: ArgDoc[];
  /** A worked example, including the `=` prefix. */
  example: string;
  /** What that example evaluates to (display string). Helps users believe it. */
  exampleResult?: string;
}

export const CATEGORY_LABELS: Record<FormulaCategory, string> = {
  aggregate: "Aggregate",
  math: "Math",
  stats: "Stats",
  logic: "Logic",
  text: "Text",
  date: "Date",
  conditional: "Conditional",
  geometry: "Geometry (GIGI)",
};

/* ── Aggregate ───────────────────────────────────────────────────── */
const AGGREGATE: FormulaDoc[] = [
  {
    name: "AVERAGE",
    category: "aggregate",
    signature: "AVERAGE(value1, [value2, …])",
    description: "Mean of the numeric arguments (or a range). Also: AVG.",
    args: [
      { name: "value1", description: "First number, cell ref, or range.", placeholder: "A1:A10" },
      { name: "value2", description: "Additional values to include.", optional: true },
    ],
    example: "=AVERAGE(A1:A5)",
    exampleResult: "23.4",
  },
  {
    name: "COUNT",
    category: "aggregate",
    signature: "COUNT(value1, [value2, …])",
    description: "Count of numeric values. Strings and blanks don't count.",
    args: [
      { name: "value1", description: "Range or value to count.", placeholder: "A1:A10" },
      { name: "value2", description: "Additional values.", optional: true },
    ],
    example: "=COUNT(A1:A10)",
    exampleResult: "7",
  },
  {
    name: "COUNTA",
    category: "aggregate",
    signature: "COUNTA(value1, [value2, …])",
    description: "Count of non-empty cells. Counts strings AND numbers.",
    args: [
      { name: "value1", description: "Range or value.", placeholder: "A1:A10" },
      { name: "value2", description: "Additional values.", optional: true },
    ],
    example: "=COUNTA(C1:C10)",
    exampleResult: "10",
  },
  {
    name: "MAX",
    category: "aggregate",
    signature: "MAX(value1, [value2, …])",
    description: "Largest numeric value across the arguments.",
    args: [
      { name: "value1", description: "First range or number.", placeholder: "A1:A10" },
      { name: "value2", description: "Additional values.", optional: true },
    ],
    example: "=MAX(A1:A5)",
    exampleResult: "42",
  },
  {
    name: "MEDIAN",
    category: "aggregate",
    signature: "MEDIAN(value1, [value2, …])",
    description: "Middle value of the sorted numerics. For even N: mean of the two middles.",
    args: [
      { name: "value1", description: "First range or number.", placeholder: "A1:A5" },
      { name: "value2", description: "Additional values.", optional: true },
    ],
    example: "=MEDIAN(1, 2, 3, 4)",
    exampleResult: "2.5",
  },
  {
    name: "MIN",
    category: "aggregate",
    signature: "MIN(value1, [value2, …])",
    description: "Smallest numeric value across the arguments.",
    args: [
      { name: "value1", description: "First range or number.", placeholder: "A1:A10" },
      { name: "value2", description: "Additional values.", optional: true },
    ],
    example: "=MIN(A1:A5)",
    exampleResult: "1",
  },
  {
    name: "SUM",
    category: "aggregate",
    signature: "SUM(value1, [value2, …])",
    description: "Sum of the numeric arguments (or a range).",
    args: [
      { name: "value1", description: "First number, cell ref, or range.", placeholder: "A1:A10" },
      { name: "value2", description: "Additional values to include.", optional: true },
    ],
    example: "=SUM(A1:A5)",
    exampleResult: "117",
  },
];

/* ── Math ────────────────────────────────────────────────────────── */
const MATH: FormulaDoc[] = [
  {
    name: "ABS",
    category: "math",
    signature: "ABS(number)",
    description: "Absolute value.",
    args: [{ name: "number", description: "A number or cell.", placeholder: "-5" }],
    example: "=ABS(-5)",
    exampleResult: "5",
  },
  {
    name: "MOD",
    category: "math",
    signature: "MOD(dividend, divisor)",
    description: "Remainder. Excel parity: sign of the result matches the divisor.",
    args: [
      { name: "dividend", description: "Number to divide.", placeholder: "10" },
      { name: "divisor", description: "Number to divide by.", placeholder: "3" },
    ],
    example: "=MOD(10, 3)",
    exampleResult: "1",
  },
  {
    name: "ROUND",
    category: "math",
    signature: "ROUND(number, [digits])",
    description: "Round half away from zero (Excel parity), NOT banker's rounding.",
    args: [
      { name: "number", description: "Value to round.", placeholder: "3.14159" },
      {
        name: "digits",
        description: "Decimal places (default 0). Negative rounds to tens, hundreds, …",
        optional: true,
        placeholder: "2",
      },
    ],
    example: "=ROUND(3.14159, 2)",
    exampleResult: "3.14",
  },
];

/* ── Stats ───────────────────────────────────────────────────────── */
const STATS: FormulaDoc[] = [
  {
    name: "PERCENTILE",
    category: "stats",
    signature: "PERCENTILE(range, k)",
    description: "k-th percentile, inclusive interpolation (PERCENTILE.INC).",
    args: [
      { name: "range", description: "Numeric range.", placeholder: "A1:A100" },
      { name: "k", description: "Percentile in [0, 1]. 0.5 = median.", placeholder: "0.95" },
    ],
    example: "=PERCENTILE(A1:A100, 0.95)",
    exampleResult: "42.1",
  },
  {
    name: "QUARTILE",
    category: "stats",
    signature: "QUARTILE(range, quart)",
    description: "Quartile: 0 = min, 1 = Q1, 2 = median, 3 = Q3, 4 = max.",
    args: [
      { name: "range", description: "Numeric range.", placeholder: "A1:A100" },
      { name: "quart", description: "0..4.", placeholder: "3" },
    ],
    example: "=QUARTILE(A1:A100, 3)",
    exampleResult: "35.0",
  },
  {
    name: "STDEV",
    category: "stats",
    signature: "STDEV(value1, [value2, …])",
    description: "Sample standard deviation (n-1 denominator). Needs ≥ 2 values.",
    args: [
      { name: "value1", description: "First range or number.", placeholder: "A1:A100" },
      { name: "value2", description: "Additional values.", optional: true },
    ],
    example: "=STDEV(A1:A100)",
    exampleResult: "4.3",
  },
  {
    name: "STDEVP",
    category: "stats",
    signature: "STDEVP(value1, [value2, …])",
    description: "Population standard deviation (n denominator).",
    args: [
      { name: "value1", description: "First range or number.", placeholder: "A1:A100" },
      { name: "value2", description: "Additional values.", optional: true },
    ],
    example: "=STDEVP(A1:A100)",
    exampleResult: "4.28",
  },
  {
    name: "VAR",
    category: "stats",
    signature: "VAR(value1, [value2, …])",
    description: "Sample variance (n-1 denominator). Needs ≥ 2 values.",
    args: [
      { name: "value1", description: "First range or number.", placeholder: "A1:A100" },
      { name: "value2", description: "Additional values.", optional: true },
    ],
    example: "=VAR(A1:A100)",
    exampleResult: "18.4",
  },
  {
    name: "VARP",
    category: "stats",
    signature: "VARP(value1, [value2, …])",
    description: "Population variance (n denominator).",
    args: [
      { name: "value1", description: "First range or number.", placeholder: "A1:A100" },
      { name: "value2", description: "Additional values.", optional: true },
    ],
    example: "=VARP(A1:A100)",
    exampleResult: "18.2",
  },
];

/* ── Logic ───────────────────────────────────────────────────────── */
const LOGIC: FormulaDoc[] = [
  {
    name: "IF",
    category: "logic",
    signature: "IF(condition, then_value, [else_value])",
    description: "Branch on a condition. Else value defaults to empty.",
    args: [
      { name: "condition", description: "Truthy expression.", placeholder: "A1>0" },
      { name: "then_value", description: "Returned when condition is truthy.", placeholder: "\"ok\"" },
      { name: "else_value", description: "Returned when condition is falsy.", optional: true, placeholder: "\"low\"" },
    ],
    example: '=IF(A1 > 0, "positive", "non-positive")',
    exampleResult: "positive",
  },
];

/* ── Text ────────────────────────────────────────────────────────── */
const TEXT: FormulaDoc[] = [
  {
    name: "CONCAT",
    category: "text",
    signature: "CONCAT(text1, [text2, …])",
    description: "Concatenate strings end-to-end. Function form of the `&` operator.",
    args: [
      { name: "text1", description: "First string / value.", placeholder: "\"Hello \"" },
      { name: "text2", description: "Additional strings.", optional: true, placeholder: "A1" },
    ],
    example: '=CONCAT("Hello ", "world")',
    exampleResult: "Hello world",
  },
  {
    name: "LEN",
    category: "text",
    signature: "LEN(text)",
    description: "Number of characters in the string form of `text`.",
    args: [{ name: "text", description: "String / value.", placeholder: "A1" }],
    example: '=LEN("hello")',
    exampleResult: "5",
  },
  {
    name: "LOWER",
    category: "text",
    signature: "LOWER(text)",
    description: "Lowercase the string form of `text`.",
    args: [{ name: "text", description: "String / value.", placeholder: "A1" }],
    example: '=LOWER("HELLO")',
    exampleResult: "hello",
  },
  {
    name: "TRIM",
    category: "text",
    signature: "TRIM(text)",
    description: "Strip leading/trailing whitespace and collapse internal runs to single spaces.",
    args: [{ name: "text", description: "String / value.", placeholder: "A1" }],
    example: '=TRIM("  hello   world  ")',
    exampleResult: "hello world",
  },
  {
    name: "UPPER",
    category: "text",
    signature: "UPPER(text)",
    description: "Uppercase the string form of `text`.",
    args: [{ name: "text", description: "String / value.", placeholder: "A1" }],
    example: '=UPPER("hello")',
    exampleResult: "HELLO",
  },
];

/* ── Date ────────────────────────────────────────────────────────── */
const DATE: FormulaDoc[] = [
  {
    name: "DATEDIF",
    category: "date",
    signature: 'DATEDIF(start, end, unit)',
    description:
      "Whole-unit difference between two dates. Unit MUST be lowercase: d, w, m, y. Negative when end < start.",
    details:
      'GIGI diverges from Excel\'s uppercase unit set ("Y", "MD", …) — we deliberately reject uppercase to surface paste-from-Excel typos as #VALUE!.',
    args: [
      { name: "start", description: "Start date or serial day.", placeholder: '"2024-01-01"' },
      { name: "end", description: "End date or serial day.", placeholder: '"2024-12-31"' },
      { name: "unit", description: 'Lowercase: "d" (days), "w" (weeks), "m" (months), "y" (years).', placeholder: '"d"' },
    ],
    example: '=DATEDIF("2024-01-01", "2024-12-31", "d")',
    exampleResult: "365",
  },
  {
    name: "DAY",
    category: "date",
    signature: "DAY(date)",
    description: "Day-of-month (1–31), UTC.",
    args: [{ name: "date", description: "Date string or serial day.", placeholder: '"2024-03-15"' }],
    example: '=DAY("2024-03-15")',
    exampleResult: "15",
  },
  {
    name: "MONTH",
    category: "date",
    signature: "MONTH(date)",
    description: "Month number (1–12), UTC.",
    args: [{ name: "date", description: "Date string or serial day.", placeholder: '"2024-03-15"' }],
    example: '=MONTH("2024-03-15")',
    exampleResult: "3",
  },
  {
    name: "TODAY",
    category: "date",
    signature: "TODAY()",
    description:
      "Today's date as a serial day. Deterministic per bundle-load — not call-time volatile.",
    args: [],
    example: "=TODAY()",
  },
  {
    name: "TO_DATE",
    category: "date",
    signature: "TO_DATE(value)",
    description:
      "Coerce a string or number to a serial-day date. ISO 8601, RFC 3339, or epoch ms (13 digits) / seconds (10 digits).",
    args: [{ name: "value", description: "Date string, or epoch ms / seconds.", placeholder: '"2024-03-15"' }],
    example: '=TO_DATE("2024-03-15")',
  },
  {
    name: "YEAR",
    category: "date",
    signature: "YEAR(date)",
    description: "Four-digit year, UTC.",
    args: [{ name: "date", description: "Date string or serial day.", placeholder: '"2024-03-15"' }],
    example: '=YEAR("2024-03-15")',
    exampleResult: "2024",
  },
];

/* ── Conditional (the *IF / *IFS family) ─────────────────────────── */
const CONDITIONAL: FormulaDoc[] = [
  {
    name: "AVERAGEIF",
    category: "conditional",
    signature: "AVERAGEIF(range, predicate, [avg_range])",
    description: "Average of cells where the predicate matches. Returns #DIV0! if no match.",
    args: [
      { name: "range", description: "Cells to evaluate.", placeholder: "A1:A100" },
      { name: "predicate", description: 'A predicate like ">5", "<>0", "INV*".', placeholder: '">10"' },
      { name: "avg_range", description: "Optional alt range to average. Defaults to range.", optional: true, placeholder: "B1:B100" },
    ],
    example: '=AVERAGEIF(A1:A10, ">5")',
  },
  {
    name: "AVERAGEIFS",
    category: "conditional",
    signature: "AVERAGEIFS(avg_range, range1, pred1, [range2, pred2, …])",
    description: "Average over avg_range where every (range_i, pred_i) pair matches.",
    args: [
      { name: "avg_range", description: "Cells to average.", placeholder: "B1:B100" },
      { name: "range1", description: "First criteria range.", placeholder: "A1:A100" },
      { name: "pred1", description: "First predicate.", placeholder: '">5"' },
      { name: "range2", description: "Second criteria range.", optional: true },
      { name: "pred2", description: "Second predicate.", optional: true },
    ],
    example: '=AVERAGEIFS(B1:B10, A1:A10, ">5", C1:C10, "USD")',
  },
  {
    name: "COUNTIF",
    category: "conditional",
    signature: "COUNTIF(range, predicate)",
    description: "Count cells matching the predicate.",
    args: [
      { name: "range", description: "Cells to evaluate.", placeholder: "A1:A100" },
      { name: "predicate", description: 'Like ">5", "INV*", "<>0".', placeholder: '">5"' },
    ],
    example: '=COUNTIF(A1:A10, "INV*")',
  },
  {
    name: "COUNTIFS",
    category: "conditional",
    signature: "COUNTIFS(range1, pred1, [range2, pred2, …])",
    description: "Count rows where every (range_i, pred_i) pair matches.",
    args: [
      { name: "range1", description: "First criteria range.", placeholder: "A1:A100" },
      { name: "pred1", description: "First predicate.", placeholder: '">5"' },
      { name: "range2", description: "Second criteria range.", optional: true },
      { name: "pred2", description: "Second predicate.", optional: true },
    ],
    example: '=COUNTIFS(A1:A10, ">5", B1:B10, "USD")',
  },
  {
    name: "MAXIFS",
    category: "conditional",
    signature: "MAXIFS(max_range, range1, pred1, [range2, pred2, …])",
    description: "Max over max_range where every (range_i, pred_i) pair matches.",
    args: [
      { name: "max_range", description: "Cells to take max of.", placeholder: "B1:B100" },
      { name: "range1", description: "First criteria range.", placeholder: "A1:A100" },
      { name: "pred1", description: "First predicate.", placeholder: '">5"' },
      { name: "range2", description: "Second criteria range.", optional: true },
      { name: "pred2", description: "Second predicate.", optional: true },
    ],
    example: '=MAXIFS(B1:B10, A1:A10, "INV*")',
  },
  {
    name: "MINIFS",
    category: "conditional",
    signature: "MINIFS(min_range, range1, pred1, [range2, pred2, …])",
    description: "Min over min_range where every (range_i, pred_i) pair matches.",
    args: [
      { name: "min_range", description: "Cells to take min of.", placeholder: "B1:B100" },
      { name: "range1", description: "First criteria range.", placeholder: "A1:A100" },
      { name: "pred1", description: "First predicate.", placeholder: '">5"' },
      { name: "range2", description: "Second criteria range.", optional: true },
      { name: "pred2", description: "Second predicate.", optional: true },
    ],
    example: '=MINIFS(B1:B10, A1:A10, "INV*")',
  },
  {
    name: "SUMIF",
    category: "conditional",
    signature: "SUMIF(range, predicate, [sum_range])",
    description: "Sum of cells where the predicate matches.",
    details:
      'Predicate grammar: ">N", "<N", ">=N", "<=N", "<>N", "=N", "N", "prefix*", "*suffix", or a bare value.',
    args: [
      { name: "range", description: "Cells the predicate evaluates against.", placeholder: "A1:A100" },
      { name: "predicate", description: 'A predicate like ">5", "<>0", "INV*".', placeholder: '">5"' },
      {
        name: "sum_range",
        description: "Optional alternate range to sum. Defaults to range.",
        optional: true,
        placeholder: "B1:B100",
      },
    ],
    example: '=SUMIF(A1:A10, ">5", B1:B10)',
  },
  {
    name: "SUMIFS",
    category: "conditional",
    signature: "SUMIFS(sum_range, range1, pred1, [range2, pred2, …])",
    description: "Sum over sum_range where every (range_i, pred_i) pair matches.",
    args: [
      { name: "sum_range", description: "Cells to sum.", placeholder: "B1:B100" },
      { name: "range1", description: "First criteria range.", placeholder: "A1:A100" },
      { name: "pred1", description: "First predicate.", placeholder: '">5"' },
      { name: "range2", description: "Second criteria range.", optional: true },
      { name: "pred2", description: "Second predicate.", optional: true },
    ],
    example: '=SUMIFS(B1:B10, A1:A10, ">5", C1:C10, "USD")',
  },
];

/* ── Geometry (GIGI-native) ──────────────────────────────────────── */
const GEOMETRY: FormulaDoc[] = [
  {
    name: "COHORT",
    category: "geometry",
    signature: "COHORT(ref)",
    description: "Cohort label for the row containing `ref` — the value at the cover field.",
    args: [{ name: "ref", description: "Any cell in the target row.", placeholder: "A1" }],
    example: "=COHORT(A1)",
    exampleResult: "ACME Corp",
  },
  {
    name: "DIST",
    category: "geometry",
    signature: "DIST(a, b)",
    description:
      "Davis distance between the rows containing `a` and `b`. Derived from SAME — SAME + DIST² = 1 exactly.",
    args: [
      { name: "a", description: "Any cell in row A.", placeholder: "A1" },
      { name: "b", description: "Any cell in row B.", placeholder: "A2" },
    ],
    example: "=DIST(A1, A2)",
  },
  {
    name: "K",
    category: "geometry",
    signature: "K(ref)",
    description: "Curvature κ for the row containing `ref` (cohort-relative).",
    args: [{ name: "ref", description: "Any cell in the target row.", placeholder: "A1" }],
    example: "=K(A1)",
    exampleResult: "0.84",
  },
  {
    name: "KAPPA_RANK",
    category: "geometry",
    signature: "KAPPA_RANK(ref)",
    description: "Dense rank by κ descending — 1 = highest κ (most anomalous).",
    args: [{ name: "ref", description: "Any cell in the target row.", placeholder: "A1" }],
    example: "=KAPPA_RANK(A1)",
  },
  {
    name: "SAME",
    category: "geometry",
    signature: "SAME(a, b)",
    description:
      "Davis sameness between the rows containing `a` and `b`. (1 + cos θ) / 2 ∈ [0, 1].",
    args: [
      { name: "a", description: "Any cell in row A.", placeholder: "A1" },
      { name: "b", description: "Any cell in row B.", placeholder: "A2" },
    ],
    example: "=SAME(A1, A2)",
  },
  {
    name: "SAMENESS_RANK",
    category: "geometry",
    signature: "SAMENESS_RANK(pivot, ref)",
    description:
      "Dense rank by sameness to pivot, descending. The pivot itself ranks 1 (S=1 against itself).",
    args: [
      { name: "pivot", description: "Reference row.", placeholder: "A1" },
      { name: "ref", description: "Row to rank.", placeholder: "A2" },
    ],
    example: "=SAMENESS_RANK(A1, A2)",
  },
];

export const FORMULA_DOCS: FormulaDoc[] = [
  ...AGGREGATE,
  ...MATH,
  ...STATS,
  ...LOGIC,
  ...TEXT,
  ...DATE,
  ...CONDITIONAL,
  ...GEOMETRY,
];

/** Case-insensitive lookup by function name. */
export function findDoc(name: string): FormulaDoc | null {
  const upper = name.toUpperCase();
  return FORMULA_DOCS.find((d) => d.name === upper) ?? null;
}

/**
 * Search the registry by name / description. Empty query returns the
 * full sorted list. Results are ordered:
 *   1. Name-prefix matches first (alphabetical within the bucket)
 *   2. Then name-substring matches
 *   3. Then description-substring matches
 *
 * Matching is case-insensitive throughout.
 */
export function searchDocs(query: string): FormulaDoc[] {
  const q = query.trim().toLowerCase();
  if (q === "") return [...FORMULA_DOCS].sort(byName);
  const prefix: FormulaDoc[] = [];
  const nameIn: FormulaDoc[] = [];
  const descIn: FormulaDoc[] = [];
  for (const d of FORMULA_DOCS) {
    const n = d.name.toLowerCase();
    if (n.startsWith(q)) prefix.push(d);
    else if (n.includes(q)) nameIn.push(d);
    else if (d.description.toLowerCase().includes(q)) descIn.push(d);
  }
  return [
    ...prefix.sort(byName),
    ...nameIn.sort(byName),
    ...descIn.sort(byName),
  ];
}

function byName(a: FormulaDoc, b: FormulaDoc): number {
  return a.name.localeCompare(b.name);
}

/**
 * Assemble a `=FN(arg1, arg2, …)` string from a function name and its
 * filled-in argument values. Trailing empty args are dropped so optional
 * params don't leave a trailing comma; interior empty args are preserved
 * (the user might mean `IF(cond, , else)`).
 *
 * Each arg is trimmed of surrounding whitespace. The function name is
 * uppercased (`sum` → `SUM`) so the assembled formula uses the canonical
 * form regardless of how the picker referenced it.
 */
export function assembleFormula(name: string, args: string[]): string {
  const upper = name.toUpperCase();
  const trimmed = args.map((a) => a.trim());
  // Strip trailing empties so `SUMIF(A1:A5, ">5", )` becomes
  // `SUMIF(A1:A5, ">5")`. We leave interior empties alone since the
  // user might intentionally have an empty middle arg.
  let end = trimmed.length;
  while (end > 0 && trimmed[end - 1] === "") end--;
  const list = trimmed.slice(0, end).join(", ");
  return `=${upper}(${list})`;
}
