import { beforeEach, describe, expect, it } from "vitest";
import {
  decodeView,
  deleteView,
  encodeView,
  listViews,
  saveView,
  urlForView,
  viewFromUrl,
  type ViewSpec,
} from "../../src/lib/view";

describe("encodeView / decodeView — round trip", () => {
  it("round-trips a minimal spec", () => {
    const spec: ViewSpec = { v: 1 };
    expect(decodeView(encodeView(spec))).toEqual({ v: 1 });
  });

  it("round-trips a fully-populated spec", () => {
    const spec: ViewSpec = {
      v: 1,
      coverField: "site_id",
      overlayOn: true,
      activeView: "geometry",
      inspectorOpen: false,
      gqlQuery: "SECTION sensors WHERE x=1;",
    };
    expect(decodeView(encodeView(spec))).toEqual(spec);
  });

  it("preserves boolean false explicitly (not lost as undefined)", () => {
    const spec: ViewSpec = { v: 1, overlayOn: false, inspectorOpen: false };
    expect(decodeView(encodeView(spec))).toEqual(spec);
  });

  it("ignores unknown activeView values defensively", () => {
    const bad = { v: 1, t: "evil-tab" };
    const url = encodeView({ v: 1 }); // any valid encoding to overwrite
    expect(url).toMatch(/^[A-Za-z0-9_-]+$/);
    // Decode raw to confirm we filter:
    const encoded = encodeViewRaw(bad);
    const decoded = decodeView(encoded);
    expect(decoded?.activeView).toBeUndefined();
  });

  it("returns null for malformed encoded strings", () => {
    expect(decodeView("not_base64!@#")).toBeNull();
    expect(decodeView("")).toBeNull();
  });

  it("returns null when version is wrong (forward-compat)", () => {
    const future = encodeViewRaw({ v: 2 });
    expect(decodeView(future)).toBeNull();
  });
});

describe("viewFromUrl + urlForView", () => {
  it("extracts a view from ?view=… (with or without leading ?)", () => {
    const spec: ViewSpec = { v: 1, coverField: "site_id", overlayOn: true };
    const search = urlForView(spec);
    expect(search.startsWith("?")).toBe(true);
    expect(viewFromUrl(search)).toEqual(spec);
    expect(viewFromUrl(search.slice(1))).toEqual(spec);
  });

  it("returns null when there is no view param", () => {
    expect(viewFromUrl("")).toBeNull();
    expect(viewFromUrl("?foo=bar")).toBeNull();
  });
});

describe("saveView / listViews / deleteView", () => {
  beforeEach(() => {
    localStorage.clear();
  });

  it("saves a view and returns a NamedView with an id", () => {
    const v = saveView({
      name: "All anomalies",
      bundle: "sensors",
      spec: { v: 1, coverField: "site_id" },
    });
    expect(v.id).toMatch(/^v_/);
    expect(v.name).toBe("All anomalies");
    expect(v.bundle).toBe("sensors");
  });

  it("lists saved views, most recent first", () => {
    saveView({ name: "a", bundle: "sensors", spec: { v: 1 } });
    saveView({ name: "b", bundle: "sensors", spec: { v: 1 } });
    saveView({ name: "c", bundle: "events", spec: { v: 1 } });
    const all = listViews();
    expect(all.map((v) => v.name)).toEqual(["c", "b", "a"]);
  });

  it("filters to the right bundle when one is given", () => {
    saveView({ name: "a", bundle: "sensors", spec: { v: 1 } });
    saveView({ name: "b", bundle: "events", spec: { v: 1 } });
    expect(listViews("sensors").map((v) => v.name)).toEqual(["a"]);
    expect(listViews("events").map((v) => v.name)).toEqual(["b"]);
  });

  it("deleteView removes by id", () => {
    const v1 = saveView({ name: "a", bundle: "sensors", spec: { v: 1 } });
    saveView({ name: "b", bundle: "sensors", spec: { v: 1 } });
    deleteView(v1.id);
    expect(listViews("sensors").map((v) => v.name)).toEqual(["b"]);
  });

  it("recovers from a corrupt storage payload", () => {
    localStorage.setItem("gigi.sheets.views", "not json");
    expect(listViews()).toEqual([]);
  });
});

// Helper: build an encoded URL string from an arbitrary object — used to
// test forward-compat / defensive decoding without going through encodeView's
// schema-aware compactor.
function encodeViewRaw(obj: unknown): string {
  const bin = new TextEncoder().encode(JSON.stringify(obj));
  let s = "";
  for (const b of bin) s += String.fromCharCode(b);
  return btoa(s).replace(/\+/g, "-").replace(/\//g, "_").replace(/=+$/, "");
}
