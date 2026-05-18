import { describe, expect, it } from "vitest";
import {
  fromBundleJson,
  fromTsv,
  toBundleJson,
  toTsv,
  validatePaste,
} from "../../src/lib/clipboard";

describe("clipboard · TSV roundtrip", () => {
  it("toTsv produces tab-separated rows with newline terminators", () => {
    const out = toTsv([
      ["a", "b", "c"],
      ["1", "2", "3"],
    ]);
    expect(out).toBe("a\tb\tc\n1\t2\t3\n");
  });

  it("roundtrips: fromTsv(toTsv(x)) === x for plain data", () => {
    const grid = [
      ["payment_id", "amount", "currency"],
      ["P-001", "250000", "USD"],
      ["P-002", "180000", "EUR"],
    ];
    expect(fromTsv(toTsv(grid))).toEqual(grid);
  });

  it("preserves empty cells", () => {
    const grid = [["a", "", "c"]];
    expect(fromTsv(toTsv(grid))).toEqual(grid);
  });

  it("handles a trailing newline gracefully (no phantom row)", () => {
    expect(fromTsv("a\tb\n1\t2\n")).toEqual([
      ["a", "b"],
      ["1", "2"],
    ]);
  });

  it("handles no trailing newline", () => {
    expect(fromTsv("a\tb\n1\t2")).toEqual([
      ["a", "b"],
      ["1", "2"],
    ]);
  });

  it("returns [] for empty input", () => {
    expect(fromTsv("")).toEqual([]);
  });

  it("normalizes CRLF to LF", () => {
    expect(fromTsv("a\tb\r\n1\t2\r\n")).toEqual([
      ["a", "b"],
      ["1", "2"],
    ]);
  });
});

describe("clipboard · bundle JSON", () => {
  it("includes columns + rows + optional bundle id", () => {
    const json = toBundleJson({
      bundle: "payment_transactions",
      columns: ["id", "amount"],
      rows: [
        { id: "P-001", amount: 250000 },
        { id: "P-002", amount: 180000 },
      ],
    });
    const parsed = JSON.parse(json);
    expect(parsed.kind).toBe("gigi.clipboard.v1");
    expect(parsed.bundle).toBe("payment_transactions");
    expect(parsed.columns).toEqual(["id", "amount"]);
    expect(parsed.rows).toHaveLength(2);
  });

  it("fromBundleJson parses and returns kind+columns+rows", () => {
    const json = toBundleJson({
      bundle: "x",
      columns: ["a", "b"],
      rows: [{ a: 1, b: 2 }],
    });
    const out = fromBundleJson(json);
    expect(out?.columns).toEqual(["a", "b"]);
    expect(out?.rows[0]).toEqual({ a: 1, b: 2 });
  });

  it("fromBundleJson returns null on invalid input", () => {
    expect(fromBundleJson("not json")).toBeNull();
    expect(fromBundleJson("{}")).toBeNull();
    expect(fromBundleJson(JSON.stringify({ kind: "wrong" }))).toBeNull();
  });
});

describe("clipboard · validatePaste", () => {
  it("flags a non-numeric value pasted into a numeric column", () => {
    const result = validatePaste(
      [["100", "potato"]],
      ["price_usd", "stock"],
      new Map([
        ["price_usd", "numeric"],
        ["stock", "numeric"],
      ]),
    );
    expect(result.warnings).toHaveLength(1);
    expect(result.warnings[0]).toMatchObject({
      row: 0,
      column: "stock",
      reason: expect.stringMatching(/numeric/i),
    });
  });

  it("returns no warnings when types align", () => {
    const result = validatePaste(
      [["100", "5"]],
      ["price_usd", "stock"],
      new Map([
        ["price_usd", "numeric"],
        ["stock", "numeric"],
      ]),
    );
    expect(result.warnings).toHaveLength(0);
  });

  it("treats empty strings as null (not a type violation)", () => {
    const result = validatePaste(
      [["", "5"]],
      ["price_usd", "stock"],
      new Map([
        ["price_usd", "numeric"],
        ["stock", "numeric"],
      ]),
    );
    expect(result.warnings).toHaveLength(0);
  });

  it("skips warnings for columns absent from the schema (unknown)", () => {
    const result = validatePaste(
      [["v"]],
      ["mystery"],
      new Map(),
    );
    expect(result.warnings).toHaveLength(0);
  });
});
