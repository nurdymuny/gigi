import { describe, expect, it, vi } from "vitest";
import { fireEvent, render, screen } from "@testing-library/react";
import { MenuBar, type Menu } from "../../src/components/MenuBar";
import { buildMenus } from "../../src/components/menuDefs";

const MENUS: Menu[] = [
  {
    name: "File",
    items: [
      { id: "file:new", label: "New" },
      { id: "file:open", label: "Open" },
      { separator: true, id: "sep1", label: "" },
      {
        id: "file:export",
        label: "Export",
        submenu: [
          { id: "file:export:csv", label: "CSV" },
          { id: "file:export:json", label: "JSON" },
        ],
      },
    ],
  },
  {
    name: "Edit",
    items: [
      { id: "edit:undo", label: "Undo", shortcut: "⌘Z" },
      { id: "edit:redo", label: "Redo", shortcut: "⌘⇧Z" },
    ],
  },
];

describe("MenuBar", () => {
  it("renders each top-level menu as a button", () => {
    render(<MenuBar menus={MENUS} onAction={() => {}} />);
    expect(screen.getByTestId("menu-file")).toBeInTheDocument();
    expect(screen.getByTestId("menu-edit")).toBeInTheDocument();
  });

  it("opens a dropdown when a menu button is clicked", () => {
    render(<MenuBar menus={MENUS} onAction={() => {}} />);
    fireEvent.click(screen.getByTestId("menu-file"));
    expect(screen.getByTestId("menu-item-file:new")).toBeInTheDocument();
    expect(screen.getByTestId("menu-item-file:open")).toBeInTheDocument();
  });

  it("dispatches the item id when an item is clicked, then closes", () => {
    const onAction = vi.fn();
    render(<MenuBar menus={MENUS} onAction={onAction} />);
    fireEvent.click(screen.getByTestId("menu-file"));
    fireEvent.click(screen.getByTestId("menu-item-file:new"));
    expect(onAction).toHaveBeenCalledWith("file:new");
    // Dropdown closed after click.
    expect(screen.queryByTestId("menu-item-file:new")).toBeNull();
  });

  it("renders shortcut hints right-aligned for items that declare them", () => {
    render(<MenuBar menus={MENUS} onAction={() => {}} />);
    fireEvent.click(screen.getByTestId("menu-edit"));
    expect(screen.getByTestId("menu-item-edit:undo")).toHaveTextContent("⌘Z");
  });

  it("clicking a submenu item dispatches the submenu's id", () => {
    const onAction = vi.fn();
    render(<MenuBar menus={MENUS} onAction={onAction} />);
    fireEvent.click(screen.getByTestId("menu-file"));
    fireEvent.click(screen.getByTestId("menu-item-file:export"));
    fireEvent.click(screen.getByTestId("menu-item-file:export:csv"));
    expect(onAction).toHaveBeenCalledWith("file:export:csv");
  });

  it("Escape closes an open dropdown", () => {
    render(<MenuBar menus={MENUS} onAction={() => {}} />);
    fireEvent.click(screen.getByTestId("menu-file"));
    expect(screen.getByTestId("menu-item-file:new")).toBeInTheDocument();
    fireEvent.keyDown(document, { key: "Escape" });
    expect(screen.queryByTestId("menu-item-file:new")).toBeNull();
  });

  it("renders a ✓ for items whose check() returns true", () => {
    const menus: Menu[] = [
      {
        name: "View",
        items: [
          { id: "view:overlay", label: "Overlay", check: () => true },
          { id: "view:grid", label: "Grid", check: () => false },
        ],
      },
    ];
    render(<MenuBar menus={menus} onAction={() => {}} />);
    fireEvent.click(screen.getByTestId("menu-view"));
    expect(screen.getByTestId("menu-item-view:overlay")).toHaveTextContent("✓");
    expect(screen.getByTestId("menu-item-view:grid")).not.toHaveTextContent("✓");
  });
});

describe("buildMenus — full structure", () => {
  it("emits all top-level menus", () => {
    const menus = buildMenus({
      overlayOn: true,
      inspectorOpen: true,
      activeView: "grid",
      multiSelectCount: 0,
      hasFocusedRow: false,
    });
    expect(menus.map((m) => m.name)).toEqual([
      "File",
      "Edit",
      "View",
      "Insert",
      "Data",
      "Geometry",
      "Tools",
      "Help",
    ]);
  });

  it("View › Grid is checked when activeView === 'grid'", () => {
    const menus = buildMenus({
      overlayOn: false,
      inspectorOpen: true,
      activeView: "grid",
      multiSelectCount: 0,
      hasFocusedRow: false,
    });
    const view = menus.find((m) => m.name === "View")!;
    const gridItem = view.items.find((i) => i.id === "view:grid");
    expect(gridItem?.check?.()).toBe(true);
    const geo = view.items.find((i) => i.id === "view:geometry");
    expect(geo?.check?.()).toBe(false);
  });

  it("View › Geometry overlay is checked when overlayOn", () => {
    const menus = buildMenus({
      overlayOn: true,
      inspectorOpen: true,
      activeView: "grid",
      multiSelectCount: 0,
      hasFocusedRow: false,
    });
    const view = menus.find((m) => m.name === "View")!;
    const overlay = view.items.find((i) => i.id === "view:overlay");
    expect(overlay?.check?.()).toBe(true);
  });
});
