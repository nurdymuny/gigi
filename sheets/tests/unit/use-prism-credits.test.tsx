import { describe, expect, it, beforeEach } from "vitest";
import { act, renderHook } from "@testing-library/react";
import { usePrismCredits, FREE_RUN_LIMIT } from "../../src/lib/use-prism-credits";

beforeEach(() => {
  localStorage.clear();
});

describe("usePrismCredits", () => {
  it("starts with the full free-run allowance for a new browser", () => {
    const { result } = renderHook(() => usePrismCredits({ subscribed: false }));
    expect(result.current.used).toBe(0);
    expect(result.current.remaining).toBe(FREE_RUN_LIMIT);
    expect(result.current.limit).toBe(FREE_RUN_LIMIT);
    expect(result.current.unlimited).toBe(false);
    expect(result.current.canRun).toBe(true);
  });

  it("consume() increments used and decrements remaining", () => {
    const { result } = renderHook(() => usePrismCredits({ subscribed: false }));
    act(() => {
      result.current.consume();
    });
    expect(result.current.used).toBe(1);
    expect(result.current.remaining).toBe(FREE_RUN_LIMIT - 1);
  });

  it("canRun flips to false after the limit is reached", () => {
    const { result } = renderHook(() => usePrismCredits({ subscribed: false }));
    act(() => {
      for (let i = 0; i < FREE_RUN_LIMIT; i++) result.current.consume();
    });
    expect(result.current.used).toBe(FREE_RUN_LIMIT);
    expect(result.current.remaining).toBe(0);
    expect(result.current.canRun).toBe(false);
  });

  it("consume() past the limit is a no-op when not subscribed", () => {
    const { result } = renderHook(() => usePrismCredits({ subscribed: false }));
    act(() => {
      for (let i = 0; i < FREE_RUN_LIMIT + 5; i++) result.current.consume();
    });
    expect(result.current.used).toBe(FREE_RUN_LIMIT);
  });

  it("subscribed users have unlimited runs", () => {
    const { result } = renderHook(() => usePrismCredits({ subscribed: true }));
    expect(result.current.unlimited).toBe(true);
    expect(result.current.canRun).toBe(true);
    expect(result.current.remaining).toBe(Infinity);
    act(() => {
      for (let i = 0; i < 100; i++) result.current.consume();
    });
    // Used still counts (for analytics) but doesn't gate.
    expect(result.current.canRun).toBe(true);
  });

  it("persists used count to localStorage", () => {
    const { result } = renderHook(() => usePrismCredits({ subscribed: false }));
    act(() => {
      result.current.consume();
      result.current.consume();
    });
    expect(localStorage.getItem("gigi.sheets.prism_credits_used")).toBe("2");
  });

  it("rehydrates used count from localStorage on fresh hook mount", () => {
    localStorage.setItem("gigi.sheets.prism_credits_used", "2");
    const { result } = renderHook(() => usePrismCredits({ subscribed: false }));
    expect(result.current.used).toBe(2);
    expect(result.current.remaining).toBe(FREE_RUN_LIMIT - 2);
  });

  it("reset() wipes the count (used in tests / debug)", () => {
    const { result } = renderHook(() => usePrismCredits({ subscribed: false }));
    act(() => {
      result.current.consume();
      result.current.consume();
    });
    expect(result.current.used).toBe(2);
    act(() => {
      result.current.reset();
    });
    expect(result.current.used).toBe(0);
  });
});
