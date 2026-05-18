import { describe, expect, it, vi } from "vitest";
import { render, screen, waitFor } from "@testing-library/react";
import { BundlePicker } from "../../src/components/BundlePicker";
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

function makeClient(payload: unknown, status = 200) {
  const fetcher = vi.fn(async () =>
    jsonResponse(payload, status),
  ) as unknown as Fetcher;
  return new SheetsClient({ baseUrl: "http://localhost:3142", fetch: fetcher });
}

describe("BundlePicker", () => {
  it("shows a loading state, then the list of bundles from listBundles()", async () => {
    const client = makeClient([
      { name: "sensors", records: 1284, fields: 6 },
      { name: "events", records: 42, fields: 4 },
    ]);
    render(<BundlePicker client={client} />);
    expect(screen.getByTestId("bundle-picker-loading")).toBeInTheDocument();
    await waitFor(() =>
      expect(screen.getByTestId("bundle-list")).toBeInTheDocument(),
    );
    expect(screen.getByTestId("bundle-pick-sensors")).toBeInTheDocument();
    expect(screen.getByTestId("bundle-pick-events")).toBeInTheDocument();
  });

  it("renders an empty-state when the engine has zero bundles", async () => {
    const client = makeClient([]);
    render(<BundlePicker client={client} />);
    await waitFor(() =>
      expect(screen.getByTestId("bundle-picker-empty")).toBeInTheDocument(),
    );
  });

  it("hides internal _gigi_* bundles behind a collapsible details element", async () => {
    const client = makeClient([
      { name: "sensors", records: 10, fields: 2 },
      { name: "_gigi_wal_log", records: 0, fields: 7 },
      { name: "_gigi_anomaly_log", records: 1, fields: 9 },
    ]);
    render(<BundlePicker client={client} />);
    await waitFor(() =>
      expect(screen.getByTestId("bundle-list")).toBeInTheDocument(),
    );
    // The user bundle is in the main list.
    expect(screen.getByTestId("bundle-pick-sensors")).toBeInTheDocument();
    // System bundles live inside a <details>.
    const systemSection = screen.getByTestId("bundle-system");
    expect(systemSection).toBeInTheDocument();
    expect(systemSection).toHaveTextContent(/System bundles \(2\)/);
  });

  it("each bundle row links to /gigi/sheets/<name>", async () => {
    const client = makeClient([{ name: "events", records: 1, fields: 1 }]);
    render(<BundlePicker client={client} />);
    await waitFor(() =>
      expect(screen.getByTestId("bundle-pick-events")).toBeInTheDocument(),
    );
    expect(screen.getByTestId("bundle-pick-events")).toHaveAttribute(
      "href",
      "/gigi/sheets/events",
    );
  });

  it("renders the row count + field count for each bundle", async () => {
    const client = makeClient([
      { name: "events", records: 12345, fields: 7 },
    ]);
    render(<BundlePicker client={client} />);
    await waitFor(() =>
      expect(screen.getByTestId("bundle-pick-events")).toBeInTheDocument(),
    );
    expect(screen.getByTestId("bundle-count-events")).toHaveTextContent(
      /12,345 rows/,
    );
  });

  it("surfaces a 'not found' header when requestedBundle + loadError are passed", async () => {
    const client = makeClient([{ name: "events", records: 1, fields: 1 }]);
    const err = new SheetsClientError("HTTP 404", "http_error", 404);
    render(
      <BundlePicker
        client={client}
        requestedBundle="sensors"
        loadError={err}
      />,
    );
    const header = screen.getByTestId("bundle-picker").querySelector("header")!;
    expect(header).toHaveTextContent(/sensors/);
    expect(header).toHaveTextContent(/HTTP 404/);
  });

  it("renders a generic 'pick a bundle' header when no requested bundle is passed", async () => {
    const client = makeClient([{ name: "events", records: 1, fields: 1 }]);
    render(<BundlePicker client={client} />);
    const header = screen.getByTestId("bundle-picker").querySelector("header")!;
    expect(header).toHaveTextContent(/pick a bundle/i);
  });

  it("surfaces engine errors instead of the list when listBundles() fails", async () => {
    const fetcher = vi.fn(async () =>
      new Response("boom", { status: 500 }),
    ) as unknown as Fetcher;
    const client = new SheetsClient({
      baseUrl: "http://localhost:3142",
      fetch: fetcher,
    });
    render(<BundlePicker client={client} />);
    await waitFor(() =>
      expect(screen.getByTestId("bundle-picker-error")).toBeInTheDocument(),
    );
    expect(screen.getByRole("alert")).toHaveTextContent(/HTTP 500/);
  });
});
