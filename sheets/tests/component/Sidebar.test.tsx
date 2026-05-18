import { describe, expect, it, vi, beforeEach, afterEach } from "vitest";
import { render, screen, waitFor, fireEvent } from "@testing-library/react";
import { Sidebar } from "../../src/components/Sidebar";
import { SheetsClient, type Fetcher } from "../../src/lib/gigi-client";

function jsonResponse(payload: unknown) {
  return new Response(JSON.stringify(payload), {
    status: 200,
    headers: { "content-type": "application/json" },
  });
}

beforeEach(() => {
  localStorage.clear();
});
afterEach(() => {
  localStorage.clear();
});

describe("Sidebar — signed in", () => {
  it("renders bundles from the engine and highlights the active one", async () => {
    const fakeFetch = vi.fn().mockResolvedValue(
      jsonResponse([
        { name: "sensors", records: 13, fields: 5 },
        { name: "iris", records: 150, fields: 6 },
        { name: "_gigi_log", records: 100, fields: 3 },
      ]),
    ) as unknown as Fetcher;
    const client = new SheetsClient({ baseUrl: "http://localhost:3142", fetch: fakeFetch });
    render(<Sidebar client={client} currentBundle="iris" signedIn />);
    await waitFor(() =>
      expect(screen.getByTestId("sidebar-bundle-iris")).toBeInTheDocument(),
    );
    expect(screen.getByTestId("sidebar-bundle-sensors")).toBeInTheDocument();
    expect(screen.getByTestId("sidebar-bundle-iris")).toHaveAttribute("data-active", "true");
    expect(screen.getByTestId("sidebar-bundle-sensors")).toHaveAttribute("data-active", "false");
    expect(screen.getByTestId("sidebar")).toHaveAttribute("data-mode", "user");
  });

  it("hides system bundles (_gigi_*) behind a disclosure", async () => {
    const fakeFetch = vi.fn().mockResolvedValue(
      jsonResponse([
        { name: "user_bundle", records: 1, fields: 1 },
        { name: "_gigi_log", records: 100, fields: 3 },
      ]),
    ) as unknown as Fetcher;
    const client = new SheetsClient({ baseUrl: "http://localhost:3142", fetch: fakeFetch });
    render(<Sidebar client={client} currentBundle="user_bundle" signedIn />);
    await waitFor(() =>
      expect(screen.getByTestId("sidebar-bundle-user_bundle")).toBeInTheDocument(),
    );
    const sysRow = screen.getByTestId("sidebar-bundle-_gigi_log");
    expect(sysRow.closest("details")).toBeTruthy();
  });

  it("fires onNewBundle when the + button is clicked", async () => {
    const fakeFetch = vi
      .fn()
      .mockResolvedValue(jsonResponse([{ name: "x", records: 1, fields: 1 }])) as unknown as Fetcher;
    const client = new SheetsClient({ baseUrl: "http://localhost:3142", fetch: fakeFetch });
    const onNewBundle = vi.fn();
    render(
      <Sidebar client={client} currentBundle="x" signedIn onNewBundle={onNewBundle} />,
    );
    await waitFor(() => expect(screen.getByTestId("sidebar-new-bundle")).toBeInTheDocument());
    fireEvent.click(screen.getByTestId("sidebar-new-bundle"));
    expect(onNewBundle).toHaveBeenCalledOnce();
  });

  it("surfaces engine errors gracefully", async () => {
    const fakeFetch = vi
      .fn()
      .mockResolvedValue(new Response("boom", { status: 500 })) as unknown as Fetcher;
    const client = new SheetsClient({ baseUrl: "http://localhost:3142", fetch: fakeFetch });
    render(<Sidebar client={client} currentBundle="x" signedIn />);
    await waitFor(() => expect(screen.getByTestId("sidebar-error")).toBeInTheDocument());
  });

  it("renders saved views for the current bundle from localStorage", async () => {
    localStorage.setItem(
      "gigi.sheets.views",
      JSON.stringify({
        v: 1,
        views: [
          {
            id: "v1",
            name: "My anomalies",
            bundle: "iris",
            spec: { v: 1, coverField: "species" },
            createdAt: Date.now(),
          },
          {
            id: "v2",
            name: "Other bundle view",
            bundle: "other",
            spec: { v: 1 },
            createdAt: Date.now(),
          },
        ],
      }),
    );
    const fakeFetch = vi
      .fn()
      .mockResolvedValue(jsonResponse([{ name: "iris", records: 150, fields: 6 }])) as unknown as Fetcher;
    const client = new SheetsClient({ baseUrl: "http://localhost:3142", fetch: fakeFetch });
    render(<Sidebar client={client} currentBundle="iris" signedIn />);
    expect(screen.getByTestId("sidebar-view-v1")).toBeInTheDocument();
    expect(screen.queryByTestId("sidebar-view-v2")).toBeNull();
  });
});

describe("Sidebar — guest", () => {
  it("renders the CTA card instead of the bundles list when not signed in", () => {
    // Even if the client is set up, we should NOT fetch — guests don't
    // get the bundles enumeration.
    const fakeFetch = vi.fn() as unknown as Fetcher;
    const client = new SheetsClient({ baseUrl: "http://localhost:3142", fetch: fakeFetch });
    render(<Sidebar client={client} currentBundle="iris" signedIn={false} />);
    expect(screen.getByTestId("sidebar-cta")).toBeInTheDocument();
    expect(screen.queryByTestId("sidebar-bundle-list")).toBeNull();
    expect(screen.getByTestId("sidebar")).toHaveAttribute("data-mode", "guest");
    // No fetch should have fired.
    expect(fakeFetch).not.toHaveBeenCalled();
  });

  it("clicking the CTA fires onSignIn", () => {
    const fakeFetch = vi.fn() as unknown as Fetcher;
    const client = new SheetsClient({ baseUrl: "http://localhost:3142", fetch: fakeFetch });
    const onSignIn = vi.fn();
    render(
      <Sidebar
        client={client}
        currentBundle="iris"
        signedIn={false}
        onSignIn={onSignIn}
      />,
    );
    fireEvent.click(screen.getByTestId("sidebar-cta-signin"));
    expect(onSignIn).toHaveBeenCalledOnce();
  });

  it("the CTA lists what signing in unlocks (saved views, sync, Prism workflows)", () => {
    const fakeFetch = vi.fn() as unknown as Fetcher;
    const client = new SheetsClient({ baseUrl: "http://localhost:3142", fetch: fakeFetch });
    render(
      <Sidebar
        client={client}
        currentBundle="iris"
        signedIn={false}
        onSignIn={() => {}}
      />,
    );
    const cta = screen.getByTestId("sidebar-cta");
    expect(cta).toHaveTextContent(/sync bundles/i);
    expect(cta).toHaveTextContent(/save views/i);
    expect(cta).toHaveTextContent(/prism/i);
  });
});
