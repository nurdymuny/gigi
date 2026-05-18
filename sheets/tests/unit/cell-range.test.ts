import { describe, expect, it } from "vitest";
import {
  cellsInRange,
  isCellInRange,
  rangeRowKeys,
  rangeFields,
  type CellRange,
} from "../../src/lib/cell-range";

/**
 * Range-selection geometry — pure helpers, no React.
 *
 * `CellRange` encodes (anchor cell, focus cell) — the topology Excel
 * uses for marching ants. The anchor is the cell where the drag/click
 * started; the focus is where it currently sits. Normalization
 * (top-left / bottom-right) happens in the helpers so callers don't
 * have to think about drag direction.
 */

const ROW_ORDER = ["R1", "R2", "R3", "R4", "R5"];
const FIELD_ORDER = ["A", "B", "C", "D"];

function range(
  anchorRow: string,
  anchorField: string,
  focusRow: string,
  focusField: string,
): CellRange {
  return { anchorRowKey: anchorRow, anchorField, focusRowKey: focusRow, focusField };
}

describe("isCellInRange · single cell range (degenerate)", () => {
  it("only the anchor itself is in range when anchor === focus", () => {
    const r = range("R2", "B", "R2", "B");
    expect(isCellInRange(r, "R2", "B", ROW_ORDER, FIELD_ORDER)).toBe(true);
    expect(isCellInRange(r, "R2", "C", ROW_ORDER, FIELD_ORDER)).toBe(false);
    expect(isCellInRange(r, "R3", "B", ROW_ORDER, FIELD_ORDER)).toBe(false);
  });
});

describe("isCellInRange · normalized bounding box", () => {
  it("anchor top-left, focus bottom-right", () => {
    const r = range("R2", "B", "R4", "C");
    // bbox: rows R2..R4 × cols B..C
    expect(isCellInRange(r, "R2", "B", ROW_ORDER, FIELD_ORDER)).toBe(true);
    expect(isCellInRange(r, "R3", "B", ROW_ORDER, FIELD_ORDER)).toBe(true);
    expect(isCellInRange(r, "R4", "C", ROW_ORDER, FIELD_ORDER)).toBe(true);
    expect(isCellInRange(r, "R1", "B", ROW_ORDER, FIELD_ORDER)).toBe(false);
    expect(isCellInRange(r, "R5", "B", ROW_ORDER, FIELD_ORDER)).toBe(false);
    expect(isCellInRange(r, "R3", "D", ROW_ORDER, FIELD_ORDER)).toBe(false);
  });

  it("works equally when the drag goes UP-LEFT (focus before anchor)", () => {
    // Same bbox via inverted drag: anchor at R4/C, focus at R2/B.
    const r = range("R4", "C", "R2", "B");
    expect(isCellInRange(r, "R2", "B", ROW_ORDER, FIELD_ORDER)).toBe(true);
    expect(isCellInRange(r, "R3", "B", ROW_ORDER, FIELD_ORDER)).toBe(true);
    expect(isCellInRange(r, "R4", "C", ROW_ORDER, FIELD_ORDER)).toBe(true);
    expect(isCellInRange(r, "R1", "B", ROW_ORDER, FIELD_ORDER)).toBe(false);
  });

  it("a row outside the bundle's visible order is not in range", () => {
    const r = range("R1", "A", "R5", "D");
    expect(isCellInRange(r, "ROGUE", "A", ROW_ORDER, FIELD_ORDER)).toBe(false);
    expect(isCellInRange(r, "R1", "ZZ", ROW_ORDER, FIELD_ORDER)).toBe(false);
  });

  it("missing range or empty axes returns false (no crash)", () => {
    expect(isCellInRange(null, "R1", "A", ROW_ORDER, FIELD_ORDER)).toBe(false);
    const r = range("R1", "A", "R2", "B");
    expect(isCellInRange(r, "R1", "A", [], FIELD_ORDER)).toBe(false);
    expect(isCellInRange(r, "R1", "A", ROW_ORDER, [])).toBe(false);
  });
});

describe("cellsInRange · enumeration of (rowKey, field) pairs", () => {
  it("returns every cell in the bbox, in row-major order", () => {
    const r = range("R2", "B", "R3", "C");
    const cells = cellsInRange(r, ROW_ORDER, FIELD_ORDER);
    expect(cells).toEqual([
      { rowKey: "R2", field: "B" },
      { rowKey: "R2", field: "C" },
      { rowKey: "R3", field: "B" },
      { rowKey: "R3", field: "C" },
    ]);
  });

  it("single-cell range returns one cell", () => {
    const r = range("R2", "B", "R2", "B");
    expect(cellsInRange(r, ROW_ORDER, FIELD_ORDER)).toEqual([
      { rowKey: "R2", field: "B" },
    ]);
  });

  it("returns empty for null or out-of-bounds anchors", () => {
    expect(cellsInRange(null, ROW_ORDER, FIELD_ORDER)).toEqual([]);
    const bogus = range("ROGUE", "B", "R2", "B");
    expect(cellsInRange(bogus, ROW_ORDER, FIELD_ORDER)).toEqual([]);
  });
});

describe("rangeRowKeys / rangeFields · axis projections", () => {
  it("rangeRowKeys returns rows in the normalized range, top-to-bottom", () => {
    const r = range("R4", "C", "R2", "B"); // inverted drag
    expect(rangeRowKeys(r, ROW_ORDER)).toEqual(["R2", "R3", "R4"]);
  });

  it("rangeFields returns columns in the normalized range, left-to-right", () => {
    const r = range("R4", "D", "R2", "A"); // inverted drag
    expect(rangeFields(r, FIELD_ORDER)).toEqual(["A", "B", "C", "D"]);
  });

  it("null range → empty arrays", () => {
    expect(rangeRowKeys(null, ROW_ORDER)).toEqual([]);
    expect(rangeFields(null, FIELD_ORDER)).toEqual([]);
  });
});
