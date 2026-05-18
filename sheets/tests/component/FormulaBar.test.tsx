import { describe, expect, it, vi } from "vitest";
import { fireEvent, render, screen } from "@testing-library/react";
import { FormulaBar } from "../../src/components/FormulaBar";
import type { FormulaContext } from "../../src/lib/formula";

function makeCtx(): FormulaContext {
  return {
    cell: (ref) => (ref === "A1" ? 10 : ref === "A2" ? 20 : null),
    sameness: (a, b) => (a === b ? 1 : 0.5),
    kappa: () => 0.1,
    cohort: (col) => `cohort:${col}`,
  };
}

describe("FormulaBar", () => {
  it("renders the fx label + input + empty result panel", () => {
    render(<FormulaBar context={makeCtx()} />);
    expect(screen.getByTestId("formula-bar")).toBeInTheDocument();
    expect(screen.getByTestId("formula-input")).toBeInTheDocument();
    expect(screen.getByTestId("formula-result")).toHaveTextContent("");
  });

  it("evaluates arithmetic live as the user types", () => {
    render(<FormulaBar context={makeCtx()} />);
    fireEvent.change(screen.getByTestId("formula-input"), {
      target: { value: "=1+2*3" },
    });
    expect(screen.getByTestId("formula-result")).toHaveTextContent("7");
  });

  it("evaluates a cell reference", () => {
    render(<FormulaBar context={makeCtx()} />);
    fireEvent.change(screen.getByTestId("formula-input"), {
      target: { value: "=A1+A2" },
    });
    expect(screen.getByTestId("formula-result")).toHaveTextContent("30");
  });

  it("evaluates =SAME(A1, A1) as 1", () => {
    render(<FormulaBar context={makeCtx()} />);
    fireEvent.change(screen.getByTestId("formula-input"), {
      target: { value: "=SAME(A1, A1)" },
    });
    expect(screen.getByTestId("formula-result")).toHaveTextContent("1");
  });

  it("surfaces #NAME! for an unknown function", () => {
    render(<FormulaBar context={makeCtx()} />);
    fireEvent.change(screen.getByTestId("formula-input"), {
      target: { value: "=NOPE(1)" },
    });
    expect(screen.getByTestId("formula-result")).toHaveTextContent("#NAME!");
  });

  it("calls onCommit with the formula + result on Enter", () => {
    const onCommit = vi.fn();
    render(<FormulaBar context={makeCtx()} onCommit={onCommit} />);
    const input = screen.getByTestId("formula-input");
    fireEvent.change(input, { target: { value: "=A1*2" } });
    fireEvent.keyDown(input, { key: "Enter" });
    expect(onCommit).toHaveBeenCalledTimes(1);
    expect(onCommit.mock.calls[0][0]).toBe("=A1*2");
    expect(onCommit.mock.calls[0][1].value).toBe(20);
  });

  it("does not show a result panel for plain (non-formula) text", () => {
    render(<FormulaBar context={makeCtx()} />);
    fireEvent.change(screen.getByTestId("formula-input"), {
      target: { value: "hello" },
    });
    // No "=" prefix means it's just a raw value, no live computation.
    expect(screen.getByTestId("formula-result")).toHaveTextContent("");
  });
});
