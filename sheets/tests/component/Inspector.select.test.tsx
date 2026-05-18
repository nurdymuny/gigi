import { describe, expect, it, vi } from "vitest";
import { render, screen, waitFor, fireEvent } from "@testing-library/react";
import { Inspector } from "../../src/components/Inspector";
import {
  SheetsClient,
  type BundleSchema,
  type Fetcher,
} from "../../src/lib/gigi-client";

const SCHEMA: BundleSchema = {
  name: "sensors",
  base_fields: [{ name: "sensor_id", type: "text" }],
  fiber_fields: [
    { name: "site_id", type: "categorical" },
    { name: "temp", type: "numeric" },
    { name: "humidity", type: "numeric" },
  ],
  indexed_fields: ["sensor_id"],
  records: 4,
  storage_mode: "mmap",
};

const ROW = {
  sensor_id: "S-0142",
  site_id: "North-3",
  temp: 38.7,
  humidity: 18.2,
};

function makeClient(handlers: Record<string, () => Response>) {
  const fetcher: Fetcher = vi.fn(async (input) => {
    const url = String(input);
    for (const [pattern, handler] of Object.entries(handlers)) {
      if (url.includes(pattern)) return handler();
    }
    return new Response("not mocked: " + url, { status: 500 });
  }) as unknown as Fetcher;
  return new SheetsClient({ baseUrl: "http://localhost:3142", fetch: fetcher });
}

function jsonResponse(payload: unknown, status = 200) {
  return new Response(JSON.stringify(payload), {
    status,
    headers: { "content-type": "application/json" },
  });
}

describe("Inspector — selection + gauges", () => {
  it("renders the empty state when no row is selected", () => {
    const client = makeClient({});
    render(
      <Inspector
        client={client}
        bundle="sensors"
        schema={SCHEMA}
        selectedRow={null}
        keyField="sensor_id"
        coverField="site_id"
        fiberFields={["temp", "humidity"]}
        kappa={undefined}
      />,
    );
    expect(screen.getByTestId("inspector-empty")).toBeInTheDocument();
  });

  it("renders all four gauges with the selected row's κ", () => {
    const client = makeClient({});
    render(
      <Inspector
        client={client}
        bundle="sensors"
        schema={SCHEMA}
        selectedRow={ROW}
        keyField="sensor_id"
        coverField="site_id"
        fiberFields={["temp", "humidity"]}
        kappa={4.21}
        spectralLambda1={0.082}
      />,
    );
    expect(screen.getByTestId("gauge-kappa")).toHaveTextContent("4.21");
    // confidence = 1/(1+4.21) ≈ 0.19
    expect(screen.getByTestId("gauge-conf")).toHaveTextContent("0.19");
    // capacity = 1.98/4.21 ≈ 0.47
    expect(screen.getByTestId("gauge-capacity")).toHaveTextContent("0.47");
    // λ₁ promoted from prop
    expect(screen.getByTestId("gauge-lambda1")).toHaveTextContent("0.082");
  });

  it("classes the row 'anomaly' when κ ≥ 2.0", () => {
    const client = makeClient({});
    render(
      <Inspector
        client={client}
        bundle="sensors"
        schema={SCHEMA}
        selectedRow={ROW}
        keyField="sensor_id"
        coverField="site_id"
        fiberFields={["temp", "humidity"]}
        kappa={4.21}
      />,
    );
    expect(screen.getByTestId("insp-flag")).toHaveTextContent("anomaly");
  });

  it("classes the row 'healthy' when κ < 0.8", () => {
    const client = makeClient({});
    render(
      <Inspector
        client={client}
        bundle="sensors"
        schema={SCHEMA}
        selectedRow={ROW}
        keyField="sensor_id"
        coverField="site_id"
        fiberFields={["temp", "humidity"]}
        kappa={0.12}
      />,
    );
    expect(screen.getByTestId("insp-flag")).toHaveTextContent("healthy");
  });

  it("shows '—' for spectral λ₁ when none is supplied", () => {
    const client = makeClient({});
    render(
      <Inspector
        client={client}
        bundle="sensors"
        schema={SCHEMA}
        selectedRow={ROW}
        keyField="sensor_id"
        coverField="site_id"
        fiberFields={["temp", "humidity"]}
        kappa={0.5}
      />,
    );
    expect(screen.getByTestId("gauge-lambda1")).toHaveTextContent("—");
  });
});

describe("Inspector — verbs run against the engine", () => {
  it("clicking SPECTRAL calls /spectral and renders the result card", async () => {
    const client = makeClient({
      "/spectral": () =>
        jsonResponse({ lambda1: 0.18, diameter: 6, spectral_capacity: 0.55 }),
    });
    render(
      <Inspector
        client={client}
        bundle="sensors"
        schema={SCHEMA}
        selectedRow={ROW}
        keyField="sensor_id"
        coverField="site_id"
        fiberFields={["temp", "humidity"]}
        kappa={4.21}
      />,
    );
    fireEvent.click(screen.getByTestId("verb-spectral"));
    await waitFor(() =>
      expect(screen.getByTestId("result-spectral")).toBeInTheDocument(),
    );
    expect(screen.getByTestId("bar-λ₁")).toHaveTextContent("0.180");
  });

  it("clicking BETTI calls /betti and renders b₀ / b₁ / χ", async () => {
    const client = makeClient({
      "/betti": () => jsonResponse({ beta_0: 4, beta_1: 2 }),
    });
    render(
      <Inspector
        client={client}
        bundle="sensors"
        schema={SCHEMA}
        selectedRow={ROW}
        keyField="sensor_id"
        coverField="site_id"
        fiberFields={["temp", "humidity"]}
        kappa={4.21}
      />,
    );
    fireEvent.click(screen.getByTestId("verb-betti"));
    await waitFor(() =>
      expect(screen.getByTestId("result-betti")).toBeInTheDocument(),
    );
    expect(screen.getByTestId("betti-chi")).toHaveTextContent("2");
  });

  it("clicking TRANSPORT issues a GQL TRANSPORT query and renders the matrix", async () => {
    const client = makeClient({
      "/v1/gql": () =>
        jsonResponse({
          rows: [
            {
              dim: 2,
              angle: 0.523,
              matrix: [0.866, -0.5, 0.5, 0.866],
            },
          ],
          count: 1,
        }),
    });
    render(
      <Inspector
        client={client}
        bundle="sensors"
        schema={SCHEMA}
        selectedRow={ROW}
        keyField="sensor_id"
        coverField="site_id"
        fiberFields={["temp", "humidity"]}
        kappa={4.21}
      />,
    );
    fireEvent.click(screen.getByTestId("verb-transport"));
    await waitFor(() =>
      expect(screen.getByTestId("result-transport")).toBeInTheDocument(),
    );
    const matrix = screen.getByTestId("matrix");
    expect(matrix.children).toHaveLength(4);
    expect(screen.getByTestId("result-transport")).toHaveTextContent("S-0142");
  });

  it("clicking HOLONOMY issues a GQL HOLONOMY query around the cover field", async () => {
    const client = makeClient({
      "/v1/gql": () =>
        jsonResponse({
          rows: [
            { site_id: "N", temp: 22, humidity: 60, transport_angle: 0.1 },
            { site_id: "S", temp: 25, humidity: 55, transport_angle: 0.3 },
            {
              _type: "summary",
              holonomy_angle: 0.4,
              holonomy_trivial: false,
            },
          ],
          count: 3,
        }),
    });
    render(
      <Inspector
        client={client}
        bundle="sensors"
        schema={SCHEMA}
        selectedRow={ROW}
        keyField="sensor_id"
        coverField="site_id"
        fiberFields={["temp", "humidity"]}
        kappa={4.21}
      />,
    );
    fireEvent.click(screen.getByTestId("verb-holonomy"));
    await waitFor(() =>
      expect(screen.getByTestId("result-holonomy")).toBeInTheDocument(),
    );
    expect(screen.getByTestId("result-holonomy")).toHaveTextContent(/2 cohorts/);
  });

  it("surfaces engine errors in the alert region without crashing", async () => {
    const client = makeClient({
      "/spectral": () => new Response("boom", { status: 500 }),
    });
    render(
      <Inspector
        client={client}
        bundle="sensors"
        schema={SCHEMA}
        selectedRow={ROW}
        keyField="sensor_id"
        coverField="site_id"
        fiberFields={["temp", "humidity"]}
        kappa={4.21}
      />,
    );
    fireEvent.click(screen.getByTestId("verb-spectral"));
    await waitFor(() =>
      expect(screen.getByTestId("verb-error")).toBeInTheDocument(),
    );
    expect(screen.getByRole("alert")).toHaveTextContent(/500|http/i);
  });
});
