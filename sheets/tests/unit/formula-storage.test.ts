import { afterEach, beforeEach, describe, expect, it } from "vitest";
import {
  clearBundleFormulas,
  clearFormula,
  getFormula,
  listFormulas,
  setFormula,
} from "../../src/lib/formula-storage";

/**
 * Phase 1.E · sidecar formula storage.
 *
 * Per FORMULAS_SPEC §"Formula cell semantics":
 *   - The bundle row stores the *displayed value* (the evaluated result).
 *   - The *formula text* lives in a sidecar map keyed by
 *     (bundleName, rowKey, fieldName), persisted to localStorage.
 *
 * vitest's jsdom environment provides a real `localStorage`, so we
 * don't need to mock it — just clear it between tests.
 */

beforeEach(() => {
  if (typeof localStorage !== "undefined") localStorage.clear();
});

afterEach(() => {
  if (typeof localStorage !== "undefined") localStorage.clear();
});

describe("formula-storage · get / set / clear", () => {
  it("returns null when no formula is stored", () => {
    expect(getFormula("b", "r1", "amount")).toBeNull();
  });

  it("stores and retrieves a formula", () => {
    setFormula("b", "r1", "amount", "=A1+B1");
    expect(getFormula("b", "r1", "amount")).toBe("=A1+B1");
  });

  it("overwrites a previously stored formula", () => {
    setFormula("b", "r1", "amount", "=A1");
    setFormula("b", "r1", "amount", "=B1");
    expect(getFormula("b", "r1", "amount")).toBe("=B1");
  });

  it("clearFormula removes the entry", () => {
    setFormula("b", "r1", "amount", "=A1");
    clearFormula("b", "r1", "amount");
    expect(getFormula("b", "r1", "amount")).toBeNull();
  });

  it("clearFormula on a non-existent entry is a no-op", () => {
    expect(() => clearFormula("b", "rX", "nope")).not.toThrow();
  });
});

describe("formula-storage · only stores formulas (leading =)", () => {
  it("setting a non-formula string clears the slot", () => {
    setFormula("b", "r1", "amount", "=A1");
    setFormula("b", "r1", "amount", "plain text");
    expect(getFormula("b", "r1", "amount")).toBeNull();
  });

  it("setting an empty string clears the slot", () => {
    setFormula("b", "r1", "amount", "=A1");
    setFormula("b", "r1", "amount", "");
    expect(getFormula("b", "r1", "amount")).toBeNull();
  });
});

describe("formula-storage · keying isolates bundles / rows / fields", () => {
  it("different bundles do not collide", () => {
    setFormula("b1", "r1", "amount", "=1");
    setFormula("b2", "r1", "amount", "=2");
    expect(getFormula("b1", "r1", "amount")).toBe("=1");
    expect(getFormula("b2", "r1", "amount")).toBe("=2");
  });

  it("different rows do not collide", () => {
    setFormula("b", "r1", "amount", "=A1");
    setFormula("b", "r2", "amount", "=A2");
    expect(getFormula("b", "r1", "amount")).toBe("=A1");
    expect(getFormula("b", "r2", "amount")).toBe("=A2");
  });

  it("different fields do not collide", () => {
    setFormula("b", "r1", "amount", "=A1");
    setFormula("b", "r1", "tax", "=B1");
    expect(getFormula("b", "r1", "amount")).toBe("=A1");
    expect(getFormula("b", "r1", "tax")).toBe("=B1");
  });
});

describe("formula-storage · listFormulas", () => {
  it("returns [] when empty", () => {
    expect(listFormulas()).toEqual([]);
  });

  it("returns all entries when no bundle is passed", () => {
    setFormula("b1", "r1", "amount", "=1");
    setFormula("b2", "r2", "tax", "=2");
    const all = listFormulas();
    expect(all).toHaveLength(2);
  });

  it("filters by bundle", () => {
    setFormula("b1", "r1", "amount", "=1");
    setFormula("b2", "r1", "amount", "=2");
    const onlyB1 = listFormulas("b1");
    expect(onlyB1).toHaveLength(1);
    expect(onlyB1[0]).toMatchObject({ bundle: "b1", rowKey: "r1", field: "amount", text: "=1" });
  });
});

describe("formula-storage · clearBundleFormulas", () => {
  it("removes all entries for the named bundle, leaves others", () => {
    setFormula("b1", "r1", "amount", "=1");
    setFormula("b1", "r2", "tax", "=2");
    setFormula("b2", "r1", "amount", "=3");
    clearBundleFormulas("b1");
    expect(listFormulas("b1")).toEqual([]);
    expect(listFormulas("b2")).toHaveLength(1);
  });
});

describe("formula-storage · key escaping (separator safety)", () => {
  it("rowKey containing the field separator does not collide", () => {
    // If the storage joins (bundle, rowKey, field) with `:` it must
    // escape; otherwise a rowKey like "r:amount" could be confused with
    // (rowKey="r", field="amount").
    setFormula("b", "r:amount", "x", "=1");
    setFormula("b", "r", "amount:x", "=2");
    expect(getFormula("b", "r:amount", "x")).toBe("=1");
    expect(getFormula("b", "r", "amount:x")).toBe("=2");
  });
});
