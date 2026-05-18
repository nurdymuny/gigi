import { describe, expect, it, vi } from "vitest";
import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { GqlView } from "../../src/components/GqlView";
import {
  SheetsClient,
  type Fetcher,
} from "../../src/lib/gigi-client";

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

describe("GqlView — controlled editor", () => {
  it("reflects the value prop in the textarea", () => {
    render(
      <GqlView
        client={client(vi.fn())}
        query="SECTION sensors;"
        onQueryChange={() => {}}
      />,
    );
    const ta = screen.getByTestId("gql-editor") as HTMLTextAreaElement;
    expect(ta.value).toBe("SECTION sensors;");
  });

  it("calls onQueryChange when the user types", () => {
    const onQueryChange = vi.fn();
    render(
      <GqlView
        client={client(vi.fn())}
        query=""
        onQueryChange={onQueryChange}
      />,
    );
    const ta = screen.getByTestId("gql-editor") as HTMLTextAreaElement;
    fireEvent.change(ta, { target: { value: "SECTION sensors;" } });
    expect(onQueryChange).toHaveBeenCalledWith("SECTION sensors;");
  });

  it("Format button reformats the current query via onQueryChange", () => {
    const onQueryChange = vi.fn();
    render(
      <GqlView
        client={client(vi.fn())}
        query="SECTION sensors WHERE x=1 LIMIT 5;"
        onQueryChange={onQueryChange}
      />,
    );
    fireEvent.click(screen.getByTestId("gql-format"));
    expect(onQueryChange).toHaveBeenCalledWith(
      "SECTION sensors\nWHERE x=1\nLIMIT 5;",
    );
  });

  it("disables Run + Format when the query is empty", () => {
    render(
      <GqlView
        client={client(vi.fn())}
        query="   "
        onQueryChange={() => {}}
      />,
    );
    expect(screen.getByTestId("gql-run")).toBeDisabled();
    expect(screen.getByTestId("gql-format")).toBeDisabled();
  });
});

describe("GqlView — running queries", () => {
  it("Run button POSTs the query and renders the rows table", async () => {
    const fetcher = vi.fn().mockResolvedValue(
      jsonResponse({
        rows: [
          { sensor_id: "S-001", temp: 22.5 },
          { sensor_id: "S-002", temp: 19.3 },
        ],
        count: 2,
      }),
    );
    render(
      <GqlView
        client={client(fetcher)}
        query="SECTION sensors;"
        onQueryChange={() => {}}
      />,
    );
    fireEvent.click(screen.getByTestId("gql-run"));
    await waitFor(() =>
      expect(screen.getByTestId("gql-table")).toBeInTheDocument(),
    );
    expect(screen.getAllByTestId("gql-tr")).toHaveLength(2);
    expect(screen.getByTestId("gql-th-sensor_id")).toBeInTheDocument();
    expect(screen.getByTestId("gql-th-temp")).toBeInTheDocument();
    expect(screen.getByTestId("meta-rows")).toHaveTextContent("2");
    expect(screen.getByTestId("meta-status")).toHaveTextContent("200");
  });

  it("⌘↵ inside the textarea triggers the same Run", async () => {
    const fetcher = vi.fn().mockResolvedValue(
      jsonResponse({ rows: [{ a: 1 }], count: 1 }),
    );
    render(
      <GqlView
        client={client(fetcher)}
        query="SECTION sensors;"
        onQueryChange={() => {}}
      />,
    );
    fireEvent.keyDown(screen.getByTestId("gql-editor"), {
      key: "Enter",
      metaKey: true,
    });
    await waitFor(() => expect(fetcher).toHaveBeenCalled());
    expect(JSON.parse(fetcher.mock.calls[0][1].body)).toEqual({
      query: "SECTION sensors;",
    });
  });

  it("renders an engine error message when the body contains { error }", async () => {
    const fetcher = vi.fn().mockResolvedValue(
      jsonResponse({ error: "Parse error: unexpected token at 'BAD'" }, 400),
    );
    render(
      <GqlView
        client={client(fetcher)}
        query="BAD GQL;"
        onQueryChange={() => {}}
      />,
    );
    fireEvent.click(screen.getByTestId("gql-run"));
    await waitFor(() =>
      expect(screen.getByTestId("gql-result-engine-msg")).toBeInTheDocument(),
    );
    expect(screen.getByRole("alert")).toHaveTextContent(/Parse error/);
    expect(screen.getByTestId("meta-status")).toHaveTextContent("400");
  });

  it("renders an 'affected' card for write-style responses", async () => {
    const fetcher = vi.fn().mockResolvedValue(
      jsonResponse({ status: "ok", affected: 7 }),
    );
    render(
      <GqlView
        client={client(fetcher)}
        query="DELETE FROM sensors WHERE site_id='N';"
        onQueryChange={() => {}}
      />,
    );
    fireEvent.click(screen.getByTestId("gql-run"));
    await waitFor(() =>
      expect(screen.getByTestId("gql-result-affected")).toBeInTheDocument(),
    );
    expect(screen.getByTestId("gql-result-affected")).toHaveTextContent("7");
  });

  it("renders a 'no rows' message when the rows array is empty", async () => {
    const fetcher = vi.fn().mockResolvedValue(
      jsonResponse({ rows: [], count: 0 }),
    );
    render(
      <GqlView
        client={client(fetcher)}
        query="SECTION sensors WHERE x=999;"
        onQueryChange={() => {}}
      />,
    );
    fireEvent.click(screen.getByTestId("gql-run"));
    await waitFor(() =>
      expect(screen.getByTestId("gql-result-zero-rows")).toBeInTheDocument(),
    );
  });

  it("surfaces meta κ and conf when present in the response", async () => {
    const fetcher = vi.fn().mockResolvedValue(
      jsonResponse({
        rows: [{ a: 1 }],
        count: 1,
        curvature: 0.412,
        confidence: 0.78,
      }),
    );
    render(
      <GqlView
        client={client(fetcher)}
        query="SECTION sensors LIMIT 1;"
        onQueryChange={() => {}}
      />,
    );
    fireEvent.click(screen.getByTestId("gql-run"));
    await waitFor(() => expect(screen.getByTestId("meta-kappa")).toBeInTheDocument());
    expect(screen.getByTestId("meta-kappa")).toHaveTextContent("0.412");
    expect(screen.getByTestId("meta-conf")).toHaveTextContent("0.780");
  });

  it("renders union of keys when rows have different shapes", async () => {
    const fetcher = vi.fn().mockResolvedValue(
      jsonResponse({
        rows: [
          { a: 1, b: 2 },
          { a: 3, c: 4 },
        ],
        count: 2,
      }),
    );
    render(
      <GqlView
        client={client(fetcher)}
        query="SECTION mixed;"
        onQueryChange={() => {}}
      />,
    );
    fireEvent.click(screen.getByTestId("gql-run"));
    await waitFor(() => expect(screen.getByTestId("gql-table")).toBeInTheDocument());
    expect(screen.getByTestId("gql-th-a")).toBeInTheDocument();
    expect(screen.getByTestId("gql-th-b")).toBeInTheDocument();
    expect(screen.getByTestId("gql-th-c")).toBeInTheDocument();
  });
});
