import { describe, expect, it } from "vitest";
import { evaluate, type FormulaContext } from "../../src/lib/formula";

/**
 * Phase 1.5.D · date functions.
 *
 * GIGI uses Excel's serial-day model: dates are NUMBERS (days since
 * Unix epoch 1970-01-01 UTC). `=date + 1` adds a day naturally. The
 * formula engine doesn't have a distinct "date" type — it leans on the
 * `TO_DATE` family to convert ISO strings / epoch numbers into serial
 * days that arithmetic and YEAR/MONTH/DAY can both work with.
 *
 * Conversions:
 *   2024-01-01 UTC → 19723 (days since 1970-01-01)
 *   1970-01-01 UTC → 0
 *   1970-01-02 UTC → 1
 *
 * `TODAY()` is **deterministic per evaluation**: the test injects a
 * fixed `today()` so assertions don't depend on wall-clock time.
 */

/** Compute serial day number from a `YYYY-MM-DD` string (test helper). */
function serialDay(iso: string): number {
  return Math.floor(Date.UTC(...isoToTuple(iso)) / 86400000);
}
function isoToTuple(iso: string): [number, number, number] {
  const [y, m, d] = iso.split("-").map(Number);
  return [y, m - 1, d];
}

function makeCtx(today = "2026-05-16"): FormulaContext {
  return {
    cell: () => null,
    sameness: () => 0.5,
    kappa: () => 0,
    cohort: () => "",
    today: () => serialDay(today),
  };
}

describe("formula · TO_DATE", () => {
  it('=TO_DATE("1970-01-01") returns 0 (epoch)', () => {
    expect(evaluate('=TO_DATE("1970-01-01")', makeCtx()).value).toBe(0);
  });

  it('=TO_DATE("2024-01-01") returns the correct serial day', () => {
    expect(evaluate('=TO_DATE("2024-01-01")', makeCtx()).value).toBe(
      serialDay("2024-01-01"),
    );
  });

  it('=TO_DATE("2026-05-16T12:34:56Z") accepts RFC 3339 timestamps', () => {
    // Time portion is truncated to the date (UTC).
    expect(evaluate('=TO_DATE("2026-05-16T12:34:56Z")', makeCtx()).value).toBe(
      serialDay("2026-05-16"),
    );
  });

  it("=TO_DATE(epoch_seconds 10-digit) converts to serial days", () => {
    // 1700000000 s = 2023-11-14T22:13:20Z → serial day for 2023-11-14
    expect(evaluate("=TO_DATE(1700000000)", makeCtx()).value).toBe(
      serialDay("2023-11-14"),
    );
  });

  it("=TO_DATE(epoch_milliseconds 13-digit) converts to serial days", () => {
    // 1700000000000 ms = same instant as above
    expect(evaluate("=TO_DATE(1700000000000)", makeCtx()).value).toBe(
      serialDay("2023-11-14"),
    );
  });

  it("=TO_DATE(numeric not 10 or 13 digits) returns #VALUE!", () => {
    // 9-digit number: ambiguous, neither s nor ms
    expect(evaluate("=TO_DATE(123456789)", makeCtx()).error).toBe("#VALUE!");
  });

  it('=TO_DATE("not a date") returns #VALUE!', () => {
    expect(evaluate('=TO_DATE("not a date")', makeCtx()).error).toBe("#VALUE!");
  });
});

describe("formula · YEAR / MONTH / DAY (UTC components)", () => {
  it("=YEAR(TO_DATE('2024-03-15')) returns 2024", () => {
    expect(evaluate('=YEAR(TO_DATE("2024-03-15"))', makeCtx()).value).toBe(2024);
  });

  it("=MONTH(TO_DATE('2024-03-15')) returns 3", () => {
    expect(evaluate('=MONTH(TO_DATE("2024-03-15"))', makeCtx()).value).toBe(3);
  });

  it("=DAY(TO_DATE('2024-03-15')) returns 15", () => {
    expect(evaluate('=DAY(TO_DATE("2024-03-15"))', makeCtx()).value).toBe(15);
  });

  it("=YEAR(0) returns 1970 (epoch)", () => {
    expect(evaluate("=YEAR(0)", makeCtx()).value).toBe(1970);
  });

  it("=YEAR / MONTH / DAY also accept a string date directly", () => {
    expect(evaluate('=YEAR("2024-03-15")', makeCtx()).value).toBe(2024);
    expect(evaluate('=MONTH("2024-03-15")', makeCtx()).value).toBe(3);
    expect(evaluate('=DAY("2024-03-15")', makeCtx()).value).toBe(15);
  });
});

describe("formula · TODAY (deterministic per evaluation)", () => {
  it("=TODAY() returns the injected today as a serial day", () => {
    expect(evaluate("=TODAY()", makeCtx("2026-05-16")).value).toBe(
      serialDay("2026-05-16"),
    );
  });

  it("=YEAR(TODAY()) round-trips through the date component fns", () => {
    expect(evaluate("=YEAR(TODAY())", makeCtx("2030-12-31")).value).toBe(2030);
    expect(evaluate("=MONTH(TODAY())", makeCtx("2030-12-31")).value).toBe(12);
    expect(evaluate("=DAY(TODAY())", makeCtx("2030-12-31")).value).toBe(31);
  });
});

describe("formula · date arithmetic (Excel serial-day model)", () => {
  it("=TO_DATE('2024-01-01') + 30 returns the serial day for Jan 31, 2024", () => {
    expect(evaluate('=TO_DATE("2024-01-01") + 30', makeCtx()).value).toBe(
      serialDay("2024-01-31"),
    );
  });

  it("=TO_DATE('2024-02-01') - TO_DATE('2024-01-01') returns 31", () => {
    expect(
      evaluate('=TO_DATE("2024-02-01") - TO_DATE("2024-01-01")', makeCtx()).value,
    ).toBe(31);
  });

  it("=DAY(TO_DATE('2024-01-31') + 1) returns 1 (rolls into Feb)", () => {
    expect(evaluate('=DAY(TO_DATE("2024-01-31") + 1)', makeCtx()).value).toBe(1);
    expect(evaluate('=MONTH(TO_DATE("2024-01-31") + 1)', makeCtx()).value).toBe(2);
  });
});

describe("formula · DATEDIF (lowercase units, GIGI divergence from Excel)", () => {
  it('=DATEDIF(jan1, jan31, "d") returns 30', () => {
    expect(
      evaluate('=DATEDIF(TO_DATE("2024-01-01"), TO_DATE("2024-01-31"), "d")', makeCtx()).value,
    ).toBe(30);
  });

  it('=DATEDIF("2024-01-01", "2024-01-15", "w") returns 2 (whole weeks)', () => {
    // 14 days / 7 = 2
    expect(
      evaluate('=DATEDIF("2024-01-01", "2024-01-15", "w")', makeCtx()).value,
    ).toBe(2);
  });

  it('=DATEDIF("2024-01-15", "2024-04-15", "m") returns 3 (whole months)', () => {
    expect(
      evaluate('=DATEDIF("2024-01-15", "2024-04-15", "m")', makeCtx()).value,
    ).toBe(3);
  });

  it('=DATEDIF("2020-06-01", "2024-06-01", "y") returns 4 (whole years)', () => {
    expect(
      evaluate('=DATEDIF("2020-06-01", "2024-06-01", "y")', makeCtx()).value,
    ).toBe(4);
  });

  it('=DATEDIF accounts for partial months / years (rounds down)', () => {
    // 2024-01-15 to 2024-04-14 is 2 whole months (15th hasn't reached)
    expect(
      evaluate('=DATEDIF("2024-01-15", "2024-04-14", "m")', makeCtx()).value,
    ).toBe(2);
  });

  it('=DATEDIF with uppercase "Y" returns #VALUE! (Excel divergence)', () => {
    // Per FORMULAS_SPEC §"DATEDIF unit divergence from Excel": we deliberately
    // reject uppercase units rather than silently doing the wrong thing.
    expect(
      evaluate('=DATEDIF("2020-01-01", "2024-01-01", "Y")', makeCtx()).error,
    ).toBe("#VALUE!");
  });

  it('=DATEDIF with bogus unit returns #VALUE!', () => {
    expect(
      evaluate('=DATEDIF("2024-01-01", "2024-02-01", "x")', makeCtx()).error,
    ).toBe("#VALUE!");
  });

  it("=DATEDIF returns negative when end < start", () => {
    // No Excel-style swap; the user asked for end-start in days.
    expect(
      evaluate('=DATEDIF("2024-01-31", "2024-01-01", "d")', makeCtx()).value,
    ).toBe(-30);
  });
});
