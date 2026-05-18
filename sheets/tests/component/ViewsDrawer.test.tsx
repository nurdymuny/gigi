import { beforeEach, describe, expect, it, vi } from "vitest";
import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { ViewsDrawer } from "../../src/components/ViewsDrawer";
import { saveView, type ViewSpec } from "../../src/lib/view";

const SPEC: ViewSpec = {
  v: 1,
  coverField: "site_id",
  overlayOn: true,
  activeView: "grid",
  inspectorOpen: true,
  gqlQuery: "SECTION sensors;",
};

describe("ViewsDrawer", () => {
  beforeEach(() => {
    localStorage.clear();
  });

  it("renders nothing when closed", () => {
    render(
      <ViewsDrawer
        open={false}
        bundle="sensors"
        currentSpec={SPEC}
        onClose={() => {}}
        onApply={() => {}}
      />,
    );
    expect(screen.queryByTestId("views-drawer")).toBeNull();
  });

  it("shows the empty state when no views are saved for this bundle", () => {
    render(
      <ViewsDrawer
        open
        bundle="sensors"
        currentSpec={SPEC}
        onClose={() => {}}
        onApply={() => {}}
      />,
    );
    expect(screen.getByTestId("views-drawer-empty")).toBeInTheDocument();
  });

  it("Save button is disabled when no name is given", () => {
    render(
      <ViewsDrawer
        open
        bundle="sensors"
        currentSpec={SPEC}
        onClose={() => {}}
        onApply={() => {}}
      />,
    );
    expect(screen.getByTestId("views-drawer-save")).toBeDisabled();
  });

  it("saving a name adds it to the list and renders it", () => {
    render(
      <ViewsDrawer
        open
        bundle="sensors"
        currentSpec={SPEC}
        onClose={() => {}}
        onApply={() => {}}
      />,
    );
    fireEvent.change(screen.getByTestId("views-drawer-name"), {
      target: { value: "North-3 anomalies" },
    });
    fireEvent.click(screen.getByTestId("views-drawer-save"));
    expect(screen.getByTestId("views-drawer-list")).toBeInTheDocument();
    expect(screen.getByText("North-3 anomalies")).toBeInTheDocument();
  });

  it("clicking a saved view calls onApply with its spec", () => {
    saveView({ name: "saved", bundle: "sensors", spec: SPEC });
    const onApply = vi.fn();
    render(
      <ViewsDrawer
        open
        bundle="sensors"
        currentSpec={SPEC}
        onClose={() => {}}
        onApply={onApply}
      />,
    );
    fireEvent.click(screen.getByText("saved"));
    expect(onApply).toHaveBeenCalledWith(SPEC);
  });

  it("Delete removes the view from the list", () => {
    const v = saveView({ name: "saved", bundle: "sensors", spec: SPEC });
    render(
      <ViewsDrawer
        open
        bundle="sensors"
        currentSpec={SPEC}
        onClose={() => {}}
        onApply={() => {}}
      />,
    );
    fireEvent.click(screen.getByTestId(`views-drawer-delete-${v.id}`));
    expect(screen.queryByText("saved")).toBeNull();
  });

  it("only shows views saved against the current bundle", () => {
    saveView({ name: "for sensors", bundle: "sensors", spec: SPEC });
    saveView({ name: "for events", bundle: "events", spec: SPEC });
    render(
      <ViewsDrawer
        open
        bundle="sensors"
        currentSpec={SPEC}
        onClose={() => {}}
        onApply={() => {}}
      />,
    );
    expect(screen.getByText("for sensors")).toBeInTheDocument();
    expect(screen.queryByText("for events")).toBeNull();
  });

  it("Copy share link writes a URL with ?view= to the clipboard", async () => {
    const writeText = vi.fn().mockResolvedValue(undefined);
    Object.defineProperty(navigator, "clipboard", {
      configurable: true,
      value: { writeText },
    });
    const onShare = vi.fn();
    render(
      <ViewsDrawer
        open
        bundle="sensors"
        currentSpec={SPEC}
        onClose={() => {}}
        onApply={() => {}}
        onShare={onShare}
      />,
    );
    fireEvent.click(screen.getByTestId("views-drawer-copy-link"));
    await waitFor(() => expect(writeText).toHaveBeenCalled());
    expect(writeText.mock.calls[0][0]).toContain("view=");
    await waitFor(() => expect(onShare).toHaveBeenCalled());
  });
});
