import { describe, expect, it, vi } from "vitest";
import { fireEvent, render, screen } from "@testing-library/react";
import { FormulaPicker } from "../../src/components/FormulaPicker";
import type { FormulaContext } from "../../src/lib/formula";

function makeCtx(): FormulaContext {
  return {
    cell: (ref) => (ref === "A1" ? 10 : ref === "A2" ? 20 : ref === "A3" ? 30 : null),
    sameness: () => 0.5,
    kappa: () => 0,
    cohort: () => "",
  };
}

describe("FormulaPicker · list view", () => {
  it("renders nothing when closed", () => {
    render(<FormulaPicker open={false} onClose={() => {}} context={makeCtx()} />);
    expect(screen.queryByTestId("formula-picker")).toBeNull();
  });

  it("renders the search box + categories when open", () => {
    render(<FormulaPicker open={true} onClose={() => {}} context={makeCtx()} />);
    expect(screen.getByTestId("formula-picker")).toBeInTheDocument();
    expect(screen.getByTestId("formula-picker-search")).toBeInTheDocument();
    expect(screen.getByTestId("formula-picker-cats")).toBeInTheDocument();
  });

  it("shows function rows", () => {
    render(<FormulaPicker open={true} onClose={() => {}} context={makeCtx()} />);
    expect(screen.getByTestId("formula-picker-result-SUM")).toBeInTheDocument();
    expect(screen.getByTestId("formula-picker-result-IF")).toBeInTheDocument();
  });

  it("typing filters the list", () => {
    render(<FormulaPicker open={true} onClose={() => {}} context={makeCtx()} />);
    fireEvent.change(screen.getByTestId("formula-picker-search"), {
      target: { value: "sumif" },
    });
    expect(screen.getByTestId("formula-picker-result-SUMIF")).toBeInTheDocument();
    // SUM doesn't start with "sumif" so it shouldn't appear.
    expect(screen.queryByTestId("formula-picker-result-SUM")).toBeNull();
  });

  it("clicking a category filters the list to that category", () => {
    render(<FormulaPicker open={true} onClose={() => {}} context={makeCtx()} />);
    // Click the Geometry category.
    const geomBtn = screen
      .getAllByRole("button")
      .find((b) => b.textContent?.startsWith("Geometry"))!;
    fireEvent.click(geomBtn);
    // SAME/DIST should show; SUM should not.
    expect(screen.getByTestId("formula-picker-result-SAME")).toBeInTheDocument();
    expect(screen.queryByTestId("formula-picker-result-SUM")).toBeNull();
  });
});

describe("FormulaPicker · edit view", () => {
  it("clicking a function moves to the edit step", () => {
    render(<FormulaPicker open={true} onClose={() => {}} context={makeCtx()} />);
    fireEvent.click(
      screen.getByTestId("formula-picker-result-SUM").querySelector("button")!,
    );
    expect(screen.getByTestId("formula-picker-back")).toBeInTheDocument();
    expect(screen.getByTestId("formula-picker-arg-value1")).toBeInTheDocument();
  });

  it("Back returns to the list view", () => {
    render(<FormulaPicker open={true} onClose={() => {}} context={makeCtx()} />);
    fireEvent.click(
      screen.getByTestId("formula-picker-result-SUM").querySelector("button")!,
    );
    fireEvent.click(screen.getByTestId("formula-picker-back"));
    expect(screen.getByTestId("formula-picker-search")).toBeInTheDocument();
  });

  it("live preview updates as the user types args", () => {
    render(<FormulaPicker open={true} onClose={() => {}} context={makeCtx()} />);
    fireEvent.click(
      screen.getByTestId("formula-picker-result-SUM").querySelector("button")!,
    );
    fireEvent.change(screen.getByTestId("formula-picker-arg-value1"), {
      target: { value: "A1+A2" },
    });
    // The preview should show `=SUM(A1+A2)` evaluating to 30.
    expect(screen.getByTestId("formula-picker-preview-value")).toHaveTextContent(
      "30",
    );
  });

  it("preview surfaces the error when the formula doesn't parse", () => {
    render(<FormulaPicker open={true} onClose={() => {}} context={makeCtx()} />);
    fireEvent.click(
      screen.getByTestId("formula-picker-result-SUM").querySelector("button")!,
    );
    fireEvent.change(screen.getByTestId("formula-picker-arg-value1"), {
      target: { value: "A1 +" }, // dangling operator → #ERROR!
    });
    expect(screen.getByTestId("formula-picker-preview-err")).toHaveTextContent(
      "#ERROR!",
    );
  });

  it("Insert button is disabled until any arg is filled", () => {
    const onInsert = vi.fn();
    render(
      <FormulaPicker
        open={true}
        onClose={() => {}}
        context={makeCtx()}
        onInsert={onInsert}
      />,
    );
    fireEvent.click(
      screen.getByTestId("formula-picker-result-SUM").querySelector("button")!,
    );
    const insert = screen.getByTestId("formula-picker-insert") as HTMLButtonElement;
    expect(insert.disabled).toBe(true);
    fireEvent.change(screen.getByTestId("formula-picker-arg-value1"), {
      target: { value: "A1:A2" },
    });
    expect(insert.disabled).toBe(false);
  });

  it("clicking Insert calls onInsert with the assembled formula AND closes the picker", () => {
    const onInsert = vi.fn();
    const onClose = vi.fn();
    render(
      <FormulaPicker
        open={true}
        onClose={onClose}
        context={makeCtx()}
        onInsert={onInsert}
      />,
    );
    fireEvent.click(
      screen.getByTestId("formula-picker-result-SUMIF").querySelector("button")!,
    );
    fireEvent.change(screen.getByTestId("formula-picker-arg-range"), {
      target: { value: "A1:A3" },
    });
    fireEvent.change(screen.getByTestId("formula-picker-arg-predicate"), {
      target: { value: '">15"' },
    });
    fireEvent.click(screen.getByTestId("formula-picker-insert"));
    expect(onInsert).toHaveBeenCalledWith('=SUMIF(A1:A3, ">15")');
    expect(onClose).toHaveBeenCalled();
  });

  it("zero-arg functions like TODAY() enable Insert immediately", () => {
    const onInsert = vi.fn();
    render(
      <FormulaPicker
        open={true}
        onClose={() => {}}
        context={makeCtx()}
        onInsert={onInsert}
      />,
    );
    fireEvent.click(
      screen.getByTestId("formula-picker-result-TODAY").querySelector("button")!,
    );
    const insert = screen.getByTestId("formula-picker-insert") as HTMLButtonElement;
    expect(insert.disabled).toBe(false);
    fireEvent.click(insert);
    expect(onInsert).toHaveBeenCalledWith("=TODAY()");
  });
});

describe("FormulaPicker · keyboard + dismiss", () => {
  it("clicking the backdrop closes", () => {
    const onClose = vi.fn();
    render(<FormulaPicker open={true} onClose={onClose} context={makeCtx()} />);
    fireEvent.click(screen.getByTestId("formula-picker-bg"));
    expect(onClose).toHaveBeenCalled();
  });

  it("clicking the close ✕ closes", () => {
    const onClose = vi.fn();
    render(<FormulaPicker open={true} onClose={onClose} context={makeCtx()} />);
    fireEvent.click(screen.getByTestId("formula-picker-close"));
    expect(onClose).toHaveBeenCalled();
  });

  it("Escape from the list view closes the picker", () => {
    const onClose = vi.fn();
    render(<FormulaPicker open={true} onClose={onClose} context={makeCtx()} />);
    fireEvent.keyDown(document, { key: "Escape" });
    expect(onClose).toHaveBeenCalled();
  });

  it("Escape from the edit view goes back to the list (does NOT close)", () => {
    const onClose = vi.fn();
    render(<FormulaPicker open={true} onClose={onClose} context={makeCtx()} />);
    fireEvent.click(
      screen.getByTestId("formula-picker-result-SUM").querySelector("button")!,
    );
    expect(screen.getByTestId("formula-picker-back")).toBeInTheDocument();
    fireEvent.keyDown(document, { key: "Escape" });
    expect(onClose).not.toHaveBeenCalled();
    // We're back on the list.
    expect(screen.getByTestId("formula-picker-search")).toBeInTheDocument();
  });
});
