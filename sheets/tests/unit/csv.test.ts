import { describe, expect, it } from "vitest";
import { parseCsv, pickKeyField } from "../../src/lib/csv";

describe("parseCsv — basic shape", () => {
  it("parses a simple CSV with header + rows", () => {
    const r = parseCsv("id,name,age\n1,Alice,30\n2,Bob,45\n");
    expect(r.headers).toEqual(["id", "name", "age"]);
    expect(r.rows).toHaveLength(2);
    expect(r.rows[0]).toEqual({ id: 1, name: "Alice", age: 30 });
    expect(r.rows[1]).toEqual({ id: 2, name: "Bob", age: 45 });
    expect(r.delimiter).toBe(",");
  });

  it("auto-detects tabs as the delimiter", () => {
    const r = parseCsv("a\tb\n1\tx\n2\ty\n");
    expect(r.delimiter).toBe("\t");
    expect(r.headers).toEqual(["a", "b"]);
    expect(r.rows).toEqual([
      { a: 1, b: "x" },
      { a: 2, b: "y" },
    ]);
  });

  it("returns empty result for empty input", () => {
    expect(parseCsv("")).toEqual({
      headers: [],
      types: [],
      rows: [],
      delimiter: ",",
      skipped: 0,
    });
    expect(parseCsv("   \n  \n").rows).toEqual([]);
  });

  it("handles quoted fields with embedded commas", () => {
    const r = parseCsv('id,name\n1,"Smith, John"\n2,"Doe, Jane"\n');
    expect(r.rows[0].name).toBe("Smith, John");
    expect(r.rows[1].name).toBe("Doe, Jane");
  });

  it("handles doubled-quote escapes inside quoted fields", () => {
    const r = parseCsv('id,note\n1,"She said ""hi"""\n');
    expect(r.rows[0].note).toBe('She said "hi"');
  });

  it("handles CRLF line endings", () => {
    const r = parseCsv("a,b\r\n1,x\r\n2,y\r\n");
    expect(r.rows).toHaveLength(2);
  });

  it("strips a leading BOM", () => {
    const r = parseCsv("﻿id,name\n1,Alice\n");
    expect(r.headers).toEqual(["id", "name"]);
  });

  it("sanitizes unsafe header characters", () => {
    const r = parseCsv("user id,first name,age!\n1,Alice,30\n");
    expect(r.headers).toEqual(["user_id", "first_name", "age_"]);
  });

  it("prefixes leading-digit headers with _", () => {
    const r = parseCsv("2024_data,name\n1,Alice\n");
    expect(r.headers[0]).toBe("_2024_data");
  });
});

describe("parseCsv — type inference", () => {
  it("classifies numeric, categorical, and text correctly", () => {
    // 'note' has 4 unique values, under the cardinality threshold → categorical.
    const r = parseCsv(`id,city,note
1,Paris,short
2,Lagos,longer text varying
3,Paris,more text
4,Lagos,still text`);
    expect(r.types).toEqual(["numeric", "categorical", "categorical"]);
  });

  it("classifies booleans as boolean", () => {
    const r = parseCsv("id,active\n1,true\n2,false\n3,true\n");
    expect(r.types[1]).toBe("boolean");
    expect(r.rows[0].active).toBe(true);
    expect(r.rows[1].active).toBe(false);
  });

  it("classifies ISO timestamps as timestamp", () => {
    const r = parseCsv("id,at\n1,2026-01-15T10:30:00Z\n2,2026-02-20T14:00:00Z\n");
    expect(r.types[1]).toBe("timestamp");
  });

  it("falls back to text when cardinality is high", () => {
    const lines = ["id,note"];
    for (let i = 0; i < 30; i++) lines.push(`${i},unique-${i}`);
    const r = parseCsv(lines.join("\n"));
    expect(r.types[1]).toBe("text");
  });
});

describe("parseCsv — value coercion", () => {
  it("coerces numeric columns to numbers", () => {
    const r = parseCsv("id,price\n1,9.99\n2,123\n");
    expect(typeof r.rows[0].price).toBe("number");
    expect(r.rows[0].price).toBeCloseTo(9.99);
  });

  it("preserves strings for text/categorical columns", () => {
    const r = parseCsv("id,city\n1,Paris\n2,Lagos\n");
    expect(typeof r.rows[0].city).toBe("string");
  });

  it("converts empty strings in numeric columns to null", () => {
    const r = parseCsv("id,age\n1,30\n2,\n3,45\n");
    expect(r.rows[1].age).toBeNull();
  });
});

describe("pickKeyField", () => {
  it("picks 'id' when present", () => {
    const r = parseCsv("name,id,age\nAlice,1,30\nBob,2,45\n");
    expect(pickKeyField(r.headers, r.rows)).toBe("id");
  });

  it("picks 'uuid' when present", () => {
    const r = parseCsv("uuid,name\nabc-123,Alice\nxyz-456,Bob\n");
    expect(pickKeyField(r.headers, r.rows)).toBe("uuid");
  });

  it("picks the first column when it has unique values", () => {
    const r = parseCsv("slug,city\nparis,Paris\nlagos,Lagos\n");
    expect(pickKeyField(r.headers, r.rows)).toBe("slug");
  });

  it("returns null when no column is a clean key", () => {
    const r = parseCsv("city,country\nParis,FR\nParis,FR\nLagos,NG\n");
    expect(pickKeyField(r.headers, r.rows)).toBeNull();
  });

  it("returns null for empty headers", () => {
    expect(pickKeyField([], [])).toBeNull();
  });
});
