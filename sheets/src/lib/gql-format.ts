/**
 * Tiny deterministic GQL formatter.
 *
 * Goals:
 *   - Idempotent: format(format(q)) === format(q)
 *   - Predictable: one clause per line, in canonical order
 *   - Conservative: don't touch the inside of string literals
 *
 * Non-goals (for v0.1):
 *   - Full AST pretty-print. We never parse; we just split on clause keywords.
 *   - Keyword case normalization (kept as written).
 */

const CLAUSE_KEYWORDS = [
  "SECTION",
  "CREATE BUNDLE",
  "DROP BUNDLE",
  "ALTER BUNDLE",
  "INSERT",
  "UPDATE",
  "DELETE",
  "WHERE",
  "ORDER BY",
  "GROUP BY",
  "HAVING",
  "LIMIT",
  "OFFSET",
  "ON FIBER",
  "FROM",
  "TO",
  "AROUND",
  "AT",
  "WITH",
  "RETURNING",
  "MODES",
  "INTEGRATE",
  "OVER",
  "COVER",
  "CURVATURE",
  "SPECTRAL",
  "TRANSPORT",
  "HOLONOMY",
  "BETTI",
  "CONSISTENCY",
];

const SECONDARY_KEYWORDS = ["AND", "OR"];

/**
 * Reformat a GQL query.
 *
 * - Collapses runs of whitespace outside strings to one space.
 * - Inserts a newline before each clause keyword.
 * - Indents AND / OR continuations by two spaces.
 * - Preserves the trailing `;` if present.
 */
export function formatGql(query: string): string {
  if (!query.trim()) return "";
  // Split on string literals so we never edit their contents.
  const segments = splitOnStrings(query);
  // Squash whitespace outside string literals.
  for (let i = 0; i < segments.length; i++) {
    if (segments[i].isString) continue;
    segments[i].text = segments[i].text.replace(/\s+/g, " ");
  }
  // Stitch back together.
  let s = segments.map((s) => s.text).join("");
  s = s.trim();

  // Preserve a trailing semicolon and strip it for processing.
  let trailingSemicolon = "";
  if (s.endsWith(";")) {
    s = s.slice(0, -1).trim();
    trailingSemicolon = ";";
  }

  // Insert \n before clause keywords (sorted longest-first so multi-word
  // keywords like "ORDER BY" win over "BY").
  const sorted = [...CLAUSE_KEYWORDS].sort((a, b) => b.length - a.length);
  for (const kw of sorted) {
    const re = new RegExp(`(^|\\s)${escapeRegExp(kw)}(?=\\s|$|\\()`, "gi");
    s = s.replace(re, (_match, lead) => `${lead === "" ? "" : "\n"}${kw}`);
  }

  // Insert \n + indent before AND / OR (which continue a WHERE).
  for (const kw of SECONDARY_KEYWORDS) {
    const re = new RegExp(`\\s+${kw}\\b`, "gi");
    s = s.replace(re, `\n  ${kw}`);
  }

  // Clean up: any double newlines → single, trim leading newline if any.
  s = s.replace(/\n\s*\n/g, "\n").replace(/^\n+/, "");

  return s + trailingSemicolon;
}

interface Segment {
  text: string;
  isString: boolean;
}

/**
 * Split a string into segments alternating between "outside string" and
 * "inside single-quoted string". Doubled-single-quotes inside strings
 * are treated as escaped quotes (GQL convention).
 */
function splitOnStrings(s: string): Segment[] {
  const out: Segment[] = [];
  let buf = "";
  let inStr = false;
  for (let i = 0; i < s.length; i++) {
    const c = s[i];
    if (!inStr) {
      if (c === "'") {
        if (buf) out.push({ text: buf, isString: false });
        buf = c;
        inStr = true;
      } else {
        buf += c;
      }
    } else {
      buf += c;
      if (c === "'") {
        // Check for escaped quote ('').
        if (s[i + 1] === "'") {
          buf += "'";
          i++;
          continue;
        }
        out.push({ text: buf, isString: true });
        buf = "";
        inStr = false;
      }
    }
  }
  if (buf) out.push({ text: buf, isString: inStr });
  return out;
}

function escapeRegExp(s: string): string {
  return s.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
}
