import { describe, expect, it, vi } from "vitest";
import { fireEvent, render, screen } from "@testing-library/react";
import { ConditionalFormatModal } from "../../src/components/ConditionalFormatModal";

/**
 * Conditional-format popover — single κ-threshold rule per column.
 */

function anchor(): HTMLElement {
  const el = document.createElement("button");
  document.body.appendChild(el);
  return el;
}

describe("ConditionalFormatModal", () => {
  it("renders threshold input + color swatches + preview", () => {
    render(
      <ConditionalFormatModal
        open={true}
        field="temp"
        rule={null}
        anchorEl={anchor()}
        onChange={() => undefined}
        onClose={() => undefined}
      />,
    );
    expect(screen.getByTestId("cf-threshold")).toBeInTheDocument();
    expect(screen.getByTestId("cf-swatch-red")).toBeInTheDocument();
    expect(screen.getByTestId("cf-pop-preview")).toBeInTheDocument();
  });

  it("Apply emits the rule with current threshold + color", () => {
    const onChange = vi.fn();
    render(
      <ConditionalFormatModal
        open={true}
        field="temp"
        rule={null}
        anchorEl={anchor()}
        onChange={onChange}
        onClose={() => undefined}
      />,
    );
    fireEvent.change(screen.getByTestId("cf-threshold"), {
      target: { value: "1.5" },
    });
    fireEvent.click(screen.getByTestId("cf-swatch-amber"));
    fireEvent.click(screen.getByTestId("cf-apply"));
    expect(onChange).toHaveBeenCalledWith({
      kappaThreshold: 1.5,
      color: "amber",
    });
  });

  it("Clear emits null", () => {
    const onChange = vi.fn();
    render(
      <ConditionalFormatModal
        open={true}
        field="temp"
        rule={{ kappaThreshold: 0.3, color: "red" }}
        anchorEl={anchor()}
        onChange={onChange}
        onClose={() => undefined}
      />,
    );
    fireEvent.click(screen.getByTestId("cf-clear"));
    expect(onChange).toHaveBeenCalledWith(null);
  });

  it("preset buttons jump the threshold to the standard κ bands", () => {
    render(
      <ConditionalFormatModal
        open={true}
        field="temp"
        rule={null}
        anchorEl={anchor()}
        onChange={() => undefined}
        onClose={() => undefined}
      />,
    );
    fireEvent.click(screen.getByTestId("cf-preset-bad"));
    expect((screen.getByTestId("cf-threshold") as HTMLInputElement).value).toBe("0.3");
    fireEvent.click(screen.getByTestId("cf-preset-drift"));
    expect((screen.getByTestId("cf-threshold") as HTMLInputElement).value).toBe("0.1");
  });

  it("seeds the form with the existing rule's threshold + color", () => {
    render(
      <ConditionalFormatModal
        open={true}
        field="temp"
        rule={{ kappaThreshold: 2.0, color: "purple" }}
        anchorEl={anchor()}
        onChange={() => undefined}
        onClose={() => undefined}
      />,
    );
    expect((screen.getByTestId("cf-threshold") as HTMLInputElement).value).toBe("2");
    // Purple swatch is active.
    expect(screen.getByTestId("cf-swatch-purple").className).toMatch(/cf-pop-swatch-active/);
  });
});
