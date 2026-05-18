import { describe, expect, it } from "vitest";
import { evaluate, type FormulaContext } from "../../src/lib/formula";

/**
 * Phase 1.5.C · string functions: LEN, LOWER, UPPER, TRIM.
 *
 * CONCAT already shipped in Phase 1 alongside the `&` operator.
 */

function makeCtx(): FormulaContext {
  const cells: Record<string, number | string | null> = {
    A1: "hello",
    A2: "  Hello World  ",
    A3: null,
    A4: 42,
    A5: "",
  };
  return {
    cell: (ref) => (ref in cells ? cells[ref] : null),
    sameness: () => 0.5,
    kappa: () => 0,
    cohort: () => "",
  };
}

describe("formula · LEN", () => {
  it('=LEN("hello") returns 5', () => {
    expect(evaluate('=LEN("hello")', makeCtx()).value).toBe(5);
  });

  it("=LEN(A1) returns 5", () => {
    expect(evaluate("=LEN(A1)", makeCtx()).value).toBe(5);
  });

  it("=LEN(A3) on null returns 0", () => {
    expect(evaluate("=LEN(A3)", makeCtx()).value).toBe(0);
  });

  it("=LEN(A4) on a number coerces to string first → 2", () => {
    expect(evaluate("=LEN(A4)", makeCtx()).value).toBe(2);
  });

  it("=LEN on multiple args → #ERROR!", () => {
    expect(evaluate('=LEN("a", "b")', makeCtx()).error).toBe("#ERROR!");
  });
});

describe("formula · LOWER", () => {
  it('=LOWER("HELLO") returns "hello"', () => {
    expect(evaluate('=LOWER("HELLO")', makeCtx()).value).toBe("hello");
  });

  it('=LOWER("Mixed Case") returns "mixed case"', () => {
    expect(evaluate('=LOWER("Mixed Case")', makeCtx()).value).toBe("mixed case");
  });

  it("=LOWER(A3) on null returns empty string", () => {
    expect(evaluate("=LOWER(A3)", makeCtx()).value).toBe("");
  });

  it("=LOWER(A4) on number returns the lowercased string form", () => {
    expect(evaluate("=LOWER(A4)", makeCtx()).value).toBe("42");
  });
});

describe("formula · UPPER", () => {
  it('=UPPER("hello") returns "HELLO"', () => {
    expect(evaluate('=UPPER("hello")', makeCtx()).value).toBe("HELLO");
  });

  it("=UPPER(A3) on null returns empty string", () => {
    expect(evaluate("=UPPER(A3)", makeCtx()).value).toBe("");
  });
});

describe("formula · TRIM", () => {
  it('=TRIM("  hello  ") strips leading and trailing whitespace', () => {
    expect(evaluate('=TRIM("  hello  ")', makeCtx()).value).toBe("hello");
  });

  it("=TRIM(A2) collapses internal runs of whitespace to a single space", () => {
    // Excel TRIM collapses internal whitespace runs to single spaces.
    // "  Hello World  " → "Hello World"
    expect(evaluate("=TRIM(A2)", makeCtx()).value).toBe("Hello World");
  });

  it('=TRIM("a   b   c") collapses runs of whitespace', () => {
    expect(evaluate('=TRIM("a   b   c")', makeCtx()).value).toBe("a b c");
  });

  it("=TRIM(A3) on null returns empty string", () => {
    expect(evaluate("=TRIM(A3)", makeCtx()).value).toBe("");
  });

  it("=TRIM handles tabs and newlines as whitespace", () => {
    expect(evaluate('=TRIM("a\tb\nc")', makeCtx()).value).toBe("a b c");
  });
});

describe("formula · string functions compose with each other and CONCAT", () => {
  it('=UPPER(TRIM("  hi  ")) returns "HI"', () => {
    expect(evaluate('=UPPER(TRIM("  hi  "))', makeCtx()).value).toBe("HI");
  });

  it('=CONCAT(UPPER("a"), LOWER("B")) returns "Ab"', () => {
    expect(evaluate('=CONCAT(UPPER("a"), LOWER("B"))', makeCtx()).value).toBe(
      "Ab",
    );
  });

  it("=LEN(TRIM(A2)) returns the trimmed-string length", () => {
    expect(evaluate("=LEN(TRIM(A2))", makeCtx()).value).toBe("Hello World".length);
  });
});
