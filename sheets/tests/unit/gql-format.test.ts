import { describe, expect, it } from "vitest";
import { formatGql } from "../../src/lib/gql-format";

describe("formatGql — basic shape", () => {
  it("returns the empty string for empty input", () => {
    expect(formatGql("")).toBe("");
    expect(formatGql("   \n\n  ")).toBe("");
  });

  it("formats a SECTION + WHERE + ORDER BY + LIMIT into one clause per line", () => {
    const q = "SECTION sensors WHERE site_id='North-3' ORDER BY κ DESC LIMIT 25;";
    expect(formatGql(q)).toBe(
      "SECTION sensors\nWHERE site_id='North-3'\nORDER BY κ DESC\nLIMIT 25;",
    );
  });

  it("indents AND continuations of a WHERE clause by two spaces", () => {
    const q =
      "SECTION sensors WHERE site_id='N' AND temp > 30 AND status='ok' LIMIT 5;";
    const out = formatGql(q);
    expect(out).toContain("WHERE site_id='N'");
    expect(out).toMatch(/\n {2}AND temp > 30/);
    expect(out).toMatch(/\n {2}AND status='ok'/);
  });

  it("indents OR continuations by two spaces too", () => {
    const q = "SECTION s WHERE x=1 OR x=2;";
    expect(formatGql(q)).toBe("SECTION s\nWHERE x=1\n  OR x=2;");
  });

  it("preserves the trailing semicolon if present", () => {
    expect(formatGql("SECTION sensors;").endsWith(";")).toBe(true);
    expect(formatGql("SECTION sensors").endsWith(";")).toBe(false);
  });

  it("canonicalizes clause keywords to uppercase but leaves identifiers alone", () => {
    const q = "section sensors where x=1;";
    expect(formatGql(q)).toBe("SECTION sensors\nWHERE x=1;");
  });
});

describe("formatGql — string-literal safety", () => {
  it("does NOT split on a WHERE inside a string literal", () => {
    const q = "SECTION sensors WHERE note='WHERE the wild things are';";
    const out = formatGql(q);
    expect(out).toBe("SECTION sensors\nWHERE note='WHERE the wild things are';");
    // Only one WHERE-prefixed line.
    expect(out.split("\nWHERE ")).toHaveLength(2);
  });

  it("handles escaped quotes ('') inside string literals", () => {
    const q = "SECTION sensors WHERE name='O''Brien' LIMIT 1;";
    const out = formatGql(q);
    expect(out).toBe("SECTION sensors\nWHERE name='O''Brien'\nLIMIT 1;");
  });

  it("does NOT split on ORDER BY inside a string", () => {
    const q = "SECTION x WHERE note='ORDER BY something';";
    expect(formatGql(q)).toBe("SECTION x\nWHERE note='ORDER BY something';");
  });
});

describe("formatGql — idempotency", () => {
  const cases = [
    "SECTION sensors WHERE site_id='N' ORDER BY κ DESC LIMIT 25;",
    "SECTION sensors WHERE x=1 AND y=2 AND z=3;",
    "SPECTRAL sensors ON FIBER (temp, humidity) MODES 3;",
    "TRANSPORT sensors FROM (sensor_id='S-001') TO (sensor_id='S-002') ON FIBER (temp, humidity);",
    "HOLONOMY sensors ON FIBER (temp, humidity) AROUND site_id;",
    "INTEGRATE temp OVER sensors COVER ALL;",
    "BETTI sensors;",
    "SECTION sensors WHERE note='WHERE i am' OR note='ORDER BY love';",
    "",
    "   SECTION   sensors    ;   ",
  ];
  for (const q of cases) {
    it(`is idempotent: format(format(${JSON.stringify(q.slice(0, 40))})) === format(...)`, () => {
      const once = formatGql(q);
      const twice = formatGql(once);
      expect(twice).toBe(once);
    });
  }
});

describe("formatGql — multi-word clause keywords win over substrings", () => {
  it("splits ON FIBER as one unit, never just ON or FIBER", () => {
    const q = "SPECTRAL sensors ON FIBER (temp, humidity) MODES 3;";
    expect(formatGql(q)).toBe(
      "SPECTRAL sensors\nON FIBER (temp, humidity)\nMODES 3;",
    );
  });

  it("splits ORDER BY as one unit, never just BY", () => {
    const q = "SECTION s ORDER BY x DESC;";
    expect(formatGql(q)).toBe("SECTION s\nORDER BY x DESC;");
  });
});
