import { describe, expect, it } from "vitest";
import {
  applyEventToRows,
  parseSubscriptionFrame,
} from "../../src/lib/subscribe";

/**
 * Wire format reference (from gigi_stream.rs SubscriptionEvent):
 *   EVENT <bundle> <op> <record_json> K=<kappa> C=<confidence>
 *   NOTICE <bundle> lagged=<count>
 *
 * These tests pin the parser + apply semantics. The grid's optimistic
 * UI relies on `applyEventToRows` being deterministic and pure.
 */

describe("parseSubscriptionFrame — happy paths", () => {
  it("parses an EVENT insert with curvature + confidence", () => {
    const frame = parseSubscriptionFrame(
      'EVENT sensors insert {"sensor_id":"S-001","temp":22.5} K=0.42 C=0.7',
    );
    expect(frame).toEqual({
      kind: "event",
      bundle: "sensors",
      op: "insert",
      record: { sensor_id: "S-001", temp: 22.5 },
      kappa: 0.42,
      confidence: 0.7,
    });
  });

  it("parses an EVENT update", () => {
    const frame = parseSubscriptionFrame(
      'EVENT sensors update {"sensor_id":"S-001","temp":42.0} K=1.4 C=0.42',
    );
    expect(frame?.kind).toBe("event");
    if (frame?.kind === "event") {
      expect(frame.op).toBe("update");
      expect(frame.record).toEqual({ sensor_id: "S-001", temp: 42.0 });
    }
  });

  it("parses an EVENT delete", () => {
    const frame = parseSubscriptionFrame(
      'EVENT sensors delete {"sensor_id":"S-001"} K=0.1 C=0.9',
    );
    expect(frame?.kind === "event" && frame.op).toBe("delete");
  });

  it("parses a bulk_update event whose record is an array", () => {
    const frame = parseSubscriptionFrame(
      'EVENT sensors bulk_update [{"sensor_id":"S-001","temp":1},{"sensor_id":"S-002","temp":2}] K=0.1 C=0.9',
    );
    expect(frame?.kind === "event" && Array.isArray(frame.record)).toBe(true);
  });

  it("parses a NOTICE lag frame", () => {
    const frame = parseSubscriptionFrame("NOTICE sensors lagged=17");
    expect(frame).toEqual({ kind: "notice", bundle: "sensors", lagged: 17 });
  });

  it("scientific-notation K/C values parse", () => {
    const frame = parseSubscriptionFrame(
      'EVENT sensors insert {"sensor_id":"S-001"} K=1.5e-3 C=9.9e-1',
    );
    expect(frame?.kind === "event" && frame.kappa).toBeCloseTo(0.0015);
    expect(frame?.kind === "event" && frame.confidence).toBeCloseTo(0.99);
  });

  it("trims whitespace around the frame", () => {
    const frame = parseSubscriptionFrame(
      '   EVENT sensors insert {"sensor_id":"S-001"} K=0 C=0   ',
    );
    expect(frame?.kind).toBe("event");
  });
});

describe("parseSubscriptionFrame — defensive", () => {
  it("returns null for empty input", () => {
    expect(parseSubscriptionFrame("")).toBeNull();
    expect(parseSubscriptionFrame("   ")).toBeNull();
  });

  it("returns null for unknown leaders", () => {
    expect(parseSubscriptionFrame("HELLO sensors")).toBeNull();
    expect(parseSubscriptionFrame("status: open")).toBeNull();
  });

  it("returns null for malformed JSON in the record", () => {
    expect(
      parseSubscriptionFrame("EVENT sensors insert {not-json} K=0 C=0"),
    ).toBeNull();
  });

  it("returns null for unknown ops (defense against future engine versions)", () => {
    expect(
      parseSubscriptionFrame(
        'EVENT sensors purge {"sensor_id":"S-001"} K=0 C=0',
      ),
    ).toBeNull();
  });

  it("does not get confused by record JSON containing literal K=", () => {
    // The K= scanner is anchored from the right.
    const frame = parseSubscriptionFrame(
      'EVENT sensors insert {"sensor_id":"S-001","note":"K=5 inside text"} K=0.7 C=0.3',
    );
    expect(frame?.kind === "event" && frame.kappa).toBe(0.7);
    expect(frame?.kind === "event" && frame.confidence).toBe(0.3);
    expect(frame?.kind === "event" && (frame.record as Record<string, unknown>).note).toBe("K=5 inside text");
  });

  it("returns null for NOTICE without a lagged count", () => {
    expect(parseSubscriptionFrame("NOTICE sensors hello")).toBeNull();
  });
});

describe("applyEventToRows", () => {
  const ROWS = [
    { sensor_id: "S-001", temp: 22, humidity: 60 },
    { sensor_id: "S-002", temp: 23, humidity: 58 },
  ];

  it("inserts a brand-new row when the key is unseen", () => {
    const next = applyEventToRows(ROWS, "sensor_id", {
      kind: "event",
      bundle: "sensors",
      op: "insert",
      record: { sensor_id: "S-003", temp: 19, humidity: 71 },
      kappa: 0,
      confidence: 0,
    });
    expect(next).toHaveLength(3);
    expect(next[2]).toMatchObject({ sensor_id: "S-003" });
  });

  it("merges incoming fields into an existing row on update", () => {
    const next = applyEventToRows(ROWS, "sensor_id", {
      kind: "event",
      bundle: "sensors",
      op: "update",
      record: { sensor_id: "S-001", temp: 99 },
      kappa: 0,
      confidence: 0,
    });
    expect(next).toHaveLength(2);
    expect(next[0]).toEqual({ sensor_id: "S-001", temp: 99, humidity: 60 });
  });

  it("removes the row on delete", () => {
    const next = applyEventToRows(ROWS, "sensor_id", {
      kind: "event",
      bundle: "sensors",
      op: "delete",
      record: { sensor_id: "S-001" },
      kappa: 0,
      confidence: 0,
    });
    expect(next.map((r) => r.sensor_id)).toEqual(["S-002"]);
  });

  it("handles bulk_update applied to multiple rows", () => {
    const next = applyEventToRows(ROWS, "sensor_id", {
      kind: "event",
      bundle: "sensors",
      op: "bulk_update",
      record: [
        { sensor_id: "S-001", temp: 99 },
        { sensor_id: "S-002", temp: 88 },
      ],
      kappa: 0,
      confidence: 0,
    });
    expect(next[0]).toMatchObject({ temp: 99 });
    expect(next[1]).toMatchObject({ temp: 88 });
  });

  it("handles bulk_delete by key array", () => {
    const next = applyEventToRows(ROWS, "sensor_id", {
      kind: "event",
      bundle: "sensors",
      op: "bulk_delete",
      record: [{ sensor_id: "S-002" }],
      kappa: 0,
      confidence: 0,
    });
    expect(next.map((r) => r.sensor_id)).toEqual(["S-001"]);
  });

  it("never mutates the input array", () => {
    const before = ROWS.slice();
    const beforeJson = JSON.stringify(before);
    applyEventToRows(ROWS, "sensor_id", {
      kind: "event",
      bundle: "sensors",
      op: "update",
      record: { sensor_id: "S-001", temp: 999 },
      kappa: 0,
      confidence: 0,
    });
    expect(JSON.stringify(ROWS)).toBe(beforeJson);
  });

  it("upsert behaves like insert when row absent and update when present", () => {
    let next = applyEventToRows(ROWS, "sensor_id", {
      kind: "event",
      bundle: "sensors",
      op: "upsert",
      record: { sensor_id: "S-999", temp: 1 },
      kappa: 0,
      confidence: 0,
    });
    expect(next).toHaveLength(3);
    next = applyEventToRows(next, "sensor_id", {
      kind: "event",
      bundle: "sensors",
      op: "upsert",
      record: { sensor_id: "S-001", temp: 77 },
      kappa: 0,
      confidence: 0,
    });
    expect(next.find((r) => r.sensor_id === "S-001")).toEqual({
      sensor_id: "S-001",
      temp: 77,
      humidity: 60,
    });
  });
});
