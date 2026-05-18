import { describe, expect, it, vi, beforeEach } from "vitest";
import { fireEvent, render, screen } from "@testing-library/react";
import { ShareModal } from "../../src/components/ShareModal";
import type { ViewSpec } from "../../src/lib/view";

const SPEC: ViewSpec = {
  v: 1,
  coverField: "species",
  overlayOn: true,
  activeView: "geometry",
  inspectorOpen: true,
  gqlQuery: "CURVATURE iris;",
};

beforeEach(() => {
  // jsdom doesn't define clipboard by default; install a stub.
  Object.defineProperty(navigator, "clipboard", {
    value: { writeText: vi.fn().mockResolvedValue(undefined) },
    configurable: true,
  });
});

describe("ShareModal", () => {
  it("renders nothing when closed", () => {
    render(
      <ShareModal
        open={false}
        bundle="iris"
        currentSpec={SPEC}
        onClose={() => {}}
        onExportCsv={() => {}}
        onExportJson={() => {}}
        onExportGql={() => {}}
      />,
    );
    expect(screen.queryByTestId("share-modal")).toBeNull();
  });

  it("renders the modal with the bundle name when open", () => {
    render(
      <ShareModal
        open
        bundle="iris"
        currentSpec={SPEC}
        onClose={() => {}}
        onExportCsv={() => {}}
        onExportJson={() => {}}
        onExportGql={() => {}}
      />,
    );
    const modal = screen.getByTestId("share-modal");
    expect(modal).toBeInTheDocument();
    expect(modal).toHaveTextContent("iris");
    expect(screen.getByTestId("share-url")).toBeInTheDocument();
  });

  it("renders a URL containing a ?view= encoded view state", () => {
    render(
      <ShareModal
        open
        bundle="iris"
        currentSpec={SPEC}
        onClose={() => {}}
        onExportCsv={() => {}}
        onExportJson={() => {}}
        onExportGql={() => {}}
      />,
    );
    const url = (screen.getByTestId("share-url") as HTMLInputElement).value;
    expect(url).toMatch(/\?view=[A-Za-z0-9_-]+/);
  });

  it("toggling an include flag changes the URL", () => {
    render(
      <ShareModal
        open
        bundle="iris"
        currentSpec={SPEC}
        onClose={() => {}}
        onExportCsv={() => {}}
        onExportJson={() => {}}
        onExportGql={() => {}}
      />,
    );
    const before = (screen.getByTestId("share-url") as HTMLInputElement).value;
    fireEvent.click(screen.getByTestId("share-include-cover"));
    const after = (screen.getByTestId("share-url") as HTMLInputElement).value;
    expect(after).not.toBe(before);
  });

  it("Copy link calls navigator.clipboard.writeText with the URL", () => {
    render(
      <ShareModal
        open
        bundle="iris"
        currentSpec={SPEC}
        onClose={() => {}}
        onExportCsv={() => {}}
        onExportJson={() => {}}
        onExportGql={() => {}}
      />,
    );
    const url = (screen.getByTestId("share-url") as HTMLInputElement).value;
    fireEvent.click(screen.getByTestId("share-copy-url"));
    expect(navigator.clipboard.writeText).toHaveBeenCalledWith(url);
  });

  it("switching to the Download tab shows three export cards", () => {
    render(
      <ShareModal
        open
        bundle="iris"
        currentSpec={SPEC}
        onClose={() => {}}
        onExportCsv={() => {}}
        onExportJson={() => {}}
        onExportGql={() => {}}
      />,
    );
    fireEvent.click(screen.getByTestId("share-tab-export"));
    expect(screen.getByTestId("share-export-csv")).toBeInTheDocument();
    expect(screen.getByTestId("share-export-json")).toBeInTheDocument();
    expect(screen.getByTestId("share-export-gql")).toBeInTheDocument();
  });

  it("clicking an export card calls the matching handler and closes the modal", () => {
    const onExportCsv = vi.fn();
    const onClose = vi.fn();
    render(
      <ShareModal
        open
        bundle="iris"
        currentSpec={SPEC}
        onClose={onClose}
        onExportCsv={onExportCsv}
        onExportJson={() => {}}
        onExportGql={() => {}}
      />,
    );
    fireEvent.click(screen.getByTestId("share-tab-export"));
    fireEvent.click(screen.getByTestId("share-export-csv"));
    expect(onExportCsv).toHaveBeenCalledOnce();
    expect(onClose).toHaveBeenCalled();
  });

  it("Email link uses a mailto: URL with subject + shareable URL in body", () => {
    render(
      <ShareModal
        open
        bundle="iris"
        currentSpec={SPEC}
        onClose={() => {}}
        onExportCsv={() => {}}
        onExportJson={() => {}}
        onExportGql={() => {}}
      />,
    );
    const a = screen.getByTestId("share-email") as HTMLAnchorElement;
    expect(a.href).toMatch(/^mailto:/);
    expect(decodeURIComponent(a.href)).toContain("iris");
    expect(decodeURIComponent(a.href)).toContain("?view=");
  });

  it("Embed snippet is a complete <iframe> tag with the share URL", () => {
    render(
      <ShareModal
        open
        bundle="iris"
        currentSpec={SPEC}
        onClose={() => {}}
        onExportCsv={() => {}}
        onExportJson={() => {}}
        onExportGql={() => {}}
      />,
    );
    const snippet = (screen.getByTestId("share-embed-snippet") as HTMLTextAreaElement).value;
    expect(snippet).toMatch(/^<iframe /);
    expect(snippet).toContain("?view=");
    expect(snippet).toContain("iris");
  });

  it("closes on Escape", () => {
    const onClose = vi.fn();
    render(
      <ShareModal
        open
        bundle="iris"
        currentSpec={SPEC}
        onClose={onClose}
        onExportCsv={() => {}}
        onExportJson={() => {}}
        onExportGql={() => {}}
      />,
    );
    fireEvent.keyDown(document, { key: "Escape" });
    expect(onClose).toHaveBeenCalled();
  });

  it("closes when the dimmed backdrop is clicked", () => {
    const onClose = vi.fn();
    render(
      <ShareModal
        open
        bundle="iris"
        currentSpec={SPEC}
        onClose={onClose}
        onExportCsv={() => {}}
        onExportJson={() => {}}
        onExportGql={() => {}}
      />,
    );
    fireEvent.click(screen.getByTestId("share-bg"));
    expect(onClose).toHaveBeenCalled();
  });
});
