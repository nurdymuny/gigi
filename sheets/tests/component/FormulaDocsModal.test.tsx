import { describe, expect, it, vi } from "vitest";
import { fireEvent, render, screen } from "@testing-library/react";
import { FormulaDocsModal } from "../../src/components/FormulaDocsModal";
import { FORMULA_DOCS } from "../../src/lib/formula-docs";

/**
 * Phase 6 · auto-generated formula reference modal. Asserts the registry
 * → docs round-trip stays honest: every doc renders, search and category
 * filter work, dismissal paths fire.
 */

describe("FormulaDocsModal", () => {
  it("renders nothing when closed", () => {
    render(<FormulaDocsModal open={false} onClose={() => {}} />);
    expect(screen.queryByTestId("formula-docs")).toBeNull();
  });

  it("renders every doc from the registry when open + filter=all", () => {
    render(<FormulaDocsModal open={true} onClose={() => {}} />);
    for (const d of FORMULA_DOCS) {
      expect(
        screen.getByTestId(`formula-docs-entry-${d.name}`),
        `entry for ${d.name}`,
      ).toBeInTheDocument();
    }
  });

  it("searching narrows the list", () => {
    render(<FormulaDocsModal open={true} onClose={() => {}} />);
    fireEvent.change(screen.getByTestId("formula-docs-search"), {
      target: { value: "sumif" },
    });
    expect(screen.getByTestId("formula-docs-entry-SUMIF")).toBeInTheDocument();
    expect(screen.queryByTestId("formula-docs-entry-AVERAGE")).toBeNull();
  });

  it("clicking a category filters to that group", () => {
    render(<FormulaDocsModal open={true} onClose={() => {}} />);
    const geomBtn = screen
      .getAllByRole("button")
      .find((b) => b.textContent?.startsWith("Geometry"))!;
    fireEvent.click(geomBtn);
    expect(screen.getByTestId("formula-docs-entry-SAME")).toBeInTheDocument();
    expect(screen.queryByTestId("formula-docs-entry-SUM")).toBeNull();
  });

  it("close ✕ + backdrop + Escape all fire onClose", () => {
    const onClose1 = vi.fn();
    const { unmount } = render(<FormulaDocsModal open={true} onClose={onClose1} />);
    fireEvent.click(screen.getByTestId("formula-docs-close"));
    expect(onClose1).toHaveBeenCalledTimes(1);
    unmount();

    const onClose2 = vi.fn();
    const { unmount: u2 } = render(<FormulaDocsModal open={true} onClose={onClose2} />);
    fireEvent.click(screen.getByTestId("formula-docs-bg"));
    expect(onClose2).toHaveBeenCalledTimes(1);
    u2();

    const onClose3 = vi.fn();
    render(<FormulaDocsModal open={true} onClose={onClose3} />);
    fireEvent.keyDown(document, { key: "Escape" });
    expect(onClose3).toHaveBeenCalledTimes(1);
  });
});
