import { describe, expect, it, vi } from "vitest";
import { SheetsClient, type Fetcher } from "../../src/lib/gigi-client";

function jsonResponse(payload: unknown) {
  return new Response(JSON.stringify(payload), {
    status: 200,
    headers: { "content-type": "application/json" },
  });
}

describe("SheetsClient.schema — type normalization", () => {
  it("lowercases TitleCase field types from the engine", async () => {
    const engineResponse = {
      name: "marcella_source_claims",
      base_fields: [{ name: "claim_id", type: "Categorical" }],
      fiber_fields: [
        { name: "claim_type", type: "Categorical" },
        { name: "line_end", type: "Numeric" },
        { name: "n_chars", type: "Numeric" },
        { name: "section_id", type: "Categorical" },
      ],
      indexed_fields: ["claim_id"],
      records: 2908,
      storage_mode: "mmap",
    };
    const fakeFetch = vi
      .fn()
      .mockResolvedValue(jsonResponse(engineResponse)) as unknown as Fetcher;
    const client = new SheetsClient({
      baseUrl: "http://localhost:3142",
      fetch: fakeFetch,
    });
    const schema = await client.schema("marcella_source_claims");
    expect(schema.base_fields[0].type).toBe("categorical");
    expect(schema.fiber_fields.map((f) => f.type)).toEqual([
      "categorical",
      "numeric",
      "numeric",
      "categorical",
    ]);
  });

  it("leaves already-lowercase types alone", async () => {
    const engineResponse = {
      name: "sensors",
      base_fields: [{ name: "sensor_id", type: "text" }],
      fiber_fields: [{ name: "temp", type: "numeric" }],
      indexed_fields: ["sensor_id"],
      records: 1,
      storage_mode: "mmap",
    };
    const fakeFetch = vi
      .fn()
      .mockResolvedValue(jsonResponse(engineResponse)) as unknown as Fetcher;
    const client = new SheetsClient({
      baseUrl: "http://localhost:3142",
      fetch: fakeFetch,
    });
    const schema = await client.schema("sensors");
    expect(schema.fiber_fields[0].type).toBe("numeric");
    expect(schema.base_fields[0].type).toBe("text");
  });

  it("handles missing base_fields/fiber_fields gracefully", async () => {
    const fakeFetch = vi
      .fn()
      .mockResolvedValue(
        jsonResponse({
          name: "empty",
          indexed_fields: [],
          records: 0,
          storage_mode: "mmap",
        }),
      ) as unknown as Fetcher;
    const client = new SheetsClient({
      baseUrl: "http://localhost:3142",
      fetch: fakeFetch,
    });
    const schema = await client.schema("empty");
    expect(schema.base_fields).toEqual([]);
    expect(schema.fiber_fields).toEqual([]);
  });
});
