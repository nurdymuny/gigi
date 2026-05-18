import { describe, expect, it, vi } from "vitest";
import { fireEvent, render, screen } from "@testing-library/react";
import {
  ContextMenu,
  type ContextMenuItem,
} from "../../src/components/ContextMenu";

function items(overrides: Partial<ContextMenuItem> = {}): ContextMenuItem[] {
  return [
    {
      id: "copy",
      label: "Copy",
      shortcut: "⌘C",
      onSelect: vi.fn(),
      ...overrides,
    },
    {
      id: "open",
      label: "Open",
      onSelect: vi.fn(),
    },
  ];
}

describe("ContextMenu", () => {
  it("renders nothing when closed", () => {
    render(
      <ContextMenu
        open={false}
        x={0}
        y={0}
        items={items()}
        onClose={() => {}}
      />,
    );
    expect(screen.queryByTestId("context-menu")).toBeNull();
  });

  it("renders the header + each item with shortcut hint when open", () => {
    render(
      <ContextMenu
        open
        x={100}
        y={100}
        header="S-001"
        items={items()}
        onClose={() => {}}
      />,
    );
    expect(screen.getByTestId("context-menu")).toBeInTheDocument();
    expect(screen.getByTestId("context-menu-header")).toHaveTextContent("S-001");
    expect(screen.getByTestId("context-menu-copy")).toHaveTextContent("Copy");
    expect(screen.getByTestId("context-menu-copy")).toHaveTextContent("⌘C");
    expect(screen.getByTestId("context-menu-open")).toHaveTextContent("Open");
  });

  it("positions the menu at (x, y) when there's room", () => {
    render(
      <ContextMenu
        open
        x={100}
        y={200}
        items={items()}
        onClose={() => {}}
      />,
    );
    const menu = screen.getByTestId("context-menu") as HTMLElement;
    expect(menu.style.left).toBe("100px");
    expect(menu.style.top).toBe("200px");
  });

  it("flips horizontally when x is too close to the right edge", () => {
    // jsdom window.innerWidth is 1024 by default in our setup
    render(
      <ContextMenu
        open
        x={1000}
        y={100}
        items={items()}
        onClose={() => {}}
      />,
    );
    const menu = screen.getByTestId("context-menu") as HTMLElement;
    // x=1000 + 240 (MENU_W) = 1240 > 1024 → flip
    expect(parseInt(menu.style.left, 10)).toBeLessThan(1000);
  });

  it("calls onSelect and then onClose when an item is clicked", () => {
    const onSelect = vi.fn();
    const onClose = vi.fn();
    render(
      <ContextMenu
        open
        x={0}
        y={0}
        items={[{ id: "x", label: "X", onSelect }]}
        onClose={onClose}
      />,
    );
    fireEvent.click(screen.getByTestId("context-menu-x"));
    expect(onSelect).toHaveBeenCalledOnce();
    expect(onClose).toHaveBeenCalledOnce();
  });

  it("does NOT call onSelect for disabled items", () => {
    const onSelect = vi.fn();
    const onClose = vi.fn();
    render(
      <ContextMenu
        open
        x={0}
        y={0}
        items={[{ id: "x", label: "X", disabled: true, onSelect }]}
        onClose={onClose}
      />,
    );
    fireEvent.click(screen.getByTestId("context-menu-x"));
    expect(onSelect).not.toHaveBeenCalled();
    expect(onClose).not.toHaveBeenCalled();
  });

  it("closes on Escape", () => {
    const onClose = vi.fn();
    render(
      <ContextMenu
        open
        x={0}
        y={0}
        items={items()}
        onClose={onClose}
      />,
    );
    fireEvent.keyDown(document, { key: "Escape" });
    expect(onClose).toHaveBeenCalledOnce();
  });

  it("renders a separator between items when item.separator is true", () => {
    const { container } = render(
      <ContextMenu
        open
        x={0}
        y={0}
        items={[
          { id: "a", label: "A", onSelect: () => {} },
          { id: "sep", label: "", separator: true, onSelect: () => {} },
          { id: "b", label: "B", onSelect: () => {} },
        ]}
        onClose={() => {}}
      />,
    );
    expect(container.querySelectorAll(".context-menu-sep")).toHaveLength(1);
  });
});
