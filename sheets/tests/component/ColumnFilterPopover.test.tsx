import { describe, expect, it, vi } from "vitest";
import { fireEvent, render, screen } from "@testing-library/react";
import { ColumnFilterPopover } from "../../src/components/ColumnFilterPopover";
import type { FieldDescriptor } from "../../src/lib/gigi-client";

/**
 * Per-column filter popover — Airtable-style.
 *
 * Type-aware UI:
 *   text / categorical / timestamp   → Contains substring
 *   numeric                          → Min / Max range
 *   boolean                          → Both / true / false
 */

function anchor(): HTMLElement {
  const el = document.createElement("button");
  document.body.appendChild(el);
  return el;
}

describe("ColumnFilterPopover · text column", () => {
  it("renders a Contains input", () => {
    render(
      <ColumnFilterPopover
        field={{ name: "site", type: "categorical" } as FieldDescriptor}
        filter={null}
        anchorEl={anchor()}
        onChange={() => undefined}
        onClose={() => undefined}
      />,
    );
    expect(screen.getByTestId("col-filter-contains")).toBeInTheDocument();
  });

  it("Apply with a non-empty value emits a `contains` filter", () => {
    const onChange = vi.fn();
    const onClose = vi.fn();
    render(
      <ColumnFilterPopover
        field={{ name: "site", type: "categorical" } as FieldDescriptor}
        filter={null}
        anchorEl={anchor()}
        onChange={onChange}
        onClose={onClose}
      />,
    );
    fireEvent.change(screen.getByTestId("col-filter-contains"), {
      target: { value: "north" },
    });
    fireEvent.click(screen.getByTestId("col-filter-apply"));
    expect(onChange).toHaveBeenCalledWith({
      kind: "text",
      column: "site",
      op: "contains",
      value: "north",
    });
    expect(onClose).toHaveBeenCalled();
  });

  it("Apply with an empty value emits null (clears the filter)", () => {
    const onChange = vi.fn();
    render(
      <ColumnFilterPopover
        field={{ name: "site", type: "categorical" } as FieldDescriptor}
        filter={null}
        anchorEl={anchor()}
        onChange={onChange}
        onClose={() => undefined}
      />,
    );
    fireEvent.click(screen.getByTestId("col-filter-apply"));
    expect(onChange).toHaveBeenCalledWith(null);
  });

  it("Clear emits null and closes", () => {
    const onChange = vi.fn();
    const onClose = vi.fn();
    render(
      <ColumnFilterPopover
        field={{ name: "site", type: "categorical" } as FieldDescriptor}
        filter={{
          kind: "text",
          column: "site",
          op: "contains",
          value: "north",
        }}
        anchorEl={anchor()}
        onChange={onChange}
        onClose={onClose}
      />,
    );
    fireEvent.click(screen.getByTestId("col-filter-clear"));
    expect(onChange).toHaveBeenCalledWith(null);
    expect(onClose).toHaveBeenCalled();
  });
});

describe("ColumnFilterPopover · numeric column", () => {
  it("renders Min and Max inputs", () => {
    render(
      <ColumnFilterPopover
        field={{ name: "temp", type: "numeric" } as FieldDescriptor}
        filter={null}
        anchorEl={anchor()}
        onChange={() => undefined}
        onClose={() => undefined}
      />,
    );
    expect(screen.getByTestId("col-filter-min")).toBeInTheDocument();
    expect(screen.getByTestId("col-filter-max")).toBeInTheDocument();
  });

  it("Apply with only Min emits a half-open range", () => {
    const onChange = vi.fn();
    render(
      <ColumnFilterPopover
        field={{ name: "temp", type: "numeric" } as FieldDescriptor}
        filter={null}
        anchorEl={anchor()}
        onChange={onChange}
        onClose={() => undefined}
      />,
    );
    fireEvent.change(screen.getByTestId("col-filter-min"), {
      target: { value: "10" },
    });
    fireEvent.click(screen.getByTestId("col-filter-apply"));
    expect(onChange).toHaveBeenCalledWith({
      kind: "range",
      column: "temp",
      min: 10,
      max: undefined,
    });
  });

  it("Apply with both Min and Max emits a closed range", () => {
    const onChange = vi.fn();
    render(
      <ColumnFilterPopover
        field={{ name: "temp", type: "numeric" } as FieldDescriptor}
        filter={null}
        anchorEl={anchor()}
        onChange={onChange}
        onClose={() => undefined}
      />,
    );
    fireEvent.change(screen.getByTestId("col-filter-min"), {
      target: { value: "10" },
    });
    fireEvent.change(screen.getByTestId("col-filter-max"), {
      target: { value: "20" },
    });
    fireEvent.click(screen.getByTestId("col-filter-apply"));
    expect(onChange).toHaveBeenCalledWith({
      kind: "range",
      column: "temp",
      min: 10,
      max: 20,
    });
  });
});

describe("ColumnFilterPopover · boolean column", () => {
  it("'true' option emits an equals='true' filter", () => {
    const onChange = vi.fn();
    render(
      <ColumnFilterPopover
        field={{ name: "active", type: "boolean" } as FieldDescriptor}
        filter={null}
        anchorEl={anchor()}
        onChange={onChange}
        onClose={() => undefined}
      />,
    );
    fireEvent.change(screen.getByTestId("col-filter-bool"), {
      target: { value: "true" },
    });
    fireEvent.click(screen.getByTestId("col-filter-apply"));
    expect(onChange).toHaveBeenCalledWith({
      kind: "text",
      column: "active",
      op: "equals",
      value: "true",
    });
  });

  it("'Both' option clears the filter", () => {
    const onChange = vi.fn();
    render(
      <ColumnFilterPopover
        field={{ name: "active", type: "boolean" } as FieldDescriptor}
        filter={{
          kind: "text",
          column: "active",
          op: "equals",
          value: "true",
        }}
        anchorEl={anchor()}
        onChange={onChange}
        onClose={() => undefined}
      />,
    );
    fireEvent.change(screen.getByTestId("col-filter-bool"), {
      target: { value: "both" },
    });
    fireEvent.click(screen.getByTestId("col-filter-apply"));
    expect(onChange).toHaveBeenCalledWith(null);
  });
});

describe("ColumnFilterPopover · dismiss", () => {
  it("Escape fires onClose", () => {
    const onClose = vi.fn();
    render(
      <ColumnFilterPopover
        field={{ name: "site", type: "categorical" } as FieldDescriptor}
        filter={null}
        anchorEl={anchor()}
        onChange={() => undefined}
        onClose={onClose}
      />,
    );
    fireEvent.keyDown(document, { key: "Escape" });
    expect(onClose).toHaveBeenCalled();
  });
});
