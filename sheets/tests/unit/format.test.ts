import { describe, expect, it } from "vitest";
import {
  defaultFormatFor,
  formatNumber,
  formatValue,
  parseFormatString,
} from "../../src/lib/format";

describe("format · formatNumber", () => {
  it("two-decimal places", () => {
    expect(formatNumber(1234.5, "0.00")).toBe("1234.50");
    expect(formatNumber(1234, "0.00")).toBe("1234.00");
  });

  it("comma-grouped thousands", () => {
    expect(formatNumber(1234567.89, "#,##0.00")).toBe("1,234,567.89");
    expect(formatNumber(1234, "#,##0")).toBe("1,234");
  });

  it("dollar prefix in literal", () => {
    expect(formatNumber(250, "$#,##0.00")).toBe("$250.00");
    expect(formatNumber(1234.5, "$ #,##0.00")).toBe("$ 1,234.50");
  });

  it("negatives keep their sign", () => {
    expect(formatNumber(-99.5, "0.00")).toBe("-99.50");
  });

  it("zero renders correctly", () => {
    expect(formatNumber(0, "0.00")).toBe("0.00");
  });

  it("non-finite renders as empty string", () => {
    expect(formatNumber(NaN, "0.00")).toBe("");
    expect(formatNumber(Infinity, "0.00")).toBe("");
  });
});

describe("format · defaultFormatFor", () => {
  it("USD-shaped column → currency default", () => {
    expect(defaultFormatFor({ name: "amount_usd", type: "numeric" })).toBe(
      "$#,##0.00",
    );
    expect(defaultFormatFor({ name: "total_usd", type: "numeric" })).toBe(
      "$#,##0.00",
    );
  });

  it("date-shaped column → ISO date default", () => {
    expect(defaultFormatFor({ name: "iso_date", type: "timestamp" })).toBe(
      "YYYY-MM-DD",
    );
    expect(defaultFormatFor({ name: "post_date", type: "timestamp" })).toBe(
      "YYYY-MM-DD",
    );
  });

  it("percent-shaped column → percentage default", () => {
    expect(defaultFormatFor({ name: "growth_pct", type: "numeric" })).toBe(
      "0.0%",
    );
  });

  it("returns null when nothing matches", () => {
    expect(defaultFormatFor({ name: "random", type: "numeric" })).toBeNull();
  });
});

describe("format · parseFormatString (κ extension)", () => {
  it("strips the [κ>τ] prefix and returns it as a condition", () => {
    const parsed = parseFormatString('[κ>0.3]"⚠️ "0.00');
    expect(parsed.condition).toEqual({ kind: "kappa-gt", threshold: 0.3 });
    // The body retains the literal prefix and the number-format core.
    expect(parsed.body).toBe('"⚠️ "0.00');
  });

  it("returns null condition when no [κ...] prefix is present", () => {
    const parsed = parseFormatString("0.00");
    expect(parsed.condition).toBeNull();
    expect(parsed.body).toBe("0.00");
  });
});

describe("format · formatValue", () => {
  it("renders a USD value with the implicit default", () => {
    const out = formatValue(250, "$#,##0.00", { kappa: 0 });
    expect(out).toBe("$250.00");
  });

  it("applies the κ-conditional prefix when κ exceeds the threshold", () => {
    const out = formatValue(120, '[κ>0.3]"⚠️ "0.00', { kappa: 0.42 });
    expect(out).toBe("⚠️ 120.00");
  });

  it("does NOT apply the conditional prefix below the threshold", () => {
    const out = formatValue(120, '[κ>0.3]"⚠️ "0.00', { kappa: 0.05 });
    expect(out).toBe("120.00"); // the literal prefix only applies under the condition
  });

  it("string value renders unchanged", () => {
    const out = formatValue("hello", "0.00", { kappa: 0 });
    expect(out).toBe("hello");
  });
});
