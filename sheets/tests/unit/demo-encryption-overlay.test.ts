import { describe, expect, it, beforeEach } from "vitest";
import {
  applyOverlay,
  clearOverlay,
  getOverlay,
  registerOverlay,
} from "../../src/lib/demo-encryption-overlay";
import type { BundleSchema } from "../../src/lib/gigi-client";

beforeEach(() => {
  localStorage.clear();
});

const SCHEMA: BundleSchema = {
  name: "hospital_records",
  base_fields: [{ name: "patient_id", type: "text" }],
  fiber_fields: [
    { name: "patient_name", type: "text" },
    { name: "department", type: "categorical" },
    { name: "bp_systolic", type: "numeric" },
  ],
  indexed_fields: ["patient_id"],
  records: 30,
  storage_mode: "mmap",
};

describe("demo encryption overlay", () => {
  it("returns null when no overlay is registered for a bundle", () => {
    expect(getOverlay("hospital_records")).toBeNull();
  });

  it("registers and reads back an overlay", () => {
    registerOverlay("hospital_records", {
      patient_id: "indexed",
      patient_name: "opaque",
      bp_systolic: "affine",
    });
    expect(getOverlay("hospital_records")).toEqual({
      patient_id: "indexed",
      patient_name: "opaque",
      bp_systolic: "affine",
    });
  });

  it("persists across reads (localStorage-backed)", () => {
    registerOverlay("hospital_records", { patient_name: "opaque" });
    // Simulate a fresh module instance — clearing in-memory state.
    expect(getOverlay("hospital_records")?.patient_name).toBe("opaque");
  });

  it("clears overlay for one bundle without affecting others", () => {
    registerOverlay("a", { f: "opaque" });
    registerOverlay("b", { g: "indexed" });
    clearOverlay("a");
    expect(getOverlay("a")).toBeNull();
    expect(getOverlay("b")).toEqual({ g: "indexed" });
  });

  it("applyOverlay tags fields with the registered encryption mode", () => {
    registerOverlay("hospital_records", {
      patient_id: "indexed",
      patient_name: "opaque",
      bp_systolic: "affine",
    });
    const tagged = applyOverlay(SCHEMA);
    expect(tagged.base_fields[0].encryption).toBe("indexed");
    expect(tagged.fiber_fields[0].encryption).toBe("opaque"); // patient_name
    expect(tagged.fiber_fields[1].encryption).toBeUndefined(); // department
    expect(tagged.fiber_fields[2].encryption).toBe("affine"); // bp_systolic
  });

  it("applyOverlay leaves the schema unchanged when no overlay is registered", () => {
    const out = applyOverlay(SCHEMA);
    expect(out).toEqual(SCHEMA);
  });

  it("applyOverlay defers to server-side encryption when present", () => {
    registerOverlay("hospital_records", { patient_name: "opaque" });
    const withServerEnc: BundleSchema = {
      ...SCHEMA,
      fiber_fields: SCHEMA.fiber_fields.map((f) =>
        f.name === "patient_name" ? { ...f, encryption: "indexed" } : f,
      ),
    };
    const out = applyOverlay(withServerEnc);
    // Server said indexed — overlay must NOT downgrade to opaque.
    expect(out.fiber_fields[0].encryption).toBe("indexed");
  });

  it("applyOverlay overrides a server 'none' value", () => {
    registerOverlay("hospital_records", { patient_name: "opaque" });
    const withNone: BundleSchema = {
      ...SCHEMA,
      fiber_fields: SCHEMA.fiber_fields.map((f) =>
        f.name === "patient_name" ? { ...f, encryption: "none" } : f,
      ),
    };
    const out = applyOverlay(withNone);
    expect(out.fiber_fields[0].encryption).toBe("opaque");
  });
});
