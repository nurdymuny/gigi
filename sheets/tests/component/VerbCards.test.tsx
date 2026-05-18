import { describe, expect, it } from "vitest";
import { render, screen, within } from "@testing-library/react";
import {
  BettiCard,
  HolonomyCard,
  SpectralCard,
  TransportCard,
} from "../../src/components/VerbCards";

describe("SpectralCard", () => {
  it("renders λ₁, diameter, capacity bars from the result", () => {
    render(
      <SpectralCard data={{ lambda1: 0.082, diameter: 7, spectral_capacity: 0.412 }} />,
    );
    expect(screen.getByTestId("result-spectral")).toBeInTheDocument();
    expect(screen.getByTestId("bar-λ₁")).toHaveTextContent("0.082");
    expect(screen.getByTestId("bar-diam")).toHaveTextContent("7");
    expect(screen.getByTestId("bar-C")).toHaveTextContent("0.412");
  });

  it("interprets high λ₁ as well-connected", () => {
    render(
      <SpectralCard data={{ lambda1: 0.5, diameter: 3, spectral_capacity: 0.9 }} />,
    );
    expect(screen.getByTestId("result-spectral")).toHaveTextContent(/well-connected/i);
  });

  it("interprets low λ₁ as loose / disconnected", () => {
    render(
      <SpectralCard data={{ lambda1: 0.05, diameter: 12, spectral_capacity: 0.1 }} />,
    );
    expect(screen.getByTestId("result-spectral")).toHaveTextContent(/loose|disconnected/i);
  });
});

describe("BettiCard", () => {
  it("renders b₀, b₁, and Euler characteristic χ = b₀ - b₁", () => {
    render(<BettiCard data={{ beta_0: 4, beta_1: 2 }} />);
    expect(screen.getByTestId("result-betti")).toBeInTheDocument();
    expect(screen.getByTestId("betti-chi")).toHaveTextContent("2");
  });

  it("handles a flat (single connected component) bundle", () => {
    render(<BettiCard data={{ beta_0: 1, beta_1: 0 }} />);
    expect(screen.getByTestId("betti-chi")).toHaveTextContent("1");
  });
});

describe("TransportCard", () => {
  it("renders a 2×2 matrix with exactly 4 cells and the angle in rad + deg", () => {
    const data = {
      dim: 2,
      angle: Math.PI / 4, // 45°
      matrix: [0.707, -0.707, 0.707, 0.707],
    };
    render(<TransportCard data={data} from="S-001" to="S-002" />);
    const card = screen.getByTestId("result-transport");
    expect(card).toHaveTextContent("S-001");
    expect(card).toHaveTextContent("S-002");
    const matrix = screen.getByTestId("matrix");
    expect(matrix.children).toHaveLength(4);
    expect(matrix).toHaveTextContent("0.707");
    expect(matrix).toHaveTextContent("-0.707");
    // 45° in degrees
    expect(card).toHaveTextContent("45.0");
  });

  it("renders an N×N matrix for arbitrary fiber dimension", () => {
    const data = {
      dim: 3,
      angle: 0.3,
      // 3×3 identity-ish
      matrix: [1, 0, 0, 0, 1, 0, 0, 0, 1],
    };
    render(<TransportCard data={data} from="A" to="B" />);
    const matrix = screen.getByTestId("matrix");
    expect(matrix.children).toHaveLength(9);
  });

  it("flags large rotations as 'large rotation'", () => {
    const data = {
      dim: 2,
      angle: 2.5,
      matrix: [0, 0, 0, 0],
    };
    render(<TransportCard data={data} from="A" to="B" />);
    expect(screen.getByTestId("result-transport")).toHaveTextContent(/large rotation/i);
  });

  it("flags small rotations as 'small rotation'", () => {
    const data = {
      dim: 2,
      angle: 0.1,
      matrix: [0, 0, 0, 0],
    };
    render(<TransportCard data={data} from="A" to="B" />);
    expect(screen.getByTestId("result-transport")).toHaveTextContent(/small rotation/i);
  });

  it("uses the engine matrix verbatim (no client-side reconstruction)", () => {
    // Exact, asymmetric numbers — the test fails if anyone replaces these
    // with cos(θ)/sin(θ) reconstructions instead of trusting the response.
    const data = {
      dim: 2,
      angle: 0,
      matrix: [0.111, 0.222, 0.333, 0.444],
    };
    render(<TransportCard data={data} from="A" to="B" />);
    const matrix = screen.getByTestId("matrix");
    const cells = within(matrix).getAllByText(/\d\.\d{3}/);
    expect(cells.map((c) => c.textContent)).toEqual([
      "0.111",
      "0.222",
      "0.333",
      "0.444",
    ]);
  });
});

describe("HolonomyCard", () => {
  it("renders δφ in rad + deg and flags trivial cases", () => {
    render(
      <HolonomyCard
        data={{ angle: 0.000001, trivial: true, centroids: [] }}
        around="site_id"
      />,
    );
    expect(screen.getByTestId("result-holonomy")).toHaveTextContent(/trivial|flat/i);
  });

  it("flags non-trivial holonomy with the centroid count", () => {
    render(
      <HolonomyCard
        data={{
          angle: 0.5,
          trivial: false,
          centroids: [
            { label: "N", fx: 22, fy: 60, transport_angle: 0.1 },
            { label: "S", fx: 25, fy: 55, transport_angle: 0.4 },
            { label: "E", fx: 21, fy: 62, transport_angle: 0.0 },
          ],
        }}
        around="site_id"
      />,
    );
    const card = screen.getByTestId("result-holonomy");
    expect(card).toHaveTextContent(/3 cohorts/);
    expect(card).toHaveTextContent("0.500"); // rad
  });

  it("converts angle to degrees correctly", () => {
    render(
      <HolonomyCard
        data={{ angle: Math.PI, trivial: false, centroids: [] }}
        around="site_id"
      />,
    );
    expect(screen.getByTestId("result-holonomy")).toHaveTextContent("180.0");
  });
});
