import { describe, expect, it, beforeEach, afterEach } from "vitest";
import { act, renderHook } from "@testing-library/react";
import { useBundleRoute } from "../../src/lib/use-bundle-route";

beforeEach(() => {
  window.history.replaceState({}, "", "/gigi/sheets/");
  sessionStorage.clear();
});
afterEach(() => {
  window.history.replaceState({}, "", "/gigi/sheets/");
  sessionStorage.clear();
});

describe("useBundleRoute", () => {
  it("reads the initial bundle from window.location.pathname", () => {
    window.history.replaceState({}, "", "/gigi/sheets/sensors");
    const { result } = renderHook(() => useBundleRoute());
    expect(result.current.bundle).toBe("sensors");
  });

  it("returns null when on the picker route", () => {
    window.history.replaceState({}, "", "/gigi/sheets/");
    const { result } = renderHook(() => useBundleRoute());
    expect(result.current.bundle).toBeNull();
  });

  it("navigateToBundle pushes a new path AND updates state", () => {
    const { result } = renderHook(() => useBundleRoute());
    act(() => result.current.navigateToBundle("iris"));
    expect(result.current.bundle).toBe("iris");
    expect(window.location.pathname).toBe("/gigi/sheets/iris");
  });

  it("navigateToBundle adds the new bundle to the open tabs list", () => {
    const { result } = renderHook(() => useBundleRoute());
    act(() => result.current.navigateToBundle("iris"));
    act(() => result.current.navigateToBundle("nba_2024"));
    expect(result.current.tabs).toEqual(["iris", "nba_2024"]);
  });

  it("navigateToBundle to an already-open tab keeps tab order stable", () => {
    const { result } = renderHook(() => useBundleRoute());
    act(() => result.current.navigateToBundle("iris"));
    act(() => result.current.navigateToBundle("nba_2024"));
    act(() => result.current.navigateToBundle("iris"));
    expect(result.current.tabs).toEqual(["iris", "nba_2024"]);
    expect(result.current.bundle).toBe("iris");
  });

  it("openInNewTab adds a tab without changing active", () => {
    const { result } = renderHook(() => useBundleRoute());
    act(() => result.current.navigateToBundle("iris"));
    act(() => result.current.openInNewTab("nba_2024"));
    expect(result.current.tabs).toEqual(["iris", "nba_2024"]);
    expect(result.current.bundle).toBe("iris");
  });

  it("closeTab removes a non-active tab and leaves the active bundle alone", () => {
    const { result } = renderHook(() => useBundleRoute());
    act(() => result.current.navigateToBundle("iris"));
    act(() => result.current.openInNewTab("nba_2024"));
    act(() => result.current.closeTab("nba_2024"));
    expect(result.current.tabs).toEqual(["iris"]);
    expect(result.current.bundle).toBe("iris");
  });

  it("closeTab on the active bundle falls back to the previous tab", () => {
    const { result } = renderHook(() => useBundleRoute());
    act(() => result.current.navigateToBundle("iris"));
    act(() => result.current.navigateToBundle("nba_2024"));
    act(() => result.current.closeTab("nba_2024"));
    expect(result.current.bundle).toBe("iris");
    expect(window.location.pathname).toBe("/gigi/sheets/iris");
  });

  it("closeTab on the last open bundle returns to the picker", () => {
    const { result } = renderHook(() => useBundleRoute());
    act(() => result.current.navigateToBundle("iris"));
    act(() => result.current.closeTab("iris"));
    expect(result.current.bundle).toBeNull();
    expect(result.current.tabs).toEqual([]);
    expect(window.location.pathname).toBe("/gigi/sheets/");
  });

  it("navigateToPicker keeps tabs intact (doesn't close them)", () => {
    const { result } = renderHook(() => useBundleRoute());
    act(() => result.current.navigateToBundle("iris"));
    act(() => result.current.openInNewTab("nba_2024"));
    act(() => result.current.navigateToPicker());
    expect(result.current.bundle).toBeNull();
    expect(result.current.tabs).toEqual(["iris", "nba_2024"]);
  });

  it("persists tabs to sessionStorage", () => {
    const { result } = renderHook(() => useBundleRoute());
    act(() => result.current.navigateToBundle("iris"));
    act(() => result.current.openInNewTab("nba_2024"));
    expect(sessionStorage.getItem("gigi.sheets.open_tabs")).toContain("iris");
    expect(sessionStorage.getItem("gigi.sheets.open_tabs")).toContain("nba_2024");
  });

  it("rehydrates tabs from sessionStorage on fresh mount", () => {
    sessionStorage.setItem(
      "gigi.sheets.open_tabs",
      JSON.stringify(["iris", "nba_2024", "world_cities"]),
    );
    window.history.replaceState({}, "", "/gigi/sheets/iris");
    const { result } = renderHook(() => useBundleRoute());
    expect(result.current.tabs).toEqual(["iris", "nba_2024", "world_cities"]);
    expect(result.current.bundle).toBe("iris");
  });

  it("adds the current URL bundle to the tab list if it isn't already there", () => {
    sessionStorage.setItem("gigi.sheets.open_tabs", JSON.stringify(["nba_2024"]));
    window.history.replaceState({}, "", "/gigi/sheets/iris");
    const { result } = renderHook(() => useBundleRoute());
    expect(result.current.tabs).toEqual(["nba_2024", "iris"]);
  });

  it("responds to browser back (popstate) by re-reading the pathname", () => {
    const { result } = renderHook(() => useBundleRoute());
    act(() => result.current.navigateToBundle("iris"));
    expect(result.current.bundle).toBe("iris");
    act(() => {
      window.history.replaceState({}, "", "/gigi/sheets/");
      window.dispatchEvent(new PopStateEvent("popstate"));
    });
    expect(result.current.bundle).toBeNull();
  });
});
