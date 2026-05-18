import { describe, expect, it, vi } from "vitest";
import { fireEvent, render, screen } from "@testing-library/react";
import { FindModal } from "../../src/components/FindModal";
import type { BundleSchema } from "../../src/lib/gigi-client";

/**
 * Find + Replace.
 *
 * The Replace UI is hidden by default (Find-only). Passing `onReplace`
 * surfaces a Replace toggle in the mode row; toggling it reveals a
 * "Replace with" input + Replace / Replace all buttons. Each
 * replacement fires `onReplace(rowKey, field, newValue)` so the host
 * routes the write through its normal edit-history path.
 */

const SCHEMA: BundleSchema = {
  name: "sensors",
  base_fields: [{ name: "sensor_id", type: "text" }],
  fiber_fields: [
    { name: "operator", type: "text" },
    { name: "site", type: "categorical" },
  ],
  indexed_fields: ["sensor_id"],
  records: 4,
  storage_mode: "mmap",
};

const ROWS = [
  { sensor_id: "S-1", operator: "alice", site: "North" },
  { sensor_id: "S-2", operator: "Alice", site: "South" },
  { sensor_id: "S-3", operator: "bob", site: "North" },
  { sensor_id: "S-4", operator: "alice", site: "East" },
];

describe("FindModal · replace UI", () => {
  it("Replace toggle is hidden when onReplace is omitted", () => {
    render(
      <FindModal
        open={true}
        schema={SCHEMA}
        rows={ROWS}
        onClose={() => {}}
        onSelectRow={() => {}}
      />,
    );
    expect(screen.queryByTestId("find-replace-toggle")).toBeNull();
  });

  it("Replace toggle reveals the input + action buttons", () => {
    render(
      <FindModal
        open={true}
        schema={SCHEMA}
        rows={ROWS}
        onClose={() => {}}
        onSelectRow={() => {}}
        onReplace={() => {}}
      />,
    );
    expect(screen.getByTestId("find-replace-toggle")).toBeInTheDocument();
    expect(screen.queryByTestId("find-replace-input")).toBeNull();
    fireEvent.click(screen.getByTestId("find-replace-toggle"));
    expect(screen.getByTestId("find-replace-input")).toBeInTheDocument();
    expect(screen.getByTestId("find-replace-one")).toBeInTheDocument();
    expect(screen.getByTestId("find-replace-all")).toBeInTheDocument();
  });
});

describe("FindModal · replace behavior", () => {
  it("Replace All fires onReplace once per matching row (case-insensitive)", () => {
    const onReplace = vi.fn();
    render(
      <FindModal
        open={true}
        schema={SCHEMA}
        rows={ROWS}
        onClose={() => {}}
        onSelectRow={() => {}}
        onReplace={onReplace}
      />,
    );
    fireEvent.change(screen.getByTestId("find-input"), {
      target: { value: "alice" },
    });
    fireEvent.click(screen.getByTestId("find-replace-toggle"));
    fireEvent.change(screen.getByTestId("find-replace-input"), {
      target: { value: "ALICE" },
    });
    fireEvent.click(screen.getByTestId("find-replace-all"));
    // Three rows contain "alice"/"Alice" (S-1, S-2, S-4); S-3 doesn't.
    expect(onReplace).toHaveBeenCalledTimes(3);
    const calls = onReplace.mock.calls.map((c) => c.slice(0, 3));
    expect(calls).toContainEqual(["S-1", "operator", "ALICE"]);
    expect(calls).toContainEqual(["S-2", "operator", "ALICE"]);
    expect(calls).toContainEqual(["S-4", "operator", "ALICE"]);
  });

  it("Replace (single) only fires for the first match", () => {
    const onReplace = vi.fn();
    render(
      <FindModal
        open={true}
        schema={SCHEMA}
        rows={ROWS}
        onClose={() => {}}
        onSelectRow={() => {}}
        onReplace={onReplace}
      />,
    );
    fireEvent.change(screen.getByTestId("find-input"), {
      target: { value: "alice" },
    });
    fireEvent.click(screen.getByTestId("find-replace-toggle"));
    fireEvent.change(screen.getByTestId("find-replace-input"), {
      target: { value: "X" },
    });
    fireEvent.click(screen.getByTestId("find-replace-one"));
    expect(onReplace).toHaveBeenCalledTimes(1);
  });

  it("Replace All buttons are disabled when there are no matches", () => {
    const onReplace = vi.fn();
    render(
      <FindModal
        open={true}
        schema={SCHEMA}
        rows={ROWS}
        onClose={() => {}}
        onSelectRow={() => {}}
        onReplace={onReplace}
      />,
    );
    fireEvent.change(screen.getByTestId("find-input"), {
      target: { value: "zzzzzz_no_match" },
    });
    fireEvent.click(screen.getByTestId("find-replace-toggle"));
    const all = screen.getByTestId("find-replace-all") as HTMLButtonElement;
    expect(all.disabled).toBe(true);
    fireEvent.click(all);
    expect(onReplace).not.toHaveBeenCalled();
  });

  it("substring replacement preserves the rest of the field", () => {
    const onReplace = vi.fn();
    const rows = [{ sensor_id: "S-1", operator: "hello world", site: "x" }];
    render(
      <FindModal
        open={true}
        schema={SCHEMA}
        rows={rows}
        onClose={() => {}}
        onSelectRow={() => {}}
        onReplace={onReplace}
      />,
    );
    fireEvent.change(screen.getByTestId("find-input"), {
      target: { value: "world" },
    });
    fireEvent.click(screen.getByTestId("find-replace-toggle"));
    fireEvent.change(screen.getByTestId("find-replace-input"), {
      target: { value: "GIGI" },
    });
    fireEvent.click(screen.getByTestId("find-replace-all"));
    expect(onReplace).toHaveBeenCalledWith("S-1", "operator", "hello GIGI");
  });
});
