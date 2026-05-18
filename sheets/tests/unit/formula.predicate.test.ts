import { describe, expect, it } from "vitest";
import { matchesPredicate, parsePredicate } from "../../src/lib/formula-predicate";

/**
 * Phase 1.5.A · micro-predicate grammar for the *IF family.
 *
 * Surface (intentionally tiny — see FORMULAS_SPEC §"Conditional aggregates"):
 *
 *   ">N"   "<N"   ">=N"   "<=N"   "<>N"   "=N"   N
 *   "value"   "prefix*"   "*suffix"
 *
 * No regexes, no AND/OR compounds. AND-of-pairs lives at the *IFS layer.
 *
 * `parsePredicate` returns a normalized object; `matchesPredicate` is
 * the value-vs-predicate runtime. Tests below pin both.
 */

describe("formula-predicate · parsing", () => {
  it("bare number → equality on that number", () => {
    expect(parsePredicate(5)).toEqual({ op: "=", operand: 5 });
  });

  it("bare string with no operator → string equality", () => {
    expect(parsePredicate("warn")).toEqual({ op: "=", operand: "warn" });
  });

  it('"=5" → equality on 5', () => {
    expect(parsePredicate("=5")).toEqual({ op: "=", operand: 5 });
  });

  it('">5" → greater-than 5', () => {
    expect(parsePredicate(">5")).toEqual({ op: ">", operand: 5 });
  });

  it('">=5.5" → greater-or-equal 5.5', () => {
    expect(parsePredicate(">=5.5")).toEqual({ op: ">=", operand: 5.5 });
  });

  it('"<0" → less-than 0', () => {
    expect(parsePredicate("<0")).toEqual({ op: "<", operand: 0 });
  });

  it('"<=10" → less-or-equal 10', () => {
    expect(parsePredicate("<=10")).toEqual({ op: "<=", operand: 10 });
  });

  it('"<>5" → not-equal 5', () => {
    expect(parsePredicate("<>5")).toEqual({ op: "<>", operand: 5 });
  });

  it('">M" → greater-than with text operand (lex compare at runtime)', () => {
    expect(parsePredicate(">M")).toEqual({ op: ">", operand: "M" });
  });

  it('"INV*" → starts-with "INV"', () => {
    expect(parsePredicate("INV*")).toEqual({ op: "starts", operand: "INV" });
  });

  it('"*payment" → ends-with "payment"', () => {
    expect(parsePredicate("*payment")).toEqual({ op: "ends", operand: "payment" });
  });

  it('"*" alone → match-anything (starts-with empty)', () => {
    // Useful as a "is this column populated" filter when combined with `<>""`.
    expect(parsePredicate("*")).toEqual({ op: "starts", operand: "" });
  });

  it('boolean true → equality on true', () => {
    expect(parsePredicate(true)).toEqual({ op: "=", operand: true });
  });
});

describe("formula-predicate · numeric range matching", () => {
  it("=5 matches 5, not 5.1", () => {
    expect(matchesPredicate(5, parsePredicate(5))).toBe(true);
    expect(matchesPredicate(5.1, parsePredicate(5))).toBe(false);
  });

  it(">5 matches 6 but not 5", () => {
    const p = parsePredicate(">5");
    expect(matchesPredicate(6, p)).toBe(true);
    expect(matchesPredicate(5, p)).toBe(false);
  });

  it(">=5 matches 5", () => {
    expect(matchesPredicate(5, parsePredicate(">=5"))).toBe(true);
  });

  it("<>5 matches anything but 5", () => {
    const p = parsePredicate("<>5");
    expect(matchesPredicate(4, p)).toBe(true);
    expect(matchesPredicate(5, p)).toBe(false);
    expect(matchesPredicate(6, p)).toBe(true);
  });
});

describe("formula-predicate · string matching", () => {
  it("=warn matches 'warn' case-insensitively", () => {
    const p = parsePredicate("warn");
    expect(matchesPredicate("warn", p)).toBe(true);
    expect(matchesPredicate("WARN", p)).toBe(true);
    expect(matchesPredicate("Warn", p)).toBe(true);
    expect(matchesPredicate("ok", p)).toBe(false);
  });

  it("'INV*' starts-with 'INV' (case-insensitive)", () => {
    const p = parsePredicate("INV*");
    expect(matchesPredicate("INV-123", p)).toBe(true);
    expect(matchesPredicate("inv-456", p)).toBe(true);
    expect(matchesPredicate("PO-789", p)).toBe(false);
  });

  it("'*payment' ends-with 'payment' (case-insensitive, Excel parity)", () => {
    const p = parsePredicate("*payment");
    expect(matchesPredicate("late payment", p)).toBe(true);
    // Whole-string match counts as ending-with for case-insensitive compare:
    // "PAYMENT".toLowerCase() === "payment", which trivially ends with "payment".
    expect(matchesPredicate("PAYMENT", p)).toBe(true);
    expect(matchesPredicate("late Payment", p)).toBe(true);
    // Trailing characters break the ends-with — "payment due" ends with "due".
    expect(matchesPredicate("payment due", p)).toBe(false);
  });

  it(">M lex-compares strings", () => {
    const p = parsePredicate(">M");
    expect(matchesPredicate("N", p)).toBe(true);
    expect(matchesPredicate("Z", p)).toBe(true);
    expect(matchesPredicate("M", p)).toBe(false);
    expect(matchesPredicate("A", p)).toBe(false);
  });
});

describe("formula-predicate · null + cross-type behavior", () => {
  it("null never matches anything except =null (operand is empty)", () => {
    expect(matchesPredicate(null, parsePredicate(">5"))).toBe(false);
    expect(matchesPredicate(null, parsePredicate("warn"))).toBe(false);
    expect(matchesPredicate(null, parsePredicate("*foo"))).toBe(false);
  });

  it('null matches =""', () => {
    expect(matchesPredicate(null, parsePredicate('=""'))).toBe(true);
  });

  it('empty string matches =""', () => {
    expect(matchesPredicate("", parsePredicate('=""'))).toBe(true);
  });

  it("a string value against numeric >5 fails to coerce → false", () => {
    expect(matchesPredicate("abc", parsePredicate(">5"))).toBe(false);
  });

  it("a numeric value against a text predicate compares stringwise", () => {
    // 5 → "5" → lex-compare against "M" — "5" < "M" (digits sort before letters)
    expect(matchesPredicate(5, parsePredicate(">M"))).toBe(false);
  });
});
