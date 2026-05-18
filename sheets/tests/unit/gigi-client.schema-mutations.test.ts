import { describe, expect, it, vi } from "vitest";
import {
  SheetsClient,
  SheetsClientError,
  type Fetcher,
} from "../../src/lib/gigi-client";

function jsonResponse(payload: unknown, status = 200) {
  return new Response(JSON.stringify(payload), {
    status,
    headers: { "content-type": "application/json" },
  });
}

describe("SheetsClient.addField", () => {
  it("POSTs name + type + default to /add-field", async () => {
    const fakeFetch = vi
      .fn()
      .mockResolvedValue(jsonResponse({ status: "ok" })) as unknown as Fetcher;
    const client = new SheetsClient({
      baseUrl: "http://localhost:3142",
      fetch: fakeFetch,
    });
    await client.addField("sensors", { name: "pressure", type: "numeric", default: 1013 });
    const [url, init] = (fakeFetch as unknown as ReturnType<typeof vi.fn>).mock.calls[0];
    expect(url).toBe("http://localhost:3142/v1/bundles/sensors/add-field");
    expect(JSON.parse(init.body)).toEqual({
      name: "pressure",
      type: "numeric",
      default: 1013,
    });
  });

  it("rejects unsafe field names without contacting the engine", async () => {
    const fakeFetch = vi.fn() as unknown as Fetcher;
    const client = new SheetsClient({
      baseUrl: "http://localhost:3142",
      fetch: fakeFetch,
    });
    await expect(
      client.addField("sensors", { name: "bad name; DROP", type: "text" }),
    ).rejects.toBeInstanceOf(SheetsClientError);
    expect(fakeFetch).not.toHaveBeenCalled();
  });

  it("surfaces engine errors as http_error", async () => {
    const fakeFetch = vi
      .fn()
      .mockResolvedValue(new Response("conflict", { status: 409 })) as unknown as Fetcher;
    const client = new SheetsClient({
      baseUrl: "http://localhost:3142",
      fetch: fakeFetch,
    });
    await expect(
      client.addField("sensors", { name: "ok_name", type: "text" }),
    ).rejects.toMatchObject({ code: "http_error", status: 409 });
  });
});

describe("SheetsClient.dropField", () => {
  it("POSTs { field } to /drop-field", async () => {
    const fakeFetch = vi
      .fn()
      .mockResolvedValue(jsonResponse({ status: "ok" })) as unknown as Fetcher;
    const client = new SheetsClient({
      baseUrl: "http://localhost:3142",
      fetch: fakeFetch,
    });
    await client.dropField("sensors", "pressure");
    const [url, init] = (fakeFetch as unknown as ReturnType<typeof vi.fn>).mock.calls[0];
    expect(url).toBe("http://localhost:3142/v1/bundles/sensors/drop-field");
    expect(JSON.parse(init.body)).toEqual({ field: "pressure" });
  });
});
