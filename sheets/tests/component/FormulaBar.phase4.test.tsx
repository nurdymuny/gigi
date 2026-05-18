import { describe, expect, it, vi } from "vitest";
import { act, fireEvent, render, screen } from "@testing-library/react";
import { FormulaBar } from "../../src/components/FormulaBar";
import type { FormulaContext } from "../../src/lib/formula";

/**
 * Phase 4 polish tests for FormulaBar:
 *   - focusToken / prefill (Insert → Formula path)
 *   - Tab / Shift+Enter / Escape key handling
 *   - viewStatus pill renders + carries tooltip
 *
 * Phase 4.A's "mirror selected cell" path is already covered by the
 * existing FormulaBar tests (the `initial` prop).
 */

function makeCtx(): FormulaContext {
  return {
    cell: () => null,
    sameness: () => 0.5,
    kappa: () => 0,
    cohort: () => "",
  };
}

describe("FormulaBar · focusToken + prefill (Insert → Formula)", () => {
  it("bumping focusToken with a prefill replaces the input value", async () => {
    // Mount without a token: the input stays at its `initial` value
    // (here, the empty default), no prefill yet.
    const { rerender } = render(
      <FormulaBar context={makeCtx()} prefill="=" />,
    );
    expect((screen.getByTestId("formula-input") as HTMLInputElement).value).toBe("");

    // First bump → prefill lands.
    rerender(<FormulaBar context={makeCtx()} focusToken={1} prefill="=" />);
    await new Promise((r) => requestAnimationFrame(() => r(undefined)));
    expect((screen.getByTestId("formula-input") as HTMLInputElement).value).toBe("=");

    // Subsequent bumps with a different prefill replace again.
    rerender(<FormulaBar context={makeCtx()} focusToken={2} prefill="=SUM(" />);
    await new Promise((r) => requestAnimationFrame(() => r(undefined)));
    expect((screen.getByTestId("formula-input") as HTMLInputElement).value).toBe("=SUM(");
  });

  it("focuses the input when focusToken bumps", async () => {
    const { rerender } = render(<FormulaBar context={makeCtx()} />);
    rerender(<FormulaBar context={makeCtx()} focusToken={1} />);
    await new Promise((r) => requestAnimationFrame(() => r(undefined)));
    expect(document.activeElement).toBe(screen.getByTestId("formula-input"));
  });
});

describe("FormulaBar · Tab / Shift+Enter / Escape", () => {
  it("Tab commits with move='right'", () => {
    const onCommit = vi.fn();
    render(<FormulaBar context={makeCtx()} onCommit={onCommit} />);
    const input = screen.getByTestId("formula-input");
    fireEvent.change(input, { target: { value: "=1+1" } });
    fireEvent.keyDown(input, { key: "Tab" });
    expect(onCommit).toHaveBeenCalledTimes(1);
    expect(onCommit.mock.calls[0][2]).toBe("right");
  });

  it("Shift+Enter commits with move=null (no advance)", () => {
    const onCommit = vi.fn();
    render(<FormulaBar context={makeCtx()} onCommit={onCommit} />);
    const input = screen.getByTestId("formula-input");
    fireEvent.change(input, { target: { value: "=1+1" } });
    fireEvent.keyDown(input, { key: "Enter", shiftKey: true });
    expect(onCommit.mock.calls[0][2]).toBeNull();
  });

  it("plain Enter commits with move='down'", () => {
    const onCommit = vi.fn();
    render(<FormulaBar context={makeCtx()} onCommit={onCommit} />);
    const input = screen.getByTestId("formula-input");
    fireEvent.change(input, { target: { value: "=1+1" } });
    fireEvent.keyDown(input, { key: "Enter" });
    expect(onCommit.mock.calls[0][2]).toBe("down");
  });

  it("Escape restores the initial value and blurs", () => {
    const onCommit = vi.fn();
    render(<FormulaBar context={makeCtx()} onCommit={onCommit} initial="=A1" />);
    const input = screen.getByTestId("formula-input") as HTMLInputElement;
    // User starts editing
    act(() => {
      input.focus();
    });
    fireEvent.change(input, { target: { value: "=ALL CHANGED" } });
    expect(input.value).toBe("=ALL CHANGED");
    fireEvent.keyDown(input, { key: "Escape" });
    expect(input.value).toBe("=A1");
    expect(onCommit).not.toHaveBeenCalled();
  });
});

describe("FormulaBar · range-selection stats (Phase 4 finisher)", () => {
  it("renders the stats strip when rangeStats is set", () => {
    render(
      <FormulaBar
        context={makeCtx()}
        rangeStats={{
          count: 5,
          numericCount: 5,
          sum: 100,
          avg: 20,
          min: 10,
          max: 30,
          field: "amount",
        }}
      />,
    );
    const strip = screen.getByTestId("formula-range-stats");
    expect(strip).toBeInTheDocument();
    expect(strip).toHaveTextContent("Count");
    expect(strip).toHaveTextContent("5");
    expect(strip).toHaveTextContent("Sum");
    expect(strip).toHaveTextContent("100");
    expect(strip).toHaveTextContent("Avg");
    expect(strip).toHaveTextContent("20");
    expect(strip).toHaveTextContent("Min");
    expect(strip).toHaveTextContent("10");
    expect(strip).toHaveTextContent("Max");
    expect(strip).toHaveTextContent("30");
    // Result panel is replaced — not in the DOM.
    expect(screen.queryByTestId("formula-result")).toBeNull();
  });

  it("shows only Count when no numeric values are in scope", () => {
    render(
      <FormulaBar
        context={makeCtx()}
        rangeStats={{ count: 3, numericCount: 0, field: "label" }}
      />,
    );
    const strip = screen.getByTestId("formula-range-stats");
    expect(strip).toHaveTextContent("Count");
    expect(strip).toHaveTextContent("3");
    expect(strip).not.toHaveTextContent("Sum");
    expect(strip).not.toHaveTextContent("Avg");
  });

  it("falls back to the formula-result panel when rangeStats is null", () => {
    render(<FormulaBar context={makeCtx()} rangeStats={null} />);
    expect(screen.getByTestId("formula-result")).toBeInTheDocument();
    expect(screen.queryByTestId("formula-range-stats")).toBeNull();
  });
});

describe("FormulaBar · viewStatus indicator (Phase 4.E)", () => {
  it("renders the pill when viewStatus is set", () => {
    render(
      <FormulaBar
        context={makeCtx()}
        viewStatus={{ label: "Filtered · 30 of 150", tooltip: "Cell refs resolve against the visible view." }}
      />,
    );
    const pill = screen.getByTestId("formula-view-status");
    expect(pill).toBeInTheDocument();
    expect(pill).toHaveTextContent("Filtered · 30 of 150");
    expect(pill.getAttribute("title")).toMatch(/visible view/);
  });

  it("does NOT render the pill when viewStatus is null", () => {
    render(<FormulaBar context={makeCtx()} viewStatus={null} />);
    expect(screen.queryByTestId("formula-view-status")).toBeNull();
  });
});
