import { describe, expect, it, vi } from "vitest";
import { fireEvent, render, screen } from "@testing-library/react";
import { Banner } from "../../src/components/Banner";

describe("Banner", () => {
  it("renders nothing when message is null", () => {
    render(<Banner message={null} onDismiss={() => {}} />);
    expect(screen.queryByTestId("banner")).toBeNull();
  });

  it("renders the text + kind data attribute", () => {
    render(
      <Banner
        message={{ kind: "warn", text: "3 anomalies detected" }}
        onDismiss={() => {}}
      />,
    );
    const el = screen.getByTestId("banner");
    expect(el).toHaveTextContent("3 anomalies detected");
    expect(el).toHaveAttribute("data-kind", "warn");
  });

  it("fires onDismiss when the close button is clicked", () => {
    const onDismiss = vi.fn();
    render(
      <Banner
        message={{ kind: "info", text: "hi" }}
        onDismiss={onDismiss}
      />,
    );
    fireEvent.click(screen.getByTestId("banner-dismiss"));
    expect(onDismiss).toHaveBeenCalledOnce();
  });

  it("renders a primary action button when message.action is supplied", () => {
    const onAction = vi.fn();
    render(
      <Banner
        message={{
          kind: "warn",
          text: "3 anomalies",
          action: { label: "Filter", onClick: onAction },
        }}
        onDismiss={() => {}}
      />,
    );
    fireEvent.click(screen.getByTestId("banner-action"));
    expect(onAction).toHaveBeenCalledOnce();
  });

  it("supports all three kinds (info / warn / error) with distinct classes", () => {
    const { rerender } = render(
      <Banner message={{ kind: "info", text: "x" }} onDismiss={() => {}} />,
    );
    expect(screen.getByTestId("banner").className).toMatch(/banner-info/);
    rerender(
      <Banner message={{ kind: "warn", text: "x" }} onDismiss={() => {}} />,
    );
    expect(screen.getByTestId("banner").className).toMatch(/banner-warn/);
    rerender(
      <Banner message={{ kind: "error", text: "x" }} onDismiss={() => {}} />,
    );
    expect(screen.getByTestId("banner").className).toMatch(/banner-error/);
  });
});
