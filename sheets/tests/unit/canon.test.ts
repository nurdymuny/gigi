import { describe, expect, it } from "vitest";
import { canonicalize, canonicalMatches, trigrams } from "../../src/lib/canon";

describe("canon · canonicalize", () => {
  it("strips whitespace, dashes, slashes, underscores, dots, commas", () => {
    expect(canonicalize("INV-2026-04823")).toBe("INV202604823");
    expect(canonicalize("INV 2026 04823")).toBe("INV202604823");
    expect(canonicalize("INV.2026.04823")).toBe("INV202604823");
    expect(canonicalize("INV_2026_04823")).toBe("INV202604823");
    expect(canonicalize("INV/2026/04823")).toBe("INV202604823");
    expect(canonicalize("INV,2026,04823")).toBe("INV202604823");
  });

  it("uppercases", () => {
    expect(canonicalize("inv-2026-04823")).toBe("INV202604823");
    expect(canonicalize("Invoice 8821")).toBe("INVOICE8821");
  });

  it("is idempotent: canon(canon(s)) === canon(s)", () => {
    const samples = [
      "INV-2026-04823",
      "supplier payment",
      "LC 2026 77432 drawdown",
      "  leading/trailing  ",
      "Mixed_Case-With.Punct,Stuff",
    ];
    for (const s of samples) {
      const once = canonicalize(s);
      const twice = canonicalize(once);
      expect(twice).toBe(once);
    }
  });

  it("returns empty string for empty input", () => {
    expect(canonicalize("")).toBe("");
  });

  it("preserves digits and letters unchanged after uppercase", () => {
    expect(canonicalize("abc123")).toBe("ABC123");
  });

  it("handles unicode pass-through (non-ascii letters remain)", () => {
    expect(canonicalize("café 2026")).toBe("CAFÉ2026");
  });
});

describe("canon · canonicalMatches", () => {
  it("matches the same reference in different formats", () => {
    expect(canonicalMatches("INV-2026-04823", "INV 2026 04823")).toBe(true);
    expect(canonicalMatches("supplier payment", "supplier-payment")).toBe(true);
    expect(canonicalMatches("Q1 dividend", "q1.dividend")).toBe(true);
  });

  it("returns false for genuinely different references", () => {
    expect(canonicalMatches("INV-2026-04823", "INV-2026-04824")).toBe(false);
    expect(canonicalMatches("Q1 dividend", "Q2 dividend")).toBe(false);
  });

  it("returns true for empty inputs (both empty)", () => {
    expect(canonicalMatches("", "")).toBe(true);
  });

  it("returns false when one side is empty and the other is not", () => {
    expect(canonicalMatches("anything", "")).toBe(false);
  });
});

describe("canon · trigrams", () => {
  it("generates overlapping 3-character windows after canonicalization", () => {
    expect(trigrams("INV20")).toEqual(["INV", "NV2", "V20"]);
  });

  it("returns empty array for strings shorter than 3 chars (after canon)", () => {
    expect(trigrams("AB")).toEqual([]);
    expect(trigrams("")).toEqual([]);
    expect(trigrams(" - ")).toEqual([]); // canonicalizes to ""
  });

  it("canonicalizes before windowing", () => {
    // "a-b-c" canonicalizes to "ABC" which is exactly one trigram.
    expect(trigrams("a-b-c")).toEqual(["ABC"]);
  });
});
