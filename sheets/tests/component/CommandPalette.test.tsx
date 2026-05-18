import { describe, expect, it, vi } from "vitest";
import { fireEvent, render, screen } from "@testing-library/react";
import { CommandPalette, type Command } from "../../src/components/CommandPalette";

const COMMANDS: Command[] = [
  { id: "view:grid", section: "Views", label: "Switch to Grid", run: vi.fn() },
  { id: "view:gql", section: "Views", label: "Switch to GQL", run: vi.fn() },
  { id: "file:share", section: "File", label: "Share this view", shortcut: "⌘⇧S", run: vi.fn() },
  { id: "bundle:iris", section: "Bundles", label: "Open iris", run: vi.fn() },
  { id: "bundle:nba_2024", section: "Bundles", label: "Open nba_2024", run: vi.fn() },
];

describe("CommandPalette", () => {
  it("renders nothing when closed", () => {
    render(<CommandPalette open={false} commands={COMMANDS} onClose={() => {}} />);
    expect(screen.queryByTestId("command-palette")).toBeNull();
  });

  it("renders an autofocused input and a sectioned list when open", () => {
    render(<CommandPalette open commands={COMMANDS} onClose={() => {}} />);
    const input = screen.getByTestId("command-input") as HTMLInputElement;
    expect(document.activeElement).toBe(input);
    expect(screen.getByTestId("command-section-Views")).toBeInTheDocument();
    expect(screen.getByTestId("command-section-File")).toBeInTheDocument();
    expect(screen.getByTestId("command-section-Bundles")).toBeInTheDocument();
  });

  it("filters commands by fuzzy substring on label", () => {
    render(<CommandPalette open commands={COMMANDS} onClose={() => {}} />);
    fireEvent.change(screen.getByTestId("command-input"), {
      target: { value: "gql" },
    });
    expect(screen.getByTestId("command-item-view:gql")).toBeInTheDocument();
    expect(screen.queryByTestId("command-item-bundle:iris")).toBeNull();
  });

  it("Enter runs the first visible command then closes", () => {
    const onClose = vi.fn();
    const run = vi.fn();
    const cmds: Command[] = [
      { id: "x", section: "X", label: "Run x", run },
    ];
    render(<CommandPalette open commands={cmds} onClose={onClose} />);
    fireEvent.keyDown(screen.getByTestId("command-input"), { key: "Enter" });
    expect(run).toHaveBeenCalledOnce();
    expect(onClose).toHaveBeenCalled();
  });

  it("clicking a result runs that command and closes", () => {
    const onClose = vi.fn();
    const run = vi.fn();
    const cmds: Command[] = [
      { id: "x", section: "X", label: "Run x", run },
    ];
    render(<CommandPalette open commands={cmds} onClose={onClose} />);
    fireEvent.click(screen.getByTestId("command-item-x"));
    expect(run).toHaveBeenCalledOnce();
    expect(onClose).toHaveBeenCalled();
  });

  it("Escape closes without running anything", () => {
    const onClose = vi.fn();
    const run = vi.fn();
    const cmds: Command[] = [
      { id: "x", section: "X", label: "Run x", run },
    ];
    render(<CommandPalette open commands={cmds} onClose={onClose} />);
    fireEvent.keyDown(screen.getByTestId("command-input"), { key: "Escape" });
    expect(onClose).toHaveBeenCalled();
    expect(run).not.toHaveBeenCalled();
  });

  it("shows empty-state when no commands match", () => {
    render(<CommandPalette open commands={COMMANDS} onClose={() => {}} />);
    fireEvent.change(screen.getByTestId("command-input"), {
      target: { value: "zzzzzzz" },
    });
    expect(screen.getByTestId("command-empty")).toBeInTheDocument();
  });

  it("groups results by section name", () => {
    render(<CommandPalette open commands={COMMANDS} onClose={() => {}} />);
    const sections = screen.getAllByTestId(/^command-section-/);
    const names = sections.map((s) => s.getAttribute("data-testid"));
    expect(names).toEqual([
      "command-section-Views",
      "command-section-File",
      "command-section-Bundles",
    ]);
  });
});
