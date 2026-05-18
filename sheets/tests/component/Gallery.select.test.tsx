import { describe, expect, it, vi } from "vitest";
import { fireEvent, render, screen } from "@testing-library/react";
import { Gallery } from "../../src/components/Gallery";
import type { BundleSchema } from "../../src/lib/gigi-client";

/**
 * Phase 7.A · Gallery multi-select + right-click.
 *
 * Mirrors the Grid's selection model so the two views stay
 * interchangeable — `selectedKeys` highlights N cards, `onRowClick`
 * carries cmd/shift modifiers, `onRowContextMenu` opens the per-card
 * action menu (Open in Inspector · Copy as JSON / SECTION GQL · Find
 * similar · Delete).
 */

const schema: BundleSchema = {
  name: "demo",
  base_fields: [{ name: "id", type: "text" }],
  fiber_fields: [
    { name: "category", type: "categorical" },
    { name: "score", type: "numeric" },
  ],
  indexed_fields: ["id"],
  records: 3,
  storage_mode: "mmap",
} as unknown as BundleSchema;

const rows = [
  { id: "R1", category: "alpha", score: 8.5 },
  { id: "R2", category: "beta", score: 4.2 },
  { id: "R3", category: "alpha", score: 9.1 },
];

const kappaMap = new Map<string, number>([
  ["R1", 0.05],
  ["R2", 3.0],
  ["R3", 1.2],
]);

function cardByKey(k: string): HTMLElement {
  const card = screen
    .getAllByTestId("gallery-card")
    .find((c) => c.getAttribute("data-row-key") === k);
  if (!card) throw new Error(`no card for ${k}`);
  return card;
}

describe("Gallery · multi-select", () => {
  it("highlights every card in selectedKeys", () => {
    render(
      <Gallery
        schema={schema}
        rows={rows}
        kappaMap={kappaMap}
        coverField="category"
        selectedKeys={new Set(["R1", "R3"])}
        onRowSelect={() => undefined}
      />,
    );
    expect(cardByKey("R1").className).toMatch(/gallery-card-selected/);
    expect(cardByKey("R2").className).not.toMatch(/gallery-card-selected/);
    expect(cardByKey("R3").className).toMatch(/gallery-card-selected/);
  });

  it("falls back to selectedRowKey when selectedKeys is omitted", () => {
    render(
      <Gallery
        schema={schema}
        rows={rows}
        kappaMap={kappaMap}
        coverField="category"
        selectedRowKey="R2"
        onRowSelect={() => undefined}
      />,
    );
    expect(cardByKey("R2").className).toMatch(/gallery-card-selected/);
  });

  it("onRowClick fires with meta=true on Cmd-click", () => {
    const onRowClick = vi.fn();
    render(
      <Gallery
        schema={schema}
        rows={rows}
        kappaMap={kappaMap}
        coverField="category"
        onRowClick={onRowClick}
        onRowSelect={() => undefined}
      />,
    );
    fireEvent.click(cardByKey("R2"), { metaKey: true });
    expect(onRowClick).toHaveBeenCalledWith(
      "R2",
      expect.objectContaining({ meta: true, shift: false }),
    );
  });

  it("onRowClick fires with shift=true on Shift-click", () => {
    const onRowClick = vi.fn();
    render(
      <Gallery
        schema={schema}
        rows={rows}
        kappaMap={kappaMap}
        coverField="category"
        onRowClick={onRowClick}
        onRowSelect={() => undefined}
      />,
    );
    fireEvent.click(cardByKey("R3"), { shiftKey: true });
    expect(onRowClick).toHaveBeenCalledWith(
      "R3",
      expect.objectContaining({ shift: true, meta: false }),
    );
  });

  it("falls back to onRowSelect for plain click when onRowClick is omitted", () => {
    const onRowSelect = vi.fn();
    render(
      <Gallery
        schema={schema}
        rows={rows}
        kappaMap={kappaMap}
        coverField="category"
        onRowSelect={onRowSelect}
      />,
    );
    fireEvent.click(cardByKey("R1"));
    expect(onRowSelect).toHaveBeenCalledWith("R1");
  });
});

describe("Gallery · right-click", () => {
  it("fires onRowContextMenu with viewport coords", () => {
    const onRowContextMenu = vi.fn();
    render(
      <Gallery
        schema={schema}
        rows={rows}
        kappaMap={kappaMap}
        coverField="category"
        onRowSelect={() => undefined}
        onRowContextMenu={onRowContextMenu}
      />,
    );
    fireEvent.contextMenu(cardByKey("R2"), {
      clientX: 150,
      clientY: 200,
    });
    expect(onRowContextMenu).toHaveBeenCalledWith("R2", 150, 200);
  });

  it("does NOT also fire onRowClick / onRowSelect when the right-click handler is present", () => {
    const onRowClick = vi.fn();
    const onRowContextMenu = vi.fn();
    render(
      <Gallery
        schema={schema}
        rows={rows}
        kappaMap={kappaMap}
        coverField="category"
        onRowClick={onRowClick}
        onRowContextMenu={onRowContextMenu}
        onRowSelect={() => undefined}
      />,
    );
    fireEvent.contextMenu(cardByKey("R2"));
    expect(onRowContextMenu).toHaveBeenCalled();
    expect(onRowClick).not.toHaveBeenCalled();
  });
});
