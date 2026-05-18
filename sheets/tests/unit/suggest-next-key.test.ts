import { describe, expect, it } from "vitest";
import { suggestNextKey } from "../../src/components/InsertRowModal";

describe("suggestNextKey · numeric keys", () => {
  it("returns '1' on an empty bundle", () => {
    expect(suggestNextKey([], "id", "numeric")).toBe("1");
  });

  it("returns max+1 of existing numeric keys", () => {
    const rows = [{ id: 1 }, { id: 2 }, { id: 5 }, { id: 3 }];
    expect(suggestNextKey(rows, "id", "numeric")).toBe("6");
  });

  it("skips non-numeric / null values", () => {
    const rows = [{ id: 1 }, { id: null }, { id: "ghost" }, { id: 4 }];
    expect(suggestNextKey(rows, "id", "numeric")).toBe("5");
  });
});

describe("suggestNextKey · text keys with prefix-digit pattern", () => {
  it("increments the trailing digit block preserving zero-pad width", () => {
    const rows = [{ id: "T-001" }, { id: "T-002" }, { id: "T-003" }];
    expect(suggestNextKey(rows, "id", "text")).toBe("T-004");
  });

  it("survives jumps in the sequence — picks max + 1", () => {
    const rows = [{ id: "T-001" }, { id: "T-005" }, { id: "T-010" }];
    expect(suggestNextKey(rows, "id", "text")).toBe("T-011");
  });

  it("rolls past the pad-width when needed (T-009 → T-010)", () => {
    const rows = [{ id: "T-009" }];
    expect(suggestNextKey(rows, "id", "text")).toBe("T-010");
  });

  it("handles long prefixes (INV-2026-04823 → INV-2026-04824)", () => {
    const rows = [{ id: "INV-2026-04823" }];
    expect(suggestNextKey(rows, "id", "text")).toBe("INV-2026-04824");
  });
});

describe("suggestNextKey · text keys without a digit pattern", () => {
  it("returns empty string when keys are pure text", () => {
    const rows = [{ id: "alpha" }, { id: "bravo" }];
    expect(suggestNextKey(rows, "id", "text")).toBe("");
  });

  it("returns empty string on empty bundle for text keys", () => {
    expect(suggestNextKey([], "id", "text")).toBe("");
  });
});

describe("suggestNextKey · edge cases", () => {
  it("ignores rows with null/missing key values", () => {
    const rows = [{ id: null }, { id: "T-001" }];
    expect(suggestNextKey(rows, "id", "text")).toBe("T-002");
  });

  it("does not crash on numeric keys with NaN / Infinity", () => {
    const rows = [{ id: 1 }, { id: NaN }, { id: Infinity }, { id: 3 }];
    expect(suggestNextKey(rows, "id", "numeric")).toBe("4");
  });
});
