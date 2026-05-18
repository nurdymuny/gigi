import { describe, expect, it } from "vitest";
import { bundleFromPath, pathForBundle } from "../../src/lib/route";

describe("bundleFromPath", () => {
  it("returns null for the bare base path (let the app default)", () => {
    expect(bundleFromPath("/gigi/sheets/")).toBeNull();
    expect(bundleFromPath("/gigi/sheets")).toBeNull();
  });

  it("extracts the first segment after the base path", () => {
    expect(bundleFromPath("/gigi/sheets/sensors")).toBe("sensors");
    expect(bundleFromPath("/gigi/sheets/sensors/")).toBe("sensors");
  });

  it("ignores anything past the first segment", () => {
    expect(bundleFromPath("/gigi/sheets/sensors/anomalies")).toBe("sensors");
  });

  it("accepts identifier-safe names (underscores, hyphens, digits)", () => {
    expect(bundleFromPath("/gigi/sheets/sensor_log")).toBe("sensor_log");
    expect(bundleFromPath("/gigi/sheets/marc-2026")).toBe("marc-2026");
    expect(bundleFromPath("/gigi/sheets/_internal")).toBe("_internal");
  });

  it("rejects unsafe names (leading digits, special chars, spaces)", () => {
    expect(bundleFromPath("/gigi/sheets/123sensors")).toBeNull();
    expect(bundleFromPath("/gigi/sheets/sensors;DROP")).toBeNull();
    expect(bundleFromPath("/gigi/sheets/spaces in name")).toBeNull();
    expect(bundleFromPath("/gigi/sheets/.dotfile")).toBeNull();
  });
});

describe("pathForBundle", () => {
  it("builds the URL for a given bundle", () => {
    expect(pathForBundle("sensors")).toBe("/gigi/sheets/sensors");
  });

  it("encodes unsafe characters defensively", () => {
    expect(pathForBundle("with space")).toContain("with%20space");
  });
});
