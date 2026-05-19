import { useCallback, useEffect, useState } from "react";
import {
  bundleFromLocation,
  pathForBundle,
  pathForSystem,
  systemPageFromPath,
  type SystemPage,
} from "./route";

function currentBundle(): string | null {
  if (typeof window === "undefined") return null;
  return bundleFromLocation(window.location.pathname, window.location.hash);
}

/**
 * Client-side router for bundle navigation, with multi-tab support.
 *
 * The URL still encodes the **active** bundle as the path segment so
 * deep links / shares work the same. The list of *open* tabs is held
 * in React state + mirrored to sessionStorage so a refresh keeps the
 * same workspace.
 *
 * Navigation primitives:
 *   navigateToBundle(name)   — switch active. Adds to tabs if not already open.
 *   openInNewTab(name)       — add to tabs without switching active.
 *   closeTab(name)           — remove from tabs. If active, falls back to the
 *                              previous tab; if no tabs remain, goes to picker.
 *   navigateToPicker()       — return to the BundlePicker.
 */
const TABS_STORAGE_KEY = "gigi.sheets.open_tabs";

function readTabs(): string[] {
  if (typeof sessionStorage === "undefined") return [];
  const raw = sessionStorage.getItem(TABS_STORAGE_KEY);
  if (!raw) return [];
  try {
    const parsed = JSON.parse(raw);
    if (Array.isArray(parsed) && parsed.every((x) => typeof x === "string")) {
      return parsed;
    }
  } catch {
    /* fall through */
  }
  return [];
}

function writeTabs(tabs: string[]): void {
  if (typeof sessionStorage === "undefined") return;
  sessionStorage.setItem(TABS_STORAGE_KEY, JSON.stringify(tabs));
}

export interface BundleRoute {
  /** The currently-active bundle (URL path segment). null = picker or system page. */
  bundle: string | null;
  /**
   * Reserved system pages that live alongside bundles under /gigi/sheets/.
   * When set, the app should render the corresponding shell (welcome /
   * account) instead of the picker or bundle UI.
   */
  systemPage: SystemPage | null;
  /** The full list of open bundle tabs, in insertion order. */
  tabs: string[];
  /** Switch active to this bundle. Adds to tabs if not already open. */
  navigateToBundle: (name: string) => void;
  /** Open in a tab without making it active. */
  openInNewTab: (name: string) => void;
  /** Close a tab. If it was active, falls back to the previous tab or picker. */
  closeTab: (name: string) => void;
  /** Return to the picker (no active bundle). */
  navigateToPicker: () => void;
  /** Navigate to a reserved system page (welcome / account). */
  navigateToSystem: (page: SystemPage) => void;
}

export function useBundleRoute(): BundleRoute {
  const [bundle, setBundle] = useState<string | null>(() => currentBundle());
  const [systemPage, setSystemPage] = useState<SystemPage | null>(() =>
    typeof window !== "undefined"
      ? systemPageFromPath(window.location.pathname)
      : null,
  );
  const [tabs, setTabs] = useState<string[]>(() => {
    const stored = readTabs();
    const initialBundle = currentBundle();
    // Make sure the currently-active bundle is in the tab list.
    if (initialBundle && !stored.includes(initialBundle)) {
      const next = [...stored, initialBundle];
      writeTabs(next);
      return next;
    }
    return stored;
  });

  useEffect(() => {
    function syncFromUrl() {
      setBundle(currentBundle());
      setSystemPage(systemPageFromPath(window.location.pathname));
    }
    window.addEventListener("popstate", syncFromUrl);
    // Bundle URLs are hash-based (see route.ts) so we ALSO have to
    // listen on hashchange — popstate doesn't fire when only the
    // fragment changes.
    window.addEventListener("hashchange", syncFromUrl);
    return () => {
      window.removeEventListener("popstate", syncFromUrl);
      window.removeEventListener("hashchange", syncFromUrl);
    };
  }, []);

  // Helper: bundle URLs now carry the bundle name in the URL hash
  // (e.g. /gigi/sheets/#sensors), so the comparison can't just be
  // pathname === next — we need the full pathname+hash too.
  function currentPathPlusHash(): string {
    return window.location.pathname + window.location.hash;
  }

  const navigateToBundle = useCallback(
    (name: string) => {
      const next = pathForBundle(name);
      if (currentPathPlusHash() !== next) {
        window.history.pushState({}, "", next);
      }
      setBundle(name);
      setSystemPage(null);
      setTabs((prev) => {
        if (prev.includes(name)) return prev;
        const updated = [...prev, name];
        writeTabs(updated);
        return updated;
      });
    },
    [],
  );

  const navigateToSystem = useCallback((page: SystemPage) => {
    const next = pathForSystem(page);
    if (currentPathPlusHash() !== next) {
      window.history.pushState({}, "", next);
    }
    setBundle(null);
    setSystemPage(page);
  }, []);

  const openInNewTab = useCallback(
    (name: string) => {
      setTabs((prev) => {
        if (prev.includes(name)) return prev;
        const updated = [...prev, name];
        writeTabs(updated);
        return updated;
      });
    },
    [],
  );

  const closeTab = useCallback(
    (name: string) => {
      setTabs((prev) => {
        const idx = prev.indexOf(name);
        if (idx < 0) return prev;
        const updated = prev.filter((x) => x !== name);
        writeTabs(updated);

        // If we just closed the active bundle, fall back somewhere.
        if (bundle === name) {
          if (updated.length === 0) {
            const nextPath = "/gigi/sheets/";
            if (currentPathPlusHash() !== nextPath) {
              window.history.pushState({}, "", nextPath);
            }
            setBundle(null);
          } else {
            // Prefer the tab to the left of the one we closed; otherwise
            // fall back to the first remaining tab.
            const fallback = updated[Math.max(0, idx - 1)] ?? updated[0];
            const nextPath = pathForBundle(fallback);
            if (currentPathPlusHash() !== nextPath) {
              window.history.pushState({}, "", nextPath);
            }
            setBundle(fallback);
          }
        }
        return updated;
      });
    },
    [bundle],
  );

  const navigateToPicker = useCallback(() => {
    const next = "/gigi/sheets/";
    if (currentPathPlusHash() !== next) {
      window.history.pushState({}, "", next);
    }
    setBundle(null);
    setSystemPage(null);
    // Keep tabs intact — going to picker doesn't close them.
  }, []);

  // Keep persistence in sync if `tabs` is updated outside `persist` (e.g. via
  // navigateToBundle's state updater). Sequential consistency is handled by
  // writing inside each updater. This effect is a belt-and-braces safety net.
  useEffect(() => {
    writeTabs(tabs);
    // We don't depend on persist; just on the canonical tabs value.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [tabs.join("|")]);

  return {
    bundle,
    systemPage,
    tabs,
    navigateToBundle,
    openInNewTab,
    closeTab,
    navigateToPicker,
    navigateToSystem,
  };
}
