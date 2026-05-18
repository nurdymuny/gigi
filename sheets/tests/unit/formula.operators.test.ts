import { describe, expect, it } from "vitest";
import { evaluate, type FormulaContext } from "../../src/lib/formula";

/**
 * Phase 1.B · operator + tokenizer extensions:
 *   - String literal "" escape
 *   - Power ^ (right-assoc)
 *   - Postfix % (0.5% → 0.005)
 *   - Comparison ops: = <> < <= > >=
 *   - & string concatenation operator
 *
 * See FORMULAS_SPEC.md §"Operators" table.
 */

function makeCtx(): FormulaContext {
  const cells: Record<string, number | string | null> = {
    A1: 10, A2: 20, A3: 5,
    B1: "hello", B2: "world",
  };
  return {
    cell: (ref) => (ref in cells ? cells[ref] : null),
    sameness: () => 0.5,
    kappa: () => 0,
    cohort: () => "",
  };
}

describe("formula · string literal \"\" escape", () => {
  it('parses "she said ""hi""" as: she said "hi"', () => {
    expect(evaluate('="she said ""hi"""', makeCtx()).value).toBe('she said "hi"');
  });
  it('a single escaped quote: "" inside literal → " (4 quotes total)', () => {
    // `=""""` → open " + escape "" + close " → result is a single "
    expect(evaluate('=""""', makeCtx()).value).toBe('"');
  });
  it("empty string", () => {
    expect(evaluate('=""', makeCtx()).value).toBe("");
  });
  it("plain string with no quotes inside", () => {
    expect(evaluate('="hello"', makeCtx()).value).toBe("hello");
  });
});

describe("formula · & (string concat operator)", () => {
  it("concatenates two string literals", () => {
    expect(evaluate('="foo" & "bar"', makeCtx()).value).toBe("foobar");
  });
  it("concatenates string + number (coerces number to string)", () => {
    expect(evaluate('="count = " & 42', makeCtx()).value).toBe("count = 42");
  });
  it("concatenates with cell refs", () => {
    expect(evaluate('=B1 & " " & B2', makeCtx()).value).toBe("hello world");
  });
  it("equivalent to CONCAT", () => {
    const a = evaluate('="a" & "b" & "c"', makeCtx()).value;
    const b = evaluate('=CONCAT("a", "b", "c")', makeCtx()).value;
    expect(a).toBe(b);
  });
});

describe("formula · power ^ (right-assoc)", () => {
  it("=2^3 returns 8", () => {
    expect(evaluate("=2^3", makeCtx()).value).toBe(8);
  });
  it("=2^0 returns 1", () => {
    expect(evaluate("=2^0", makeCtx()).value).toBe(1);
  });
  it("=4^0.5 returns 2 (square root via power)", () => {
    expect(evaluate("=4^0.5", makeCtx()).value).toBe(2);
  });
  it("right-associative: 2^3^2 = 2^(3^2) = 512, not (2^3)^2 = 64", () => {
    expect(evaluate("=2^3^2", makeCtx()).value).toBe(512);
  });
  it("precedence over multiplication: 2*3^2 = 2*9 = 18", () => {
    expect(evaluate("=2*3^2", makeCtx()).value).toBe(18);
  });
});

describe("formula · postfix % (NOT modulo)", () => {
  it("=5% returns 0.05", () => {
    expect(evaluate("=5%", makeCtx()).value).toBe(0.05);
  });
  it("=100% returns 1", () => {
    expect(evaluate("=100%", makeCtx()).value).toBe(1);
  });
  it("works on a cell ref: =A1% returns 0.1 (10/100)", () => {
    expect(evaluate("=A1%", makeCtx()).value).toBe(0.1);
  });
  it("combines with arithmetic: =1 + 50% returns 1.5", () => {
    expect(evaluate("=1 + 50%", makeCtx()).value).toBe(1.5);
  });
  it("modulo lives in MOD() — = 5 % 2 should NOT parse as 5 mod 2", () => {
    // Modulo on the operator slot is intentionally not supported.
    // Whether it's #ERROR! or the user-typed "5%" expression followed
    // by "2" depends on parser cleanup; either way it must NOT be 1.
    const r = evaluate("=5 % 2", makeCtx());
    expect(r.value).not.toBe(1);
  });
});

describe("formula · comparison operators", () => {
  it("= : equality", () => {
    expect(evaluate("=1=1", makeCtx()).value).toBe(true);
    expect(evaluate("=1=2", makeCtx()).value).toBe(false);
  });
  it("<> : inequality", () => {
    expect(evaluate("=1<>2", makeCtx()).value).toBe(true);
    expect(evaluate("=1<>1", makeCtx()).value).toBe(false);
  });
  it("< : less than", () => {
    expect(evaluate("=1<2", makeCtx()).value).toBe(true);
    expect(evaluate("=2<1", makeCtx()).value).toBe(false);
    expect(evaluate("=1<1", makeCtx()).value).toBe(false);
  });
  it("> : greater than", () => {
    expect(evaluate("=2>1", makeCtx()).value).toBe(true);
    expect(evaluate("=1>2", makeCtx()).value).toBe(false);
  });
  it("<= : less or equal", () => {
    expect(evaluate("=1<=1", makeCtx()).value).toBe(true);
    expect(evaluate("=1<=2", makeCtx()).value).toBe(true);
    expect(evaluate("=2<=1", makeCtx()).value).toBe(false);
  });
  it(">= : greater or equal", () => {
    expect(evaluate("=1>=1", makeCtx()).value).toBe(true);
    expect(evaluate("=2>=1", makeCtx()).value).toBe(true);
    expect(evaluate("=1>=2", makeCtx()).value).toBe(false);
  });
  it("string comparisons (lexicographic, case-insensitive equality)", () => {
    expect(evaluate('="apple" = "apple"', makeCtx()).value).toBe(true);
    expect(evaluate('="apple" = "APPLE"', makeCtx()).value).toBe(true);
    expect(evaluate('="apple" < "banana"', makeCtx()).value).toBe(true);
  });
  it("comparisons feed into IF", () => {
    expect(evaluate('=IF(A1 > 5, "big", "small")', makeCtx()).value).toBe("big");
    expect(evaluate('=IF(A3 > 5, "big", "small")', makeCtx()).value).toBe(
      "small",
    );
  });
  it("comparison precedence below arithmetic: =1+1 > 1 is (1+1) > 1 = true", () => {
    expect(evaluate("=1+1 > 1", makeCtx()).value).toBe(true);
    expect(evaluate("=2*3 = 6", makeCtx()).value).toBe(true);
  });
});
