/**
 * Number / date format strings with a κ-conditional extension.
 *
 * The format language is a deliberately small Excel-flavored subset:
 *
 *   0.00          two-decimal places, no thousands separator
 *   #,##0.00      thousands separator + two decimals
 *   $#,##0.00     literal prefix '$'
 *   0%            percentage with no decimals
 *   0.0%          percentage with one decimal
 *   YYYY-MM-DD    ISO date
 *   "literal"...  string-literal segments
 *   [κ>τ]…        the GIGI extension: prefix the format with a condition;
 *                 the part *after* the bracket is applied only when the
 *                 row's curvature exceeds τ. The format core (digits +
 *                 grouping) still runs unconditionally; only literal
 *                 prefixes inside the conditional body are gated.
 *
 * `defaultFormatFor` does schema-driven inference so a freshly imported
 * USD column auto-formats as currency without the user touching anything.
 */

export interface FieldDescriptor {
  name: string;
  /** Engine field types. Kept as a plain string so we can accept the
   *  gigi-client's wider type without a cast. Only "numeric" and
   *  "timestamp" trigger format inference today; other values fall
   *  through to the schema-default null path. */
  type: string;
}

export interface FormatContext {
  /** Curvature for this row. Used by `[κ>τ]` conditions. */
  kappa: number;
}

export interface ParsedFormat {
  condition: { kind: "kappa-gt"; threshold: number } | null;
  body: string;
}

const KAPPA_PREFIX = /^\[κ>([0-9.]+)\]/;

/** Strip the optional `[κ>τ]` condition off the front of a format string. */
export function parseFormatString(fmt: string): ParsedFormat {
  const m = fmt.match(KAPPA_PREFIX);
  if (!m) return { condition: null, body: fmt };
  return {
    condition: { kind: "kappa-gt", threshold: Number(m[1]) },
    body: fmt.slice(m[0].length),
  };
}

/** Heuristic default format for a column based on its name + type. */
export function defaultFormatFor(field: FieldDescriptor): string | null {
  const n = field.name.toLowerCase();
  if (field.type === "numeric") {
    if (/_usd$|^total_|^amount_|_amount$|^net_|_net$/.test(n)) return "$#,##0.00";
    if (/_pct$|_percent$|_rate$|^pct_/.test(n)) return "0.0%";
  }
  if (field.type === "timestamp" || /_date$|^date_/.test(n)) return "YYYY-MM-DD";
  return null;
}

/** Format a raw value according to the format string + κ context. */
export function formatValue(value: unknown, fmt: string, ctx: FormatContext): string {
  if (value == null || value === "") return "";
  const parsed = parseFormatString(fmt);
  // The conditional only gates literal-prefix segments inside the body.
  // The numeric / date core still runs.
  const conditionMet =
    parsed.condition && ctx.kappa > parsed.condition.threshold;
  let body = parsed.body;
  if (parsed.condition && !conditionMet) {
    // Strip leading literal-prefix segments ("…") that were meant for the
    // conditional branch.
    body = body.replace(/^("[^"]*")+/, "");
  }

  if (typeof value === "number") {
    return formatNumber(value, body);
  }
  if (value instanceof Date) {
    return formatDate(value, body);
  }
  // Strings: try date parse if format looks date-y, otherwise pass through.
  if (/Y{2,4}|M{1,2}|D{1,2}/.test(body)) {
    const d = new Date(String(value));
    if (!Number.isNaN(d.getTime())) return formatDate(d, body);
  }
  return String(value);
}

/** Format a number against a numeric format string. */
export function formatNumber(value: number, fmt: string): string {
  if (!Number.isFinite(value)) return "";
  // Extract string-literal segments ("..." or "..").
  const litRe = /"([^"]*)"/g;
  const literals: string[] = [];
  const stripped = fmt.replace(litRe, (_m, lit) => {
    literals.push(lit);
    return "\x00";
  });

  // Detect percent.
  const isPct = stripped.includes("%");
  let v = isPct ? value * 100 : value;

  // Decimal places — count zeros after the dot.
  const dotIdx = stripped.indexOf(".");
  let decimals = 0;
  if (dotIdx >= 0) {
    let j = dotIdx + 1;
    while (j < stripped.length && stripped[j] === "0") {
      decimals++;
      j++;
    }
  }

  // Thousands separator presence.
  const useThousands = stripped.includes(",");

  const sign = v < 0 ? "-" : "";
  v = Math.abs(v);
  let rendered = v.toFixed(decimals);
  if (useThousands) {
    const [whole, frac] = rendered.split(".");
    const withSep = whole.replace(/\B(?=(\d{3})+(?!\d))/g, ",");
    rendered = frac != null ? `${withSep}.${frac}` : withSep;
  }
  rendered = sign + rendered;
  if (isPct) rendered += "%";

  // Re-insert literal prefixes (anything before the digit pattern in the
  // original format).
  // We replaced each "…" with \x00 above — walk back through the original
  // stripped format and emit literals where we saw them.
  let prefix = "";
  let litIdx = 0;
  for (const ch of stripped) {
    if (ch === "\x00") {
      prefix += literals[litIdx++];
      continue;
    }
    // Stop at the first numeric placeholder — what follows is the
    // number itself which we've already rendered.
    if (ch === "0" || ch === "#" || ch === ",") break;
    prefix += ch;
  }
  return prefix + rendered;
}

/** Format a Date against a date format. Tiny subset: YYYY, MM, DD, MMM, D. */
function formatDate(d: Date, fmt: string): string {
  const y = d.getUTCFullYear();
  const m = d.getUTCMonth() + 1;
  const day = d.getUTCDate();
  const months = ["Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec"];
  return fmt
    .replace(/YYYY/g, String(y))
    .replace(/MMM/g, months[m - 1])
    .replace(/MM/g, String(m).padStart(2, "0"))
    .replace(/DD/g, String(day).padStart(2, "0"))
    .replace(/D/g, String(day));
}
