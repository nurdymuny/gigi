import { describe, expect, it, vi, beforeEach } from "vitest";
import { fireEvent, render, screen } from "@testing-library/react";
import { PrismWorkflowsDrawer } from "../../src/components/PrismWorkflows";
import type { PrismCredits } from "../../src/lib/use-prism-credits";
import type { BundleSchema } from "../../src/lib/gigi-client";

const SCHEMA: BundleSchema = {
  name: "sensors",
  base_fields: [{ name: "sensor_id", type: "text" }],
  fiber_fields: [
    { name: "site", type: "categorical" },
    { name: "temp", type: "numeric" },
    { name: "humidity", type: "numeric" },
  ],
  indexed_fields: ["sensor_id"],
  records: 4,
  storage_mode: "mmap",
};

const ROWS = [
  { sensor_id: "S-1", site: "North", temp: 22.5, humidity: 60 },
  { sensor_id: "S-2", site: "North", temp: 22.5, humidity: 60 }, // dup-ish
  { sensor_id: "S-3", site: "South", temp: 38.7, humidity: 18 },
  { sensor_id: "S-4", site: "South", temp: 22.0, humidity: 59 },
];

const KAPPA = new Map<string, number>([
  ["S-1", 0.2],
  ["S-2", 0.1],
  ["S-3", 4.5],
  ["S-4", 0.3],
]);

function makeCredits(over: Partial<PrismCredits> = {}): PrismCredits {
  return {
    used: 0,
    limit: 3,
    remaining: 3,
    unlimited: false,
    canRun: true,
    consume: vi.fn(),
    reset: vi.fn(),
    ...over,
  };
}

beforeEach(() => {
  localStorage.clear();
});

describe("PrismWorkflowsDrawer", () => {
  it("renders nothing when closed and no transient modals are pending", () => {
    render(
      <PrismWorkflowsDrawer
        open={false}
        onClose={() => {}}
        schema={SCHEMA}
        rows={ROWS}
        kappaMap={KAPPA}
        credits={makeCredits()}
        onSignIn={() => {}}
      />,
    );
    expect(screen.queryByTestId("prism-drawer")).toBeNull();
  });

  it("renders four workflow cards when open", () => {
    render(
      <PrismWorkflowsDrawer
        open
        onClose={() => {}}
        schema={SCHEMA}
        rows={ROWS}
        kappaMap={KAPPA}
        credits={makeCredits()}
        onSignIn={() => {}}
      />,
    );
    expect(screen.getByTestId("prism-workflow-dedup")).toBeInTheDocument();
    expect(screen.getByTestId("prism-workflow-forecast")).toBeInTheDocument();
    expect(screen.getByTestId("prism-workflow-monitor")).toBeInTheDocument();
    expect(screen.getByTestId("prism-workflow-books")).toBeInTheDocument();
  });

  it("shows the credit count in the footer", () => {
    render(
      <PrismWorkflowsDrawer
        open
        onClose={() => {}}
        schema={SCHEMA}
        rows={ROWS}
        kappaMap={KAPPA}
        credits={makeCredits({ remaining: 2 })}
        onSignIn={() => {}}
      />,
    );
    expect(screen.getByTestId("prism-credits")).toHaveTextContent(/2 of 3/i);
  });

  it("running a workflow consumes a credit and shows the result modal", () => {
    const consume = vi.fn();
    render(
      <PrismWorkflowsDrawer
        open
        onClose={() => {}}
        schema={SCHEMA}
        rows={ROWS}
        kappaMap={KAPPA}
        credits={makeCredits({ consume })}
        onSignIn={() => {}}
      />,
    );
    fireEvent.click(screen.getByTestId("prism-run-monitor"));
    expect(consume).toHaveBeenCalledOnce();
    expect(screen.getByTestId("prism-result-modal")).toBeInTheDocument();
    expect(screen.getByTestId("prism-result-title")).toHaveTextContent(/monitor/i);
  });

  it("opens the upsell modal when credits are exhausted", () => {
    render(
      <PrismWorkflowsDrawer
        open
        onClose={() => {}}
        schema={SCHEMA}
        rows={ROWS}
        kappaMap={KAPPA}
        credits={makeCredits({ used: 3, remaining: 0, canRun: false })}
        onSignIn={() => {}}
      />,
    );
    fireEvent.click(screen.getByTestId("prism-run-dedup"));
    expect(screen.getByTestId("prism-upsell-modal")).toBeInTheDocument();
  });

  it("upsell modal's sign-in button calls onSignIn", () => {
    const onSignIn = vi.fn();
    render(
      <PrismWorkflowsDrawer
        open
        onClose={() => {}}
        schema={SCHEMA}
        rows={ROWS}
        kappaMap={KAPPA}
        credits={makeCredits({ used: 3, remaining: 0, canRun: false })}
        onSignIn={onSignIn}
      />,
    );
    fireEvent.click(screen.getByTestId("prism-run-dedup"));
    fireEvent.click(screen.getByTestId("prism-upsell-signin"));
    expect(onSignIn).toHaveBeenCalledOnce();
  });

  it("monitor flags the κ-bad row in its result table", () => {
    render(
      <PrismWorkflowsDrawer
        open
        onClose={() => {}}
        schema={SCHEMA}
        rows={ROWS}
        kappaMap={KAPPA}
        credits={makeCredits()}
        onSignIn={() => {}}
      />,
    );
    fireEvent.click(screen.getByTestId("prism-run-monitor"));
    expect(screen.getByTestId("prism-result-table")).toHaveTextContent("S-3");
  });

  it("forecast picks the highest-variance numeric column and reports a trend", () => {
    render(
      <PrismWorkflowsDrawer
        open
        onClose={() => {}}
        schema={SCHEMA}
        rows={ROWS}
        kappaMap={KAPPA}
        credits={makeCredits()}
        onSignIn={() => {}}
      />,
    );
    fireEvent.click(screen.getByTestId("prism-run-forecast"));
    // Result modal should mention a field and a trend.
    const headline = screen.getByTestId("prism-result-headline");
    expect(headline.textContent).toMatch(/temp|humidity/);
  });
});
