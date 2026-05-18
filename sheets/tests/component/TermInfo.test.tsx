import { describe, expect, it, vi } from "vitest";
import { fireEvent, render, screen } from "@testing-library/react";
import { TermInfo } from "../../src/components/TermInfo";
import { GLOSSARY, lookupTerm } from "../../src/lib/geometry-glossary";

describe("geometry-glossary", () => {
  it("has entries for every verb GIGI surfaces", () => {
    for (const k of [
      "cover",
      "kappa",
      "curvature",
      "section",
      "spectral",
      "transport",
      "holonomy",
      "betti",
      "integrate",
      "geodesic",
      "capacity",
      "confidence",
    ]) {
      expect(GLOSSARY[k], `missing glossary entry: ${k}`).toBeDefined();
    }
  });

  it("each entry has summary + body + example", () => {
    for (const [key, entry] of Object.entries(GLOSSARY)) {
      expect(entry.title.length, `${key} title`).toBeGreaterThan(0);
      expect(entry.summary.length, `${key} summary`).toBeGreaterThan(20);
      expect(entry.body.length, `${key} body paragraphs`).toBeGreaterThan(0);
      expect(entry.example.setup.length, `${key} setup`).toBeGreaterThan(20);
      expect(entry.example.result.length, `${key} result`).toBeGreaterThan(20);
    }
  });

  it("lookupTerm is case-insensitive", () => {
    expect(lookupTerm("SPECTRAL")).toBe(GLOSSARY.spectral);
    expect(lookupTerm("Spectral")).toBe(GLOSSARY.spectral);
    expect(lookupTerm("spectral")).toBe(GLOSSARY.spectral);
  });

  it("lookupTerm returns null for unknown terms", () => {
    expect(lookupTerm("not-a-real-verb")).toBeNull();
  });
});

describe("TermInfo", () => {
  it("renders an info button for a known term, hidden modal initially", () => {
    render(<TermInfo term="spectral" />);
    expect(screen.getByTestId("term-info-spectral")).toBeInTheDocument();
    expect(screen.queryByTestId("term-info-modal-spectral")).toBeNull();
  });

  it("renders nothing for an unknown term", () => {
    const { container } = render(<TermInfo term="not-a-thing" />);
    expect(container.firstChild).toBeNull();
  });

  it("clicking the icon opens the modal with the right title + summary", () => {
    render(<TermInfo term="holonomy" />);
    fireEvent.click(screen.getByTestId("term-info-holonomy"));
    const modal = screen.getByTestId("term-info-modal-holonomy");
    expect(modal).toBeInTheDocument();
    expect(modal).toHaveTextContent("HOLONOMY");
    expect(modal).toHaveTextContent(/closed loop/i);
  });

  it("the modal renders an example block (setup + result)", () => {
    render(<TermInfo term="betti" />);
    fireEvent.click(screen.getByTestId("term-info-betti"));
    const modal = screen.getByTestId("term-info-modal-betti");
    expect(modal).toHaveTextContent(/Setup/i);
    expect(modal).toHaveTextContent(/Result/i);
  });

  it("the modal renders a GQL snippet when the entry has one", () => {
    render(<TermInfo term="transport" />);
    fireEvent.click(screen.getByTestId("term-info-transport"));
    const modal = screen.getByTestId("term-info-modal-transport");
    expect(modal).toHaveTextContent(/TRANSPORT.*FROM.*TO/);
  });

  it("clicking the X button closes the modal", () => {
    render(<TermInfo term="spectral" />);
    fireEvent.click(screen.getByTestId("term-info-spectral"));
    expect(screen.getByTestId("term-info-modal-spectral")).toBeInTheDocument();
    fireEvent.click(screen.getByTestId("term-info-close"));
    expect(screen.queryByTestId("term-info-modal-spectral")).toBeNull();
  });

  it("clicking the dimmed backdrop closes the modal", () => {
    render(<TermInfo term="spectral" />);
    fireEvent.click(screen.getByTestId("term-info-spectral"));
    fireEvent.click(screen.getByTestId("term-info-bg"));
    expect(screen.queryByTestId("term-info-modal-spectral")).toBeNull();
  });

  it("pressing Escape closes the modal", () => {
    render(<TermInfo term="spectral" />);
    fireEvent.click(screen.getByTestId("term-info-spectral"));
    fireEvent.keyDown(window, { key: "Escape" });
    expect(screen.queryByTestId("term-info-modal-spectral")).toBeNull();
  });

  it("the icon click does not bubble (so it can sit inside row/label click targets)", () => {
    const onRowClick = vi.fn();
    render(
      <div onClick={onRowClick}>
        <TermInfo term="spectral" />
      </div>,
    );
    fireEvent.click(screen.getByTestId("term-info-spectral"));
    expect(onRowClick).not.toHaveBeenCalled();
  });
});

