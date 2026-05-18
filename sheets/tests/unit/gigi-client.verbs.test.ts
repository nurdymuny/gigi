import { describe, expect, it, vi } from "vitest";
import {
  SheetsClient,
  SheetsClientError,
  type Fetcher,
} from "../../src/lib/gigi-client";

/**
 * S3 — verb client surface: spectral, betti, transport, holonomy, gql.
 *
 * Wire contracts come from gigi_stream.rs (audited 2026-05-14):
 *   GET  /v1/bundles/{name}/spectral  → SpectralReport { lambda1, diameter, spectral_capacity }
 *   GET  /v1/bundles/{name}/betti     → BettiReport    { beta_0, beta_1 }
 *   POST /v1/gql  body={query}        → { rows, count }
 *     TRANSPORT rows: { dim, angle, matrix: number[] }
 *     HOLONOMY  rows: [{ <around>, <f0>, <f1>, transport_angle }, …, summary]
 */

function jsonResponse(payload: unknown, status = 200) {
  return new Response(JSON.stringify(payload), {
    status,
    headers: { "content-type": "application/json" },
  });
}

function client(fetcher: ReturnType<typeof vi.fn>) {
  return new SheetsClient({
    baseUrl: "http://localhost:3142",
    fetch: fetcher as unknown as Fetcher,
  });
}

describe("SheetsClient.spectral", () => {
  it("GETs the spectral endpoint and returns lambda1 + diameter + capacity", async () => {
    const fakeFetch = vi
      .fn()
      .mockResolvedValue(
        jsonResponse({ lambda1: 0.082, diameter: 7, spectral_capacity: 0.412 }),
      );
    const r = await client(fakeFetch).spectral("sensors");
    expect(r).toEqual({ lambda1: 0.082, diameter: 7, spectral_capacity: 0.412 });
    const [url, init] = fakeFetch.mock.calls[0];
    expect(url).toBe("http://localhost:3142/v1/bundles/sensors/spectral");
    expect(init.method).toBe("GET");
  });

  it("defaults missing numeric fields to 0", async () => {
    const fakeFetch = vi.fn().mockResolvedValue(jsonResponse({ lambda1: 0.1 }));
    const r = await client(fakeFetch).spectral("sensors");
    expect(r).toEqual({ lambda1: 0.1, diameter: 0, spectral_capacity: 0 });
  });

  it("surfaces 404 as SheetsClientError(http_error, 404)", async () => {
    const fakeFetch = vi
      .fn()
      .mockResolvedValue(new Response("not found", { status: 404 }));
    await expect(client(fakeFetch).spectral("missing")).rejects.toMatchObject({
      name: "SheetsClientError",
      code: "http_error",
      status: 404,
    });
  });
});

describe("SheetsClient.betti", () => {
  it("GETs the betti endpoint and returns beta_0 + beta_1", async () => {
    const fakeFetch = vi
      .fn()
      .mockResolvedValue(jsonResponse({ beta_0: 4, beta_1: 2 }));
    const r = await client(fakeFetch).betti("sensors");
    expect(r).toEqual({ beta_0: 4, beta_1: 2 });
    const [url] = fakeFetch.mock.calls[0];
    expect(url).toBe("http://localhost:3142/v1/bundles/sensors/betti");
  });
});

describe("SheetsClient.gql", () => {
  it("POSTs { query } and returns { rows, count }", async () => {
    const fakeFetch = vi
      .fn()
      .mockResolvedValue(
        jsonResponse({ rows: [{ a: 1 }, { a: 2 }], count: 2 }),
      );
    const r = await client(fakeFetch).gql("SECTION sensors;");
    expect(r.rows).toHaveLength(2);
    expect(r.count).toBe(2);
    const [url, init] = fakeFetch.mock.calls[0];
    expect(url).toBe("http://localhost:3142/v1/gql");
    expect(init.method).toBe("POST");
    expect(JSON.parse(init.body)).toEqual({ query: "SECTION sensors;" });
  });

  it("tolerates a missing count by falling back to rows.length", async () => {
    const fakeFetch = vi
      .fn()
      .mockResolvedValue(jsonResponse({ rows: [{ a: 1 }] }));
    const r = await client(fakeFetch).gql("SECTION sensors;");
    expect(r.count).toBe(1);
  });
});

describe("SheetsClient.transport", () => {
  it("issues TRANSPORT ... FROM ... TO ... ON FIBER (...) and parses dim/angle/matrix", async () => {
    const fakeFetch = vi.fn().mockResolvedValue(
      jsonResponse({
        rows: [
          {
            dim: 2,
            angle: 0.512,
            matrix: [0.871, -0.491, 0.491, 0.871],
          },
        ],
        count: 1,
      }),
    );
    const r = await client(fakeFetch).transport(
      "sensors",
      { sensor_id: "S-001" },
      { sensor_id: "S-002" },
      ["temp", "humidity"],
    );
    expect(r).toEqual({
      dim: 2,
      angle: 0.512,
      matrix: [0.871, -0.491, 0.491, 0.871],
    });
    const body = JSON.parse(fakeFetch.mock.calls[0][1].body);
    expect(body.query).toBe(
      "TRANSPORT sensors FROM (sensor_id='S-001') TO (sensor_id='S-002') ON FIBER (temp, humidity);",
    );
  });

  it("escapes single quotes in string key values", async () => {
    const fakeFetch = vi
      .fn()
      .mockResolvedValue(jsonResponse({ rows: [{ dim: 2, angle: 0, matrix: [] }], count: 1 }));
    await client(fakeFetch).transport(
      "sensors",
      { sensor_id: "O'Brien" },
      { sensor_id: "Smith" },
      ["temp", "humidity"],
    );
    const body = JSON.parse(fakeFetch.mock.calls[0][1].body);
    expect(body.query).toContain("(sensor_id='O''Brien')");
  });

  it("supports numeric and boolean key values", async () => {
    const fakeFetch = vi
      .fn()
      .mockResolvedValue(jsonResponse({ rows: [{ dim: 2, angle: 0, matrix: [] }], count: 1 }));
    await client(fakeFetch).transport(
      "events",
      { id: 7, active: true },
      { id: 8, active: false },
      ["x", "y"],
    );
    const body = JSON.parse(fakeFetch.mock.calls[0][1].body);
    expect(body.query).toContain("FROM (id=7, active=TRUE)");
    expect(body.query).toContain("TO (id=8, active=FALSE)");
  });

  it("rejects identifiers that don't match [A-Za-z_][A-Za-z0-9_]*", async () => {
    const fakeFetch = vi.fn();
    await expect(
      client(fakeFetch).transport(
        "sensors; DROP TABLE x; --",
        { sensor_id: "S-001" },
        { sensor_id: "S-002" },
        ["temp"],
      ),
    ).rejects.toBeInstanceOf(SheetsClientError);
    expect(fakeFetch).not.toHaveBeenCalled();
  });

  it("throws when the engine returns zero rows", async () => {
    const fakeFetch = vi.fn().mockResolvedValue(jsonResponse({ rows: [], count: 0 }));
    await expect(
      client(fakeFetch).transport(
        "sensors",
        { sensor_id: "S-001" },
        { sensor_id: "S-002" },
        ["temp", "humidity"],
      ),
    ).rejects.toMatchObject({ code: "parse_error" });
  });
});

describe("SheetsClient.holonomy", () => {
  it("issues HOLONOMY ... ON FIBER (...) AROUND <field>; and extracts the summary row", async () => {
    const fakeFetch = vi.fn().mockResolvedValue(
      jsonResponse({
        rows: [
          { site_id: "N", temp: 22, humidity: 60, transport_angle: 0.1 },
          { site_id: "S", temp: 25, humidity: 55, transport_angle: 0.2 },
          {
            _type: "summary",
            holonomy_angle: 0.183,
            holonomy_trivial: false,
          },
        ],
        count: 3,
      }),
    );
    const r = await client(fakeFetch).holonomy(
      "sensors",
      ["temp", "humidity"],
      "site_id",
    );
    expect(r.angle).toBeCloseTo(0.183);
    expect(r.trivial).toBe(false);
    expect(r.centroids).toHaveLength(2);
    expect(r.centroids[0]).toEqual({
      label: "N",
      fx: 22,
      fy: 60,
      transport_angle: 0.1,
    });
    const body = JSON.parse(fakeFetch.mock.calls[0][1].body);
    expect(body.query).toBe(
      "HOLONOMY sensors ON FIBER (temp, humidity) AROUND site_id;",
    );
  });

  it("requires at least 2 fiber fields", async () => {
    const fakeFetch = vi.fn();
    await expect(
      client(fakeFetch).holonomy("sensors", ["temp"], "site_id"),
    ).rejects.toMatchObject({ code: "parse_error" });
    expect(fakeFetch).not.toHaveBeenCalled();
  });

  it("returns angle=0 / trivial=true when the engine emits no summary row (degenerate cohort)", async () => {
    const fakeFetch = vi
      .fn()
      .mockResolvedValue(jsonResponse({ rows: [], count: 0 }));
    const r = await client(fakeFetch).holonomy(
      "sensors",
      ["temp", "humidity"],
      "site_id",
    );
    expect(r.angle).toBe(0);
    expect(r.trivial).toBe(true);
    expect(r.centroids).toEqual([]);
  });
});
