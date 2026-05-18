import type { Menu } from "./MenuBar";

/**
 * Build the top-level menu structure. The `state` argument supplies the
 * live checkmarks (so View › Geometry overlay shows ✓ when on, etc.).
 *
 * Every id in here must have a real branch in App's `dispatchMenu`. If
 * the action is genuinely not yet implemented, do not add it to the
 * menu — a half-wired menu item is worse than a missing one.
 */
export interface MenuState {
  overlayOn: boolean;
  inspectorOpen: boolean;
  activeView: "grid" | "geometry" | "charts" | "kanban" | "gallery" | "form" | "gql";
  multiSelectCount: number;
  hasFocusedRow: boolean;
}

export function buildMenus(state: MenuState): Menu[] {
  return [
    {
      name: "File",
      items: [
        { id: "file:new", label: "New bundle…", shortcut: "⌘N" },
        { id: "file:open", label: "Open bundle…", shortcut: "⌘O" },
        { separator: true, id: "sep1", label: "" },
        {
          id: "file:import",
          label: "Import",
          submenu: [
            { id: "file:import:csv", label: "From CSV…" },
            { id: "file:import:json", label: "From JSON…" },
          ],
        },
        {
          id: "file:export",
          label: "Export",
          submenu: [
            { id: "file:export:csv", label: "CSV (visible rows)" },
            { id: "file:export:json", label: "JSON" },
            { id: "file:export:gql", label: "GQL script" },
          ],
        },
        { separator: true, id: "sep2", label: "" },
        { id: "file:share", label: "Share link", shortcut: "⌘⇧S" },
        { id: "file:print", label: "Print / PDF…", shortcut: "⌘P" },
      ],
    },
    {
      name: "Edit",
      items: [
        { id: "edit:undo", label: "Undo", shortcut: "⌘Z" },
        { id: "edit:redo", label: "Redo", shortcut: "⌘⇧Z" },
        { separator: true, id: "sep0", label: "" },
        { id: "edit:cut", label: "Cut", shortcut: "⌘X" },
        { id: "edit:copy", label: "Copy", shortcut: "⌘C" },
        { id: "edit:paste", label: "Paste", shortcut: "⌘V" },
        { separator: true, id: "sep1", label: "" },
        { id: "edit:find", label: "Find…", shortcut: "⌘F" },
        { id: "edit:select-all", label: "Select all rows", shortcut: "⌘A" },
        { separator: true, id: "sep2", label: "" },
        { id: "edit:insert-row-above", label: "Insert row above" },
        { id: "edit:insert-row-below", label: "Insert row below" },
        { id: "edit:delete-row", label: "Delete selected row", shortcut: "⌫" },
      ],
    },
    {
      name: "View",
      items: [
        { section: "Switch view", id: "section1", label: "" },
        {
          id: "view:grid",
          label: "Grid",
          shortcut: "⌘1",
          check: () => state.activeView === "grid",
        },
        {
          id: "view:geometry",
          label: "Geometry",
          shortcut: "⌘2",
          check: () => state.activeView === "geometry",
        },
        {
          id: "view:charts",
          label: "Charts",
          shortcut: "⌘3",
          check: () => state.activeView === "charts",
        },
        {
          id: "view:kanban",
          label: "Kanban",
          shortcut: "⌘4",
          check: () => state.activeView === "kanban",
        },
        {
          id: "view:gql",
          label: "GQL",
          shortcut: "⌘5",
          check: () => state.activeView === "gql",
        },
        { separator: true, id: "sep1", label: "" },
        { section: "Show / hide", id: "section2", label: "" },
        {
          id: "view:overlay",
          label: "Geometry overlay",
          check: () => state.overlayOn,
        },
        {
          id: "view:inspector",
          label: "Inspector panel",
          check: () => state.inspectorOpen,
        },
        { id: "view:hide-fields", label: "Hide fields…" },
        { separator: true, id: "sep2", label: "" },
        { id: "view:zoom-in", label: "Zoom in", shortcut: "⌘=" },
        { id: "view:zoom-out", label: "Zoom out", shortcut: "⌘-" },
        { id: "view:zoom-reset", label: "Reset zoom", shortcut: "⌘0" },
        { id: "view:fullscreen", label: "Full screen", shortcut: "F11" },
      ],
    },
    {
      name: "Insert",
      items: [
        { id: "insert:row", label: "Row…", shortcut: "⌘⇧+" },
        {
          id: "insert:field",
          label: "Field…",
          submenu: [
            { id: "insert:field:text", label: "Text" },
            { id: "insert:field:numeric", label: "Numeric" },
            { id: "insert:field:categorical", label: "Categorical" },
            { id: "insert:field:timestamp", label: "Timestamp" },
            { id: "insert:field:enc-opaque", label: "Encrypted · OPAQUE" },
            { id: "insert:field:enc-indexed", label: "Encrypted · INDEXED" },
            { id: "insert:field:enc-affine", label: "Encrypted · AFFINE" },
          ],
        },
        { separator: true, id: "sep1", label: "" },
        { id: "insert:formula", label: "Formula…", shortcut: "⌘=" },
        { separator: true, id: "sep2", label: "" },
        { id: "insert:saved-view", label: "Saved view…" },
      ],
    },
    {
      name: "Data",
      items: [
        { id: "data:sort", label: "Sort…" },
        { id: "data:filter", label: "Filter…", shortcut: "⌘⇧F" },
        { separator: true, id: "sep1", label: "" },
        { id: "data:validate", label: "Validate schema" },
        { id: "data:refresh", label: "Refresh from server", shortcut: "⌘R" },
      ],
    },
    {
      name: "Geometry",
      items: [
        { section: "Bundle-wide", id: "section1", label: "" },
        { id: "geo:kappa", label: "Curvature κ (this view)" },
        { id: "geo:spectral", label: "Spectral λ₁" },
        { id: "geo:betti", label: "Betti numbers b₀ b₁ b₂" },
        { separator: true, id: "sep1", label: "" },
        { section: "Selected row", id: "section2", label: "" },
        { id: "geo:kappa-row", label: "Curvature here" },
        { id: "geo:transport", label: "Transport to nearest peer" },
        { id: "geo:holonomy", label: "Holonomy around cover" },
        { separator: true, id: "sep2", label: "" },
        { id: "geo:recompute", label: "Recompute geometry now" },
        {
          id: "view:overlay",
          label: "Toggle κ overlay",
          check: () => state.overlayOn,
        },
      ],
    },
    {
      name: "Tools",
      items: [
        { id: "tools:gql", label: "GQL console", shortcut: "⌘⇧G" },
        { id: "tools:schema", label: "Schema editor" },
        { id: "tools:views", label: "Saved views" },
        { id: "tools:insights", label: "Insights" },
      ],
    },
    {
      name: "Help",
      items: [
        { id: "help:shortcuts", label: "Keyboard shortcuts", shortcut: "?" },
        { id: "help:formulas", label: "Formula reference…" },
        { separator: true, id: "sep1", label: "" },
        { id: "help:about", label: "About GIGI Sheets" },
      ],
    },
  ];
}
