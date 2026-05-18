import { describe, expect, it, vi } from "vitest";
import { fireEvent, render, screen } from "@testing-library/react";
import { LandingPage } from "../../src/components/LandingPage";
import { SheetsClient, type Fetcher } from "../../src/lib/gigi-client";

function makeClient(): SheetsClient {
  const fakeFetch = vi.fn().mockResolvedValue(
    new Response(JSON.stringify([]), {
      status: 200,
      headers: { "content-type": "application/json" },
    }),
  ) as unknown as Fetcher;
  return new SheetsClient({ baseUrl: "http://localhost:3142", fetch: fakeFetch });
}

describe("LandingPage", () => {
  it("renders the hero pitch with the Davis identity", () => {
    render(<LandingPage client={makeClient()} />);
    expect(screen.getByTestId("landing-page")).toBeInTheDocument();
    // Hero headline is rendered.
    expect(
      screen.getByRole("heading", { level: 1 }),
    ).toHaveTextContent(/spreadsheets made with real math/i);
    // The Davis identity appears in multiple places (hero meta + pillar tag
    // + math strip). Assert at least one is present.
    expect(screen.getAllByText(/S = \(1 \+ cos/i).length).toBeGreaterThan(0);
  });

  it("renders the comparison table with Excel · Airtable · GIGI columns", () => {
    render(<LandingPage client={makeClient()} />);
    const table = screen.getByTestId("landing-compare");
    expect(table).toBeInTheDocument();
    expect(table).toHaveTextContent("Excel");
    expect(table).toHaveTextContent("Airtable");
    expect(table).toHaveTextContent("GIGI Sheets");
    // Every row has GIGI's column winning — check a few load-bearing ones.
    expect(table).toHaveTextContent(/sameness-to-pivot/i);
    expect(table).toHaveTextContent(/det · ored · opaque/i);
    expect(table).toHaveTextContent(/=SAME, =K, =DIST, =COHORT/);
  });

  it("renders all sixteen feature cards", () => {
    render(<LandingPage client={makeClient()} />);
    const expected = [
      "Sort",
      "Filter",
      "Find & replace",
      "Range select",
      "Copy / paste",
      "Drag-fill",
      "Freeze cols",
      "Conditional fmt",
      "Number / date fmt",
      "Multi-select",
      "Linked records",
      "Per-view state",
      "Calendar",
      "Gallery",
      "Form view",
      "Formula bar",
    ];
    for (const name of expected) {
      expect(
        screen.getByRole("heading", { level: 4, name }),
      ).toBeInTheDocument();
    }
  });

  it("opens sign-in when the nav or final CTA is clicked", () => {
    const onSignInClick = vi.fn();
    render(
      <LandingPage client={makeClient()} onSignInClick={onSignInClick} />,
    );
    fireEvent.click(screen.getByTestId("landing-signin"));
    expect(onSignInClick).toHaveBeenCalledTimes(1);
    fireEvent.click(screen.getByTestId("landing-signin-final"));
    expect(onSignInClick).toHaveBeenCalledTimes(2);
  });

  it("does NOT ship any 'coming soon' badge — every feature is treated as live", () => {
    render(<LandingPage client={makeClient()} />);
    const root = screen.getByTestId("landing-page");
    expect(root.textContent ?? "").not.toMatch(/coming soon/i);
    expect(root.textContent ?? "").not.toMatch(/in beta/i);
    expect(root.textContent ?? "").not.toMatch(/roadmap/i);
  });
});
