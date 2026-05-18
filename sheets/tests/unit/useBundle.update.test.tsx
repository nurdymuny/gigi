import { describe, expect, it, vi } from "vitest";
import { renderHook, act, waitFor } from "@testing-library/react";
import { useBundle } from "../../src/hooks/useBundle";
import { SheetsClient, type Fetcher } from "../../src/lib/gigi-client";

/**
 * S1 hook-level tests for useBundle.updateCell — optimistic application
 * and rollback semantics. The Grid renders whatever the hook returns,
 * so getting the hook's rows-array right is what makes inline edit feel
 * instant.
 */

const SCHEMA = {
  name: "sensors",
  base_fields: [{ name: "sensor_id", type: "text" }],
  fiber_fields: [
    { name: "temp", type: "numeric" },
    { name: "humidity", type: "numeric" },
  ],
  indexed_fields: ["sensor_id"],
  records: 2,
  storage_mode: "mmap",
};

const ROWS = [
  { sensor_id: "S-001", temp: 22.5, humidity: 60.1 },
  { sensor_id: "S-002", temp: 19.3, humidity: 71.4 },
];

function makeFetch(handlers: {
  schema?: () => Response | Promise<Response>;
  section?: () => Response | Promise<Response>;
  update?: () => Response | Promise<Response>;
}): Fetcher {
  return vi.fn(async (input: RequestInfo | URL) => {
    const url = String(input);
    if (url.endsWith("/schema") && handlers.schema) return handlers.schema();
    if (url.endsWith("/query") && handlers.section) return handlers.section();
    if (url.endsWith("/update") && handlers.update) return handlers.update();
    return new Response("not mocked: " + url, { status: 500 });
  }) as unknown as Fetcher;
}

function jsonResponse(payload: unknown, status = 200) {
  return new Response(JSON.stringify(payload), {
    status,
    headers: { "content-type": "application/json" },
  });
}

async function setupHook(fetchFn: Fetcher) {
  const client = new SheetsClient({
    baseUrl: "http://localhost:3142",
    fetch: fetchFn,
  });
  const hook = renderHook(() => useBundle(client, "sensors"));
  await waitFor(() => expect(hook.result.current.loading).toBe(false));
  return hook;
}

describe("useBundle.updateCell", () => {
  it("applies the new value optimistically before the engine responds", async () => {
    let resolveUpdate: ((r: Response) => void) | null = null;
    const updatePromise = new Promise<Response>((resolve) => {
      resolveUpdate = resolve;
    });
    const fetchFn = makeFetch({
      schema: () => jsonResponse(SCHEMA),
      section: () =>
        jsonResponse({ data: ROWS, total: 2, curvature: 0.1, confidence: 0.9 }),
      update: () => updatePromise,
    });
    const { result } = await setupHook(fetchFn);

    // Fire the update — don't await it yet.
    let mutPromise!: Promise<unknown>;
    act(() => {
      mutPromise = result.current.updateCell("S-001", "temp", 42.0);
    });

    // Optimistic value should be visible before the network resolves.
    await waitFor(() => {
      expect(result.current.rows[0]).toMatchObject({ sensor_id: "S-001", temp: 42.0 });
    });

    // Resolve the engine and let the promise settle.
    await act(async () => {
      resolveUpdate?.(
        jsonResponse({
          status: "updated",
          data: { sensor_id: "S-001", temp: 42.0, humidity: 60.1 },
          total: 2,
          curvature: 1.4,
          confidence: 0.42,
        }),
      );
      await mutPromise;
    });

    expect(result.current.rows[0]).toMatchObject({ temp: 42.0 });
    expect(result.current.curvature).toBeCloseTo(1.4);
    expect(result.current.confidence).toBeCloseTo(0.42);
  });

  it("rolls back the row when the engine rejects the write", async () => {
    const fetchFn = makeFetch({
      schema: () => jsonResponse(SCHEMA),
      section: () =>
        jsonResponse({ data: ROWS, total: 2, curvature: 0.1, confidence: 0.9 }),
      update: () => jsonResponse({ error: "bad" }, 400),
    });
    const { result } = await setupHook(fetchFn);

    let outcome!: { ok: boolean; error?: { code?: string } };
    await act(async () => {
      outcome = (await result.current.updateCell("S-001", "temp", 999)) as typeof outcome;
    });

    expect(outcome.ok).toBe(false);
    expect(outcome.error?.code).toBe("http_error");
    // Row reverted to original.
    expect(result.current.rows[0]).toMatchObject({ temp: 22.5 });
    // Bundle-level κ unchanged.
    expect(result.current.curvature).toBeCloseTo(0.1);
  });

  it("rolls back on a version_conflict from the engine", async () => {
    const fetchFn = makeFetch({
      schema: () => jsonResponse(SCHEMA),
      section: () =>
        jsonResponse({ data: ROWS, total: 2, curvature: 0.1, confidence: 0.9 }),
      update: () =>
        jsonResponse({ status: "version_conflict", total: 2, curvature: 0.1, confidence: 0.9 }),
    });
    const { result } = await setupHook(fetchFn);

    let outcome!: { ok: boolean; error?: { code?: string } };
    await act(async () => {
      outcome = (await result.current.updateCell("S-001", "temp", 42)) as typeof outcome;
    });

    expect(outcome.ok).toBe(false);
    expect(outcome.error?.code).toBe("version_conflict");
    expect(result.current.rows[0]).toMatchObject({ temp: 22.5 });
  });

  it("returns ok=false with code='no_key' when the row isn't in view", async () => {
    const fetchFn = makeFetch({
      schema: () => jsonResponse(SCHEMA),
      section: () =>
        jsonResponse({ data: ROWS, total: 2, curvature: 0.1, confidence: 0.9 }),
    });
    const { result } = await setupHook(fetchFn);

    let outcome!: { ok: boolean; error?: { code?: string } };
    await act(async () => {
      outcome = (await result.current.updateCell("S-MISSING", "temp", 1)) as typeof outcome;
    });

    expect(outcome.ok).toBe(false);
    expect(outcome.error?.code).toBe("no_key");
  });
});
