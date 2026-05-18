import { describe, expect, it, vi } from "vitest";
import {
  fireEvent,
  render,
  screen,
  waitFor,
} from "@testing-library/react";
import { WorkflowPicker } from "../../src/components/WorkflowPicker";
import type { Fetcher } from "../../src/lib/gigi-client";
import { SheetsClient } from "../../src/lib/gigi-client";

function makeClient(): SheetsClient {
  // Fresh Response per call — Response bodies can only be consumed once,
  // and the apply-workflow flow makes N requests (1 createBundle + N inserts).
  const fakeFetch = vi
    .fn()
    .mockImplementation(async () =>
      new Response(JSON.stringify({ ok: true }), {
        status: 200,
        headers: { "content-type": "application/json" },
      }),
    ) as unknown as Fetcher;
  return new SheetsClient({ baseUrl: "http://localhost:3142", fetch: fakeFetch });
}

describe("WorkflowPicker", () => {
  it("renders the header + 6 workflow cards", () => {
    render(<WorkflowPicker client={makeClient()} onApplied={() => undefined} />);
    expect(screen.getByTestId("workflow-picker")).toBeInTheDocument();
    // Each template gets a card with testid `workflow-<id>`.
    expect(screen.getByTestId("workflow-project_tracker")).toBeInTheDocument();
    expect(screen.getByTestId("workflow-content_calendar")).toBeInTheDocument();
    expect(screen.getByTestId("workflow-crm")).toBeInTheDocument();
    expect(screen.getByTestId("workflow-event_planning")).toBeInTheDocument();
    expect(screen.getByTestId("workflow-inventory")).toBeInTheDocument();
    expect(screen.getByTestId("workflow-recruiting")).toBeInTheDocument();
  });

  it("shows the GIGI edge callout on every card", () => {
    render(<WorkflowPicker client={makeClient()} onApplied={() => undefined} />);
    const callouts = screen.getAllByText("GIGI edge");
    expect(callouts).toHaveLength(6);
  });

  it("clicking apply on a workflow calls onApplied with the default bundle", async () => {
    const onApplied = vi.fn();
    render(
      <WorkflowPicker client={makeClient()} onApplied={onApplied} />,
    );
    fireEvent.click(screen.getByTestId("workflow-apply-project_tracker"));
    await waitFor(() => expect(onApplied).toHaveBeenCalled());
    expect(onApplied.mock.calls[0][0]).toBe("workflow_projects");
    expect(onApplied.mock.calls[0][1].id).toBe("project_tracker");
  });

  it("surfaces an error if bundle creation fails", async () => {
    const fakeFetch = vi.fn().mockImplementation(async () =>
      new Response(JSON.stringify({ error: "conflict" }), {
        status: 409,
        headers: { "content-type": "application/json" },
      }),
    ) as unknown as Fetcher;
    const client = new SheetsClient({
      baseUrl: "http://localhost:3142",
      fetch: fakeFetch,
    });
    const onApplied = vi.fn();
    render(<WorkflowPicker client={client} onApplied={onApplied} />);
    fireEvent.click(screen.getByTestId("workflow-apply-inventory"));
    await waitFor(() =>
      expect(
        screen.getByTestId("workflow-inventory").getAttribute("data-state"),
      ).toBe("error"),
    );
    expect(onApplied).not.toHaveBeenCalled();
  });
});
