import { describe, expect, it, vi } from "vitest";
import { act, renderHook, waitFor } from "@testing-library/react";
import { useBundle } from "../../src/hooks/useBundle";
import {
  SheetsClient,
  type Fetcher,
  type MinimalWebSocket,
} from "../../src/lib/gigi-client";

/**
 * S2.5 hook-level realtime tests.
 *
 * useBundle subscribes to /ws on mount. Incoming EVENT frames fold into
 * the rows array; NOTICE frames bump laggedCount.
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

interface FakeWS extends MinimalWebSocket {
  emit(line: string): void;
  fire(type: "open" | "close" | "error"): void;
  url: string;
  sent: string[];
}

function makeStubWebSocket(url: string): FakeWS {
  type Listener = (ev: { data?: string }) => void;
  const listeners: Record<string, Listener[]> = {
    open: [],
    message: [],
    close: [],
    error: [],
  };
  const ws: FakeWS = {
    url,
    readyState: 0,
    sent: [],
    send(data: string) {
      ws.sent.push(data);
    },
    close() {
      ws.readyState = 3;
      for (const l of listeners.close) l({});
    },
    addEventListener(type, listener) {
      listeners[type].push(listener);
    },
    emit(line: string) {
      for (const l of listeners.message) l({ data: line });
    },
    fire(type) {
      if (type === "open") ws.readyState = 1;
      for (const l of listeners[type]) l({});
    },
  };
  return ws;
}

function fetchMock(handlers: Record<string, () => Response>): Fetcher {
  return vi.fn(async (input: RequestInfo | URL) => {
    const url = String(input);
    for (const [k, h] of Object.entries(handlers)) {
      if (url.includes(k)) return h();
    }
    return new Response("not mocked: " + url, { status: 500 });
  }) as unknown as Fetcher;
}

function jsonResponse(payload: unknown, status = 200) {
  return new Response(JSON.stringify(payload), {
    status,
    headers: { "content-type": "application/json" },
  });
}

async function setup() {
  const sockets: FakeWS[] = [];
  const client = new SheetsClient({
    baseUrl: "http://localhost:3142",
    fetch: fetchMock({
      "/schema": () => jsonResponse(SCHEMA),
      "/query": () =>
        jsonResponse({ data: ROWS, total: 2, curvature: 0.1, confidence: 0.9 }),
    }),
    WebSocket: (url) => {
      const s = makeStubWebSocket(url);
      sockets.push(s);
      return s;
    },
  });
  const hook = renderHook(() => useBundle(client, "sensors"));
  await waitFor(() => expect(hook.result.current.loading).toBe(false));
  // The subscription effect runs synchronously; the socket exists by now.
  expect(sockets).toHaveLength(1);
  return { hook, socket: sockets[0] };
}

describe("useBundle — realtime status lifecycle", () => {
  it("starts in 'connecting' and flips to 'open' on the WS open event", async () => {
    const { hook, socket } = await setup();
    expect(hook.result.current.realtime).toBe("connecting");
    await act(async () => {
      socket.fire("open");
    });
    expect(hook.result.current.realtime).toBe("open");
  });

  it("sends SUBSCRIBE <bundle> after open", async () => {
    const { socket } = await setup();
    await act(async () => {
      socket.fire("open");
    });
    expect(socket.sent).toEqual(["SUBSCRIBE sensors"]);
  });

  it("flips to 'closed' when the socket closes", async () => {
    const { hook, socket } = await setup();
    await act(async () => {
      socket.fire("open");
      socket.fire("close");
    });
    expect(hook.result.current.realtime).toBe("closed");
  });

  it("flips to 'error' on a socket error", async () => {
    const { hook, socket } = await setup();
    await act(async () => {
      socket.fire("error");
    });
    expect(hook.result.current.realtime).toBe("error");
  });

  it("never opens a socket when the schema fetch 404s", async () => {
    const sockets: FakeWS[] = [];
    const client = new SheetsClient({
      baseUrl: "http://localhost:3142",
      fetch: fetchMock({
        "/schema": () => new Response("not found", { status: 404 }),
      }),
      WebSocket: (url) => {
        const s = makeStubWebSocket(url);
        sockets.push(s);
        return s;
      },
    });
    const hook = renderHook(() => useBundle(client, "ghost-bundle"));
    await waitFor(() => expect(hook.result.current.loading).toBe(false));
    expect(hook.result.current.error?.status).toBe(404);
    // No WS handshake to the engine — this is the bug the schema gate fixes.
    expect(sockets).toHaveLength(0);
    expect(hook.result.current.realtime).toBe("off");
  });

  it("realtime: false leaves the status as 'off' and never opens a socket", async () => {
    const sockets: FakeWS[] = [];
    const client = new SheetsClient({
      baseUrl: "http://localhost:3142",
      fetch: fetchMock({
        "/schema": () => jsonResponse(SCHEMA),
        "/query": () =>
          jsonResponse({ data: ROWS, total: 2, curvature: 0.1, confidence: 0.9 }),
      }),
      WebSocket: (url) => {
        const s = makeStubWebSocket(url);
        sockets.push(s);
        return s;
      },
    });
    const hook = renderHook(() =>
      useBundle(client, "sensors", {}, { realtime: false }),
    );
    await waitFor(() => expect(hook.result.current.loading).toBe(false));
    expect(sockets).toHaveLength(0);
    expect(hook.result.current.realtime).toBe("off");
  });
});

describe("useBundle — folds incoming events into rows", () => {
  it("applies an UPDATE event without a user gesture", async () => {
    const { hook, socket } = await setup();
    await act(async () => {
      socket.fire("open");
      socket.emit(
        'EVENT sensors update {"sensor_id":"S-001","temp":99.9} K=2.4 C=0.29',
      );
    });
    expect(hook.result.current.rows[0]).toMatchObject({
      sensor_id: "S-001",
      temp: 99.9,
    });
    // κ + conf promoted from the event.
    expect(hook.result.current.curvature).toBeCloseTo(2.4);
    expect(hook.result.current.confidence).toBeCloseTo(0.29);
  });

  it("appends an INSERT event as a new row", async () => {
    const { hook, socket } = await setup();
    await act(async () => {
      socket.fire("open");
      socket.emit(
        'EVENT sensors insert {"sensor_id":"S-NEW","temp":42,"humidity":50} K=0.1 C=0.9',
      );
    });
    expect(hook.result.current.rows).toHaveLength(3);
    expect(hook.result.current.rows[2].sensor_id).toBe("S-NEW");
  });

  it("removes the row on DELETE", async () => {
    const { hook, socket } = await setup();
    await act(async () => {
      socket.fire("open");
      socket.emit(
        'EVENT sensors delete {"sensor_id":"S-002"} K=0.1 C=0.9',
      );
    });
    expect(
      hook.result.current.rows.map((r) => r.sensor_id),
    ).toEqual(["S-001"]);
  });

  it("ignores events for a different bundle (defense in depth)", async () => {
    const { hook, socket } = await setup();
    const before = JSON.stringify(hook.result.current.rows);
    await act(async () => {
      socket.fire("open");
      socket.emit(
        'EVENT other_bundle update {"sensor_id":"S-001","temp":999} K=10 C=0.01',
      );
    });
    expect(JSON.stringify(hook.result.current.rows)).toBe(before);
  });

  it("accumulates laggedCount from NOTICE frames", async () => {
    const { hook, socket } = await setup();
    await act(async () => {
      socket.fire("open");
      socket.emit("NOTICE sensors lagged=3");
      socket.emit("NOTICE sensors lagged=7");
    });
    expect(hook.result.current.laggedCount).toBe(10);
  });

  it("handles multiple events in a single message blob (newline-delimited)", async () => {
    const { hook, socket } = await setup();
    await act(async () => {
      socket.fire("open");
      socket.emit(
        [
          'EVENT sensors update {"sensor_id":"S-001","temp":50} K=1.0 C=0.5',
          'EVENT sensors insert {"sensor_id":"S-003","temp":20,"humidity":62} K=1.1 C=0.48',
        ].join("\n"),
      );
    });
    expect(hook.result.current.rows).toHaveLength(3);
    expect(hook.result.current.rows[0].temp).toBe(50);
    expect(hook.result.current.rows[2].sensor_id).toBe("S-003");
  });
});
