import { describe, expect, it, vi } from "vitest";
import { fireEvent, render, screen } from "@testing-library/react";
import { GqlView } from "../../src/components/GqlView";
import { SheetsClient, type BundleSchema, type Fetcher } from "../../src/lib/gigi-client";

/**
 * GQL panel sample chips — drop ready-to-run queries into the editor
 * targeted at the active bundle. Helps users discover the GIGI
 * primitives (CURVATURE / BETTI / SPECTRAL / SECTION / INTEGRATE /
 * HOLONOMY / TRANSPORT) without having to memorize the syntax.
 */

const fetcher = vi.fn() as unknown as Fetcher;
const fakeClient = new SheetsClient({ baseUrl: "http://localhost:3142", fetch: fetcher });

const SCHEMA: BundleSchema = {
  name: "nba_2024",
  base_fields: [{ name: "team", type: "text" }],
  fiber_fields: [
    { name: "conference", type: "categorical" },
    { name: "wins", type: "numeric" },
    { name: "points_scored", type: "numeric" },
  ],
  indexed_fields: ["team"],
  records: 30,
  storage_mode: "mmap",
} as unknown as BundleSchema;

describe("GqlView · sample chips", () => {
  it("renders no chip strip when there's no schema (picker / loading)", () => {
    render(
      <GqlView client={fakeClient} query="" onQueryChange={() => undefined} />,
    );
    expect(screen.queryByTestId("gql-samples")).toBeNull();
  });

  it("renders chips for the bundle-wide primitives even with no row keys yet", () => {
    render(
      <GqlView
        client={fakeClient}
        query=""
        onQueryChange={() => undefined}
        schema={SCHEMA}
      />,
    );
    expect(screen.getByTestId("gql-samples")).toBeInTheDocument();
    expect(screen.getByTestId("gql-sample-curvature")).toBeInTheDocument();
    expect(screen.getByTestId("gql-sample-betti")).toBeInTheDocument();
    expect(screen.getByTestId("gql-sample-spectral")).toBeInTheDocument();
    // SECTION + TRANSPORT need row keys — should be missing.
    expect(screen.queryByTestId("gql-sample-section")).toBeNull();
    expect(screen.queryByTestId("gql-sample-transport")).toBeNull();
  });

  it("clicking a chip drops the assembled query into the editor", () => {
    const onQueryChange = vi.fn();
    render(
      <GqlView
        client={fakeClient}
        query=""
        onQueryChange={onQueryChange}
        schema={SCHEMA}
        coverField="conference"
        sampleRowKey="BOS"
        secondRowKey="LAL"
      />,
    );
    fireEvent.click(screen.getByTestId("gql-sample-curvature"));
    expect(onQueryChange).toHaveBeenLastCalledWith("CURVATURE nba_2024;");
  });

  it("the transport chip uses both row keys + two fiber fields", () => {
    const onQueryChange = vi.fn();
    render(
      <GqlView
        client={fakeClient}
        query=""
        onQueryChange={onQueryChange}
        schema={SCHEMA}
        coverField="conference"
        sampleRowKey="BOS"
        secondRowKey="LAL"
      />,
    );
    fireEvent.click(screen.getByTestId("gql-sample-transport"));
    expect(onQueryChange).toHaveBeenLastCalledWith(
      "TRANSPORT nba_2024 FROM (team='BOS') TO (team='LAL') ON FIBER (wins, points_scored);",
    );
  });

  it("hovering a chip shows the description + full query in the tooltip", () => {
    render(
      <GqlView
        client={fakeClient}
        query=""
        onQueryChange={() => undefined}
        schema={SCHEMA}
        coverField="conference"
        sampleRowKey="BOS"
      />,
    );
    const section = screen.getByTestId("gql-sample-section");
    const title = section.getAttribute("title") ?? "";
    expect(title).toMatch(/Point query/);
    expect(title).toMatch(/SECTION nba_2024 AT team='BOS'/);
  });

  it("INTEGRATE chip emits the bundle-first MEASURE form (engine grammar regression test)", () => {
    // Regression test for the screenshot bug: the chip used to generate
    // `INTEGRATE <field> OVER <bundle> COVER ALL;` which the engine
    // rejected with "No bundle: <field>". The correct form puts the
    // bundle first and the numeric field inside MEASURE(...).
    const onQueryChange = vi.fn();
    render(
      <GqlView
        client={fakeClient}
        query=""
        onQueryChange={onQueryChange}
        schema={SCHEMA}
        coverField="conference"
        sampleRowKey="BOS"
      />,
    );
    fireEvent.click(screen.getByTestId("gql-sample-integrate"));
    expect(onQueryChange).toHaveBeenLastCalledWith(
      "INTEGRATE nba_2024 OVER conference MEASURE AVG(wins);",
    );
  });
});
