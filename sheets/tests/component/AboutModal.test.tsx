import { describe, expect, it, vi } from "vitest";
import { fireEvent, render, screen } from "@testing-library/react";
import { AboutModal } from "../../src/components/AboutModal";

describe("AboutModal", () => {
  it("renders nothing when closed", () => {
    render(<AboutModal open={false} onClose={() => {}} />);
    expect(screen.queryByTestId("about-modal")).toBeNull();
  });

  it("renders the hero with brand mark + title + byline when open", () => {
    render(<AboutModal open onClose={() => {}} />);
    const modal = screen.getByTestId("about-modal");
    expect(modal).toBeInTheDocument();
    expect(modal).toHaveTextContent("GIGI");
    expect(modal).toHaveTextContent(/Geometric Intrinsic Global Index/);
    expect(modal).toHaveTextContent(/Bee Rosa Davis/);
    expect(modal).toHaveTextContent(/Davis Geometric/);
  });

  it("defaults to the engine tab and lists key concepts", () => {
    render(<AboutModal open onClose={() => {}} />);
    expect(screen.getByTestId("about-engine")).toBeInTheDocument();
    // Geometric concept callouts.
    const body = screen.getByTestId("about-engine");
    expect(body).toHaveTextContent(/O\(1\) point queries/);
    expect(body).toHaveTextContent(/Gauge encryption/);
    expect(body).toHaveTextContent(/DHOOM/);
    // Every geometric verb shows up.
    for (const verb of [
      "SECTION",
      "INTEGRATE",
      "CURVATURE",
      "SPECTRAL",
      "HOLONOMY",
      "TRANSPORT",
      "BETTI",
      "GEODESIC",
    ]) {
      expect(body).toHaveTextContent(verb);
    }
  });

  it("switches to the person tab and shows Gigi's bio", () => {
    render(<AboutModal open onClose={() => {}} />);
    fireEvent.click(screen.getByTestId("about-tab-person"));
    const body = screen.getByTestId("about-person");
    expect(body).toBeInTheDocument();
    expect(body).toHaveTextContent(/Bee Rosa Davis/);
    expect(body).toHaveTextContent(/Gigi/);
    // The products she made.
    for (const product of ["GIGI", "KRAKEN", "Marcella", "ICARUS", "Just-Gigi"]) {
      expect(body).toHaveTextContent(product);
    }
  });

  it("marks the active tab via aria-selected", () => {
    render(<AboutModal open onClose={() => {}} />);
    const engineTab = screen.getByTestId("about-tab-engine");
    const personTab = screen.getByTestId("about-tab-person");
    expect(engineTab).toHaveAttribute("aria-selected", "true");
    expect(personTab).toHaveAttribute("aria-selected", "false");
    fireEvent.click(personTab);
    expect(engineTab).toHaveAttribute("aria-selected", "false");
    expect(personTab).toHaveAttribute("aria-selected", "true");
  });

  it("closes via the X button", () => {
    const onClose = vi.fn();
    render(<AboutModal open onClose={onClose} />);
    fireEvent.click(screen.getByTestId("about-close"));
    expect(onClose).toHaveBeenCalledOnce();
  });

  it("closes on Escape", () => {
    const onClose = vi.fn();
    render(<AboutModal open onClose={onClose} />);
    fireEvent.keyDown(document, { key: "Escape" });
    expect(onClose).toHaveBeenCalled();
  });

  it("closes when the dimmed backdrop is clicked", () => {
    const onClose = vi.fn();
    render(<AboutModal open onClose={onClose} />);
    fireEvent.click(screen.getByTestId("about-bg"));
    expect(onClose).toHaveBeenCalled();
  });

  it("does NOT close when the inner modal is clicked", () => {
    const onClose = vi.fn();
    render(<AboutModal open onClose={onClose} />);
    fireEvent.click(screen.getByTestId("about-modal"));
    expect(onClose).not.toHaveBeenCalled();
  });
});
