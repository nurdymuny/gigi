import { describe, expect, it, vi } from "vitest";
import {
  buildProjectTrackerTour,
  registerTourHelpers,
} from "../../src/lib/tutorial-scripts";
import { SheetsClient, type Fetcher } from "../../src/lib/gigi-client";

function makeClient(): SheetsClient {
  const fakeFetch = vi.fn().mockImplementation(async () =>
    new Response(JSON.stringify({ ok: true }), {
      status: 200,
      headers: { "content-type": "application/json" },
    }),
  ) as unknown as Fetcher;
  return new SheetsClient({ baseUrl: "http://localhost:3142", fetch: fakeFetch });
}

describe("tutorial-scripts · buildProjectTrackerTour", () => {
  it("returns a non-empty sequence of well-formed steps", () => {
    const steps = buildProjectTrackerTour({
      client: makeClient(),
      navigateToBundle: vi.fn(),
    });
    expect(steps.length).toBeGreaterThan(4);
    for (const s of steps) {
      expect(s.title).toBeTruthy();
      expect(s.body).toBeTruthy();
    }
  });

  it("step 2 (loader) calls navigateToBundle with workflow_projects", async () => {
    const navigateToBundle = vi.fn();
    const steps = buildProjectTrackerTour({
      client: makeClient(),
      navigateToBundle,
    });
    // The "Loading the workflow…" step has the apply action.
    const loaderStep = steps.find((s) =>
      String(s.title).toLowerCase().includes("loading"),
    );
    expect(loaderStep).toBeDefined();
    await loaderStep!.action!();
    expect(navigateToBundle).toHaveBeenCalledWith("workflow_projects");
  });

  it("kanban / gallery / form / grid steps call into the tour registry's setActiveView", async () => {
    const steps = buildProjectTrackerTour({
      client: makeClient(),
      navigateToBundle: vi.fn(),
    });
    const setActiveView = vi.fn();
    const cleanup = registerTourHelpers({ setActiveView });

    const expectations: Array<[string, string]> = [
      ["kanban", "kanban"],
      ["gallery", "gallery"],
      ["form", "form"],
      ["grid", "grid"],
    ];
    for (const [titleSubstr, expectedView] of expectations) {
      const step = steps.find((s) =>
        String(s.title).toLowerCase().includes(titleSubstr),
      );
      expect(step, `step matching "${titleSubstr}"`).toBeDefined();
      if (step?.action) {
        setActiveView.mockClear();
        await step.action();
        expect(setActiveView).toHaveBeenCalledWith(expectedView);
      }
    }
    cleanup();
  });
});

describe("tutorial-scripts · registerTourHelpers", () => {
  it("returns a cleanup that clears the registered helpers", () => {
    const setActiveView = vi.fn();
    const cleanup = registerTourHelpers({ setActiveView });
    expect(window.__gigi_tour__?.setActiveView).toBe(setActiveView);
    cleanup();
    expect(window.__gigi_tour__?.setActiveView).toBeUndefined();
  });
});
