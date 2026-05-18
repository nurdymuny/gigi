import { describe, expect, it, vi, beforeEach, afterEach } from "vitest";
import { render, screen, fireEvent, waitFor } from "@testing-library/react";
import { DemoBundles } from "../../src/components/DemoBundles";
import { SheetsClient, type Fetcher } from "../../src/lib/gigi-client";

function jsonResponse(payload: unknown) {
  return new Response(JSON.stringify(payload), {
    status: 200,
    headers: { "content-type": "application/json" },
  });
}

const originalLocation = window.location;

beforeEach(() => {
  // Stub window.location so the demo loader's navigation doesn't actually fire.
  Object.defineProperty(window, "location", {
    value: { ...originalLocation, href: "" },
    writable: true,
  });
});

afterEach(() => {
  Object.defineProperty(window, "location", { value: originalLocation, writable: true });
});

describe("DemoBundles", () => {
  it("renders one card per demo dataset with stats and source", () => {
    const client = new SheetsClient({
      baseUrl: "http://localhost:3142",
      fetch: vi.fn() as unknown as Fetcher,
    });
    render(<DemoBundles client={client} existing={new Set()} />);
    expect(screen.getByTestId("demo-iris")).toBeInTheDocument();
    expect(screen.getByTestId("demo-nba_2024")).toBeInTheDocument();
    expect(screen.getByTestId("demo-world_cities")).toBeInTheDocument();
    expect(screen.getByTestId("demo-mall_customers")).toBeInTheDocument();
    expect(screen.getByTestId("demo-iris")).toHaveTextContent(/150 rows/);
    expect(screen.getByTestId("demo-iris")).toHaveTextContent(/Fisher/);
  });

  it("marks an already-imported demo with an 'open' link instead of a load button", () => {
    const client = new SheetsClient({
      baseUrl: "http://localhost:3142",
      fetch: vi.fn() as unknown as Fetcher,
    });
    render(<DemoBundles client={client} existing={new Set(["iris"])} />);
    expect(screen.getByTestId("demo-open-iris")).toBeInTheDocument();
    expect(screen.queryByTestId("demo-load-iris")).toBeNull();
    expect(screen.getByTestId("demo-iris")).toHaveAttribute("data-state", "imported");
  });

  it("calls createBundle + insert when the user clicks Load", async () => {
    const fakeFetch = vi
      .fn()
      .mockResolvedValueOnce(jsonResponse({ status: "ok" })) // createBundle
      .mockResolvedValue(jsonResponse({ inserted: 50, curvature: 0 })) as unknown as Fetcher;
    const client = new SheetsClient({
      baseUrl: "http://localhost:3142",
      fetch: fakeFetch,
    });
    const onImported = vi.fn();
    render(
      <DemoBundles
        client={client}
        existing={new Set()}
        onImported={onImported}
      />,
    );
    fireEvent.click(screen.getByTestId("demo-load-mall_customers"));
    await waitFor(() => expect(onImported).toHaveBeenCalledWith("mall_customers"));

    // First call creates the bundle (POST /v1/bundles); subsequent ones insert.
    const calls = (fakeFetch as unknown as ReturnType<typeof vi.fn>).mock.calls;
    expect(calls[0][0]).toMatch(/\/v1\/bundles$/);
    expect(calls[0][1].method).toBe("POST");
    const createBody = JSON.parse(calls[0][1].body);
    expect(createBody.name).toBe("mall_customers");
    // The engine wants a nested schema with keys.
    expect(createBody.schema?.keys ?? createBody.keys).toEqual(["id"]);
    // Subsequent calls are inserts.
    const insertCalls = calls.slice(1);
    expect(insertCalls.length).toBeGreaterThan(0);
    for (const c of insertCalls) {
      expect(c[0]).toContain("/insert");
    }
  });

  it("surfaces errors from the engine without crashing", async () => {
    const fakeFetch = vi
      .fn()
      .mockResolvedValue(new Response("name conflict", { status: 409 })) as unknown as Fetcher;
    const client = new SheetsClient({
      baseUrl: "http://localhost:3142",
      fetch: fakeFetch,
    });
    render(<DemoBundles client={client} existing={new Set()} />);
    fireEvent.click(screen.getByTestId("demo-load-iris"));
    await waitFor(() =>
      expect(screen.getByTestId("demo-iris")).toHaveAttribute("data-state", "error"),
    );
  });
});
