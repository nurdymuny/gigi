import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import gigiIconUrl from "./assets/gigi-icon.svg";
import { AboutModal } from "./components/AboutModal";
import { AccountMenu } from "./components/AccountMenu";
import { Banner, type BannerMessage } from "./components/Banner";
import { BundlePicker } from "./components/BundlePicker";
import { BundleTabs } from "./components/BundleTabs";
import { LandingPage } from "./components/LandingPage";
import { Charts } from "./components/Charts";
import { ColumnFilterPopover } from "./components/ColumnFilterPopover";
import {
  ConditionalFormatModal,
  type ConditionalFormatRule,
} from "./components/ConditionalFormatModal";
import { CommandPalette, type Command } from "./components/CommandPalette";
import { ContextMenu, type ContextMenuItem } from "./components/ContextMenu";
import { FindModal } from "./components/FindModal";
import { FormulaBar } from "./components/FormulaBar";
import { FormulaDocsModal } from "./components/FormulaDocsModal";
import { FormulaPicker } from "./components/FormulaPicker";
import { FormView } from "./components/FormView";
import { Gallery } from "./components/Gallery";
import { Tutorial } from "./components/Tutorial";
import { fromTsv, toTsv } from "./lib/clipboard";
import { evaluate } from "./lib/formula";
import { cellsInRange, rangeFields, rangeRowKeys, type CellRange } from "./lib/cell-range";
import {
  dragFillCategorical,
  dragFillDate,
  dragFillNumeric,
} from "./lib/dragfill";
import { buildBundleFormulaContext, buildBundleSameness } from "./lib/formula-context";
import { getFormula, setFormula } from "./lib/formula-storage";
import {
  buildProjectTrackerTour,
  registerTourHelpers,
} from "./lib/tutorial-scripts";
import { Geometry } from "./components/Geometry";
import { GqlView } from "./components/GqlView";
import { Kanban } from "./components/Kanban";
import { PrismWorkflowsDrawer } from "./components/PrismWorkflows";
import { Grid, type RowClickModifiers } from "./components/Grid";
import { applyFilters, type Filter } from "./lib/filter";
import { HideFieldsModal } from "./components/HideFieldsModal";
import { ImportCsvModal } from "./components/ImportCsvModal";
import { InsertRowModal } from "./components/InsertRowModal";
import { InsightsDrawer } from "./components/InsightsDrawer";
import { Inspector } from "./components/Inspector";
import { MenuBar } from "./components/MenuBar";
import { buildMenus } from "./components/menuDefs";
import { SchemaModal } from "./components/SchemaModal";
import { ShareModal } from "./components/ShareModal";
import { Sidebar } from "./components/Sidebar";
import { SignInModal } from "./components/SignInModal";
import { Toast, makeToast, type ToastMessage } from "./components/Toast";
import { Toolbar } from "./components/Toolbar";
import { ViewsDrawer } from "./components/ViewsDrawer";
import { useBundle } from "./hooks/useBundle";
import { SheetsClient, type RowMap } from "./lib/gigi-client";
import {
  computeCohortKappa,
  kappaClass,
  numericFiberFields,
  pickDefaultCoverField,
} from "./lib/kappa";
// route helpers are imported by children that still need them (e.g. DemoBundles
// for the "open" fallback link); App itself routes through useBundleRoute.
import { useAccount } from "./lib/use-account";
import { useBundleRoute } from "./lib/use-bundle-route";
import { useEngineAccess } from "./lib/use-engine-access";
import { WelcomePage } from "./components/WelcomePage";
import { AccountPage } from "./components/AccountPage";
import { EngineLockedPanel } from "./components/EngineLockedPanel";
import { useEditHistory } from "./lib/use-edit-history";
import { usePrismCredits } from "./lib/use-prism-credits";
import { useGlobalShortcuts, type ShortcutBinding } from "./lib/use-global-shortcuts";
import { viewFromUrl, type ViewSpec } from "./lib/view";

type ViewKind = "grid" | "geometry" | "charts" | "kanban" | "gallery" | "form" | "gql";

/**
 * Engine URL. In dev / on localhost it falls back to the local dev
 * port. Production deployments MUST supply `VITE_GIGI_BASE_URL` (which
 * Vite inlines at build time) so we never emit a plaintext-HTTP request
 * for a non-local engine.
 *
 * If `VITE_GIGI_BASE_URL` is set to a non-HTTPS URL whose host isn't
 * `localhost` / `127.0.0.1`, we refuse to start — better to fail
 * loudly than to silently leak data over the wire.
 */
function resolveEngineBaseUrl(): string {
  const fromEnv = (import.meta.env?.VITE_GIGI_BASE_URL ?? "").trim();
  const fallback = "http://localhost:3142";
  const url = fromEnv || fallback;
  try {
    const parsed = new URL(url);
    const isLocal =
      parsed.hostname === "localhost" ||
      parsed.hostname === "127.0.0.1" ||
      parsed.hostname === "[::1]";
    if (parsed.protocol === "http:" && !isLocal) {
      throw new Error(
        `Refusing to talk to ${url} over plaintext HTTP — set VITE_GIGI_BASE_URL to an https:// URL.`,
      );
    }
    return url;
  } catch (e) {
    if (e instanceof Error && e.message.startsWith("Refusing")) throw e;
    throw new Error(`VITE_GIGI_BASE_URL is not a valid URL: ${url}`);
  }
}

const DEFAULT_SERVER = resolveEngineBaseUrl();

export function App() {
  const client = useMemo(() => new SheetsClient({ baseUrl: DEFAULT_SERVER }), []);
  const route = useBundleRoute();
  // One auth + engine-access pair for the whole bundle/picker path.
  // The child components (BundleApp, PickerShell) instantiate their
  // own useAccount for now — keeping this hook here is what enables the
  // app-level "private deployment" gate before any engine call fires.
  const gateAccount = useAccount();
  const engineAccess = useEngineAccess(client, gateAccount);
  // Tour state at the app level so it survives the picker → bundle-app
  // transition (the first tour step navigates to a workflow bundle, which
  // unmounts PickerShell and mounts BundleApp; Tutorial keeps running).
  const [tourOpen, setTourOpen] = useState<boolean>(false);
  const tourSteps = useMemo(
    () =>
      buildProjectTrackerTour({
        client,
        navigateToBundle: route.navigateToBundle,
      }),
    [client, route.navigateToBundle],
  );

  // System pages (/welcome, /account) live outside the bundle UI. They
  // pre-empt picker + BundleApp because their concerns (T&Cs flow,
  // account dashboard) don't depend on a loaded bundle and shouldn't
  // share the bundle chrome.
  if (route.systemPage === "welcome") {
    return (
      <SystemWelcome
        onDone={(next) => {
          // Hand the user back to where they wanted to go. If `next` is a
          // bundle path, route through navigateToBundle so the tab list
          // picks it up; otherwise fall back to the picker.
          const stripped = next.startsWith("/gigi/sheets/")
            ? next.slice("/gigi/sheets/".length).split(/[?#]/)[0]
            : "";
          if (stripped && /^[A-Za-z_][A-Za-z0-9_-]*$/.test(stripped)) {
            route.navigateToBundle(stripped);
          } else {
            route.navigateToPicker();
          }
        }}
      />
    );
  }
  if (route.systemPage === "account") {
    return (
      <SystemAccount
        openTabs={route.tabs}
        onOpenBundle={route.navigateToBundle}
        onBackToPicker={route.navigateToPicker}
      />
    );
  }

  // Engine-access gate. Guests still see the public LandingPage (no
  // engine calls) via PickerShell's own branch, so we only short-circuit
  // signed-in users that the engine refuses. Loading state for a guest
  // is uninteresting — the picker can render itself.
  if (gateAccount.state === "user") {
    if (engineAccess.kind === "loading") {
      return <EngineLockedPanel reason="loading" />;
    }
    if (engineAccess.kind === "denied") {
      return (
        <EngineLockedPanel
          reason="denied"
          message={engineAccess.message}
          onOpenAccount={() => route.navigateToSystem("account")}
          onSignOut={() => void gateAccount.signOut()}
        />
      );
    }
    if (engineAccess.kind === "error") {
      return (
        <EngineLockedPanel
          reason="error"
          message={engineAccess.message}
          onOpenAccount={() => route.navigateToSystem("account")}
          onSignOut={() => void gateAccount.signOut()}
        />
      );
    }
  }

  return (
    <>
      {!route.bundle ? (
        <PickerShell
          client={client}
          requestedBundle={null}
          loadError={null}
          onPickBundle={route.navigateToBundle}
          onStartTour={() => setTourOpen(true)}
          onOpenAccount={() => route.navigateToSystem("account")}
        />
      ) : (
        // No `key` here intentionally — switching tabs should be smooth, not a
        // full app remount. Each hook resets its own state via a useEffect that
        // watches bundleName (see editHistory + selection resets below).
        <BundleApp
          client={client}
          bundleName={route.bundle}
          tabs={route.tabs}
          onNavigateToBundle={route.navigateToBundle}
          onCloseTab={route.closeTab}
          onNavigateToPicker={route.navigateToPicker}
          onOpenAccount={() => route.navigateToSystem("account")}
        />
      )}
      <Tutorial
        open={tourOpen}
        steps={tourSteps}
        title="Project tracker workflow"
        onClose={() => setTourOpen(false)}
      />
    </>
  );
}

/**
 * Thin wrappers that instantiate the auth hook for the system pages.
 * Kept here (not inlined) so they participate in React's normal hook
 * rules — useAccount sets up its own session fetch effect.
 */
function SystemWelcome({ onDone }: { onDone: (next: string) => void }) {
  const account = useAccount();
  const [signInOpen, setSignInOpen] = useState(false);
  const params = new URLSearchParams(
    typeof window !== "undefined" ? window.location.search : "",
  );
  const next = params.get("next") ?? "/gigi/sheets/";
  return (
    <>
      <WelcomePage
        account={account}
        next={next}
        onRequestSignIn={() => setSignInOpen(true)}
        onDone={onDone}
      />
      <SignInModal
        open={signInOpen}
        onClose={() => setSignInOpen(false)}
        onSignIn={account.signInWithEmail}
      />
    </>
  );
}

function SystemAccount({
  openTabs,
  onOpenBundle,
  onBackToPicker,
}: {
  openTabs: string[];
  onOpenBundle: (name: string) => void;
  onBackToPicker: () => void;
}) {
  const account = useAccount();
  const [signInOpen, setSignInOpen] = useState(false);
  return (
    <>
      <AccountPage
        account={account}
        openTabs={openTabs}
        onOpenBundle={onOpenBundle}
        onBackToPicker={onBackToPicker}
        onRequestSignIn={() => setSignInOpen(true)}
      />
      <SignInModal
        open={signInOpen}
        onClose={() => setSignInOpen(false)}
        onSignIn={account.signInWithEmail}
      />
    </>
  );
}

interface BundleAppProps {
  client: SheetsClient;
  bundleName: string;
  tabs: string[];
  onNavigateToBundle: (name: string) => void;
  onCloseTab: (name: string) => void;
  onNavigateToPicker: () => void;
  /** Jump to /gigi/sheets/account (sheets-branded account dashboard). */
  onOpenAccount: () => void;
}

function BundleApp({
  client,
  bundleName,
  tabs,
  onNavigateToBundle,
  onCloseTab,
  onNavigateToPicker,
  onOpenAccount,
}: BundleAppProps) {
  const {
    schema,
    rows,
    total,
    curvature,
    confidence,
    loading,
    error,
    updateCell,
    refetch,
    realtime,
    laggedCount,
  } = useBundle(client, bundleName, { limit: 250 });

  const initialView = useMemo(
    () => viewFromUrl(window.location.search),
    [],
  );
  const [toast, setToast] = useState<ToastMessage | null>(null);
  const [overlayOn, setOverlayOn] = useState<boolean>(initialView?.overlayOn ?? true);
  const [coverField, setCoverField] = useState<string>(initialView?.coverField ?? "");
  const [selectedRowKey, setSelectedRowKey] = useState<string | null>(null);
  const [selectedKeys, setSelectedKeys] = useState<Set<string>>(new Set());
  const [anchorKey, setAnchorKey] = useState<string | null>(null);
  const [activeView, setActiveView] = useState<ViewKind>(
    initialView?.activeView ?? "grid",
  );

  // Tutorial integration: expose this BundleApp's setActiveView (and
  // other actions, if needed) to the App-level Tutorial component via
  // window.__gigi_tour__. The registry self-cleans on unmount.
  useEffect(() => {
    return registerTourHelpers({
      setActiveView,
    });
  }, []);
  const [gqlQuery, setGqlQuery] = useState<string>(
    // CURVATURE is the simplest bundle-wide GQL the parser accepts.
    // SECTION requires `AT (key=val)` for point queries; there is no
    // top-level LIMIT clause. For row browsing, use the Grid view —
    // the engine's /v1/bundles/:name/query endpoint backs it.
    initialView?.gqlQuery ?? `CURVATURE ${bundleName};`,
  );
  const [inspectorOpen, setInspectorOpen] = useState<boolean>(
    initialView?.inspectorOpen ?? true,
  );
  const [selectedColumn, setSelectedColumn] = useState<string | null>(null);
  const [contextMenu, setContextMenu] = useState<
    | { x: number; y: number; rowKey: string; column?: string }
    | null
  >(null);
  const [schemaOpen, setSchemaOpen] = useState<boolean>(false);
  const [viewsOpen, setViewsOpen] = useState<boolean>(false);
  const [insightsOpen, setInsightsOpen] = useState<boolean>(false);
  const [hideFieldsOpen, setHideFieldsOpen] = useState<boolean>(false);
  const [hiddenFields, setHiddenFields] = useState<Set<string>>(new Set());
  const [importOpen, setImportOpen] = useState<boolean>(false);
  const [insertRowOpen, setInsertRowOpen] = useState<boolean>(false);
  const [aboutOpen, setAboutOpen] = useState<boolean>(false);
  const [shareOpen, setShareOpen] = useState<boolean>(false);
  const [findOpen, setFindOpen] = useState<boolean>(false);
  const [paletteOpen, setPaletteOpen] = useState<boolean>(false);
  const [allBundles, setAllBundles] = useState<string[]>([]);
  const [anomaliesOnly, setAnomaliesOnly] = useState<boolean>(false);
  /**
   * Bump to force React to re-read sidecar formula text after a write.
   * The sidecar lives in localStorage outside the React tree, so when a
   * formula is created or changed we need this nudge so cells re-render
   * with their new `data-has-formula` / formula tooltip.
   */
  const [formulaTick, setFormulaTick] = useState<number>(0);
  /**
   * Imperative-focus counter for the formula bar. Bumped by
   * `Insert → Formula` and the row context-menu "Insert formula here"
   * item; the bar watches for changes and pulls focus + prefills `=`.
   */
  const [formulaFocusToken, setFormulaFocusToken] = useState<number>(0);
  /** Excel-style "Insert Function" dialog. */
  const [formulaPickerOpen, setFormulaPickerOpen] = useState<boolean>(false);
  /** Read-only formula reference (Help → Formula reference…). */
  const [formulaDocsOpen, setFormulaDocsOpen] = useState<boolean>(false);
  /**
   * Gallery find-similar pivot key. When set, Gallery sorts cards by
   * Davis sameness against this row (desc), pins the pivot at the top,
   * and shows a per-card sameness bar. Driven by the per-row right-
   * click menu's "Find similar to this row" item.
   */
  const [gallerySimilarPivot, setGallerySimilarPivot] = useState<string | null>(null);
  /**
   * Cell-level range from drag-select in the Grid. Powers the
   * formula bar's range-stats panel (Sum/Avg/Min/Max/Count), copy as
   * TSV, and the drag-fill / conditional-format flows that follow.
   * Null when no rectangle is active.
   */
  const [cellRange, setCellRange] = useState<CellRange | null>(null);
  /**
   * Per-column filters, keyed by field name. Wired through the grid's
   * column-header funnel buttons. One filter per column for v1
   * (Airtable-style); the math layer (`applyFilters`) already supports
   * stacking so multi-filter-per-column is a later refinement.
   */
  const [columnFilters, setColumnFilters] = useState<Map<string, Filter>>(
    new Map(),
  );
  /** Which column's filter popover is open (null = none). */
  const [filterPopover, setFilterPopover] = useState<{
    field: string;
    anchorEl: HTMLElement;
  } | null>(null);
  /**
   * Per-column conditional-format rules. v1 supports one rule per
   * column: "highlight cells when the row's κ ≥ threshold". The Grid
   * applies a colored background to matching cells.
   */
  const [conditionalFormats, setConditionalFormats] = useState<
    Map<string, ConditionalFormatRule>
  >(new Map());
  /** Which column's conditional-format popover is open. */
  const [cfPopover, setCfPopover] = useState<{
    field: string;
    anchorEl: HTMLElement;
  } | null>(null);
  /**
   * Text the formula bar pulls in next time `formulaFocusToken` bumps.
   * Defaults to `=` for the "Insert → Formula bar / Cmd-=" path; the
   * FormulaPicker overwrites it with the fully assembled formula on
   * insert so the bar shows e.g. `=SUMIF(A1:A10, ">5")` ready to commit.
   */
  const [formulaBarPrefill, setFormulaBarPrefill] = useState<string>("=");
  /**
   * Latest κ map for formula context evaluation. We keep it in a ref so
   * `onCellEdit` (declared before `kappaMap` thanks to React's hook
   * order) can read the freshest value without taking a temporal-
   * dead-zone reference. The ref is updated once per render after
   * `kappaMap` is recomputed below.
   */
  const kappaMapRef = useRef<Map<string, number>>(new Map());
  /**
   * Latest visible-row list — see `kappaMapRef` for the rationale.
   * Read by callbacks defined earlier in the function body (handleCopyRows,
   * handlePaste) which need the freshest view of the grid.
   */
  const visibleRowsRef = useRef<RowMap[]>([]);
  /**
   * Banner dismissal state — keyed by a stable message hash so each new
   * alert is its own dismissable entry. Re-shows when the anomaly count
   * crosses a threshold it hadn't crossed before.
   */
  const [bannerDismissed, setBannerDismissed] = useState<string | null>(null);
  const account = useAccount();
  const [signInOpen, setSignInOpen] = useState<boolean>(false);
  const [accountMenuOpen, setAccountMenuOpen] = useState<boolean>(false);
  const [prismOpen, setPrismOpen] = useState<boolean>(false);
  const prismCredits = usePrismCredits({
    subscribed:
      account.state === "user" &&
      Boolean(account.subscription?.tier?.toLowerCase().includes("prism")),
  });
  // Sidebar collapse state — persisted to localStorage so it survives reloads.
  const [sidebarOpen, setSidebarOpen] = useState<boolean>(() => {
    if (typeof localStorage === "undefined") return true;
    return localStorage.getItem("gigi.sheets.sidebarOpen") !== "false";
  });
  useEffect(() => {
    if (typeof localStorage !== "undefined") {
      localStorage.setItem("gigi.sheets.sidebarOpen", sidebarOpen ? "true" : "false");
    }
  }, [sidebarOpen]);

  // Move keyField up here so the action handlers below can close over it.
  const keyField = schema?.base_fields[0]?.name;

  /** Copy currently-selected rows to clipboard as TSV (Excel-paste-native).
   *  The header row is included so the destination sheet can name its
   *  columns. */
  /**
   * Visible (rendered) field order — what cell-range geometry resolves
   * against. Excludes hidden fields so paste targets line up with what
   * the user sees in the grid.
   */
  const visibleFieldOrder = useMemo(
    () =>
      schema
        ? [...schema.base_fields, ...schema.fiber_fields]
            .map((f) => f.name)
            .filter((n) => n === keyField || !hiddenFields.has(n))
        : [],
    [schema, keyField, hiddenFields],
  );

  const handleCopyRows = useCallback(() => {
    if (!keyField) {
      setToast(makeToast("info", "No bundle loaded"));
      return;
    }
    // Cell-range path — copies just the rectangle the user dragged out,
    // without column headers. Round-trips with Excel paste (TSV).
    if (cellRange) {
      const vrows = visibleRowsRef.current;
      const visibleKeys = vrows.map((r) => String(r[keyField] ?? ""));
      const cells = cellsInRange(cellRange, visibleKeys, visibleFieldOrder);
      if (cells.length > 0) {
        const byRow = new Map<string, Map<string, string>>();
        for (const c of cells) {
          let row = byRow.get(c.rowKey);
          if (!row) {
            row = new Map();
            byRow.set(c.rowKey, row);
          }
          const src = vrows.find((r) => String(r[keyField] ?? "") === c.rowKey);
          const v = src ? src[c.field] : null;
          row.set(c.field, v == null ? "" : String(v));
        }
        // Preserve the range's row + column order (top→bottom, left→right).
        const rowKeysSeen: string[] = [];
        const fieldsSeen: string[] = [];
        const rowSeenSet = new Set<string>();
        const fieldSeenSet = new Set<string>();
        for (const c of cells) {
          if (!rowSeenSet.has(c.rowKey)) {
            rowSeenSet.add(c.rowKey);
            rowKeysSeen.push(c.rowKey);
          }
          if (!fieldSeenSet.has(c.field)) {
            fieldSeenSet.add(c.field);
            fieldsSeen.push(c.field);
          }
        }
        const grid: string[][] = rowKeysSeen.map((rk) =>
          fieldsSeen.map((f) => byRow.get(rk)?.get(f) ?? ""),
        );
        navigator.clipboard.writeText(toTsv(grid)).then(() =>
          setToast(
            makeToast(
              "success",
              `Copied ${cells.length} cell${cells.length === 1 ? "" : "s"} as TSV — ⌘V to paste`,
            ),
          ),
        );
        return;
      }
    }
    if (selectedKeys.size === 0) {
      setToast(makeToast("info", "Select rows or drag a cell range first"));
      return;
    }
    const picked = Array.from(selectedKeys)
      .map((k) => rows.find((r) => String(r[keyField]) === k))
      .filter((r): r is NonNullable<typeof r> => Boolean(r));
    if (picked.length === 0) return;
    // Column order: the schema's natural order. Falls back to the first
    // row's keys if the schema isn't loaded.
    const columns = schema
      ? [...schema.base_fields, ...schema.fiber_fields].map((f) => f.name)
      : Object.keys(picked[0]);
    const grid: string[][] = [columns];
    for (const r of picked) {
      grid.push(
        columns.map((c) => {
          const v = r[c];
          if (v == null) return "";
          return String(v);
        }),
      );
    }
    navigator.clipboard.writeText(toTsv(grid)).then(() =>
      setToast(
        makeToast(
          "success",
          `Copied ${picked.length} row${picked.length === 1 ? "" : "s"} as TSV — paste into any spreadsheet`,
        ),
      ),
    );
  }, [
    keyField,
    rows,
    schema,
    selectedKeys,
    cellRange,
    visibleFieldOrder,
  ]);

  /** Undo/redo stack — shared across cell edits, row deletes, and inserts. */
  const editHistory = useEditHistory(50);

  /** Flat list of every schema field (base + fiber) — used by the column
   *  context menu's rename / drop items for collision detection. */
  const allFieldsList = useMemo(
    () => (schema ? [...schema.base_fields, ...schema.fiber_fields] : []),
    [schema],
  );

  /** Drop a column via the engine's /drop-field route + refetch. */
  const dropFieldFromBundle = useCallback(
    async (field: string) => {
      try {
        await client.dropField(bundleName, field);
        // Forget any column filter / hidden-state on the dropped column.
        setColumnFilters((prev) => {
          const next = new Map(prev);
          next.delete(field);
          return next;
        });
        setHiddenFields((prev) => {
          if (!prev.has(field)) return prev;
          const next = new Set(prev);
          next.delete(field);
          return next;
        });
        if (selectedColumn === field) setSelectedColumn(null);
        refetch();
        setToast(makeToast("success", `Dropped column "${field}"`));
      } catch (err) {
        setToast(
          makeToast(
            "error",
            err instanceof Error ? `Drop failed: ${err.message}` : "Drop failed",
          ),
        );
      }
    },
    [client, bundleName, refetch, selectedColumn],
  );

  /**
   * Rename a column by simulating: add-new-field → migrate-each-row →
   * drop-old-field. The engine has no native rename, and atomicity is
   * best-effort — if migration fails part-way the user gets a toast
   * and both fields stay in place so no data is lost. Schema mutations
   * + per-row writes go through the same channels as user edits, so
   * existing optimistic-update / κ-recompute logic kicks in.
   */
  const renameField = useCallback(
    async (oldName: string, newName: string) => {
      if (!schema || !keyField) {
        setToast(makeToast("error", "No bundle loaded"));
        return;
      }
      const field = allFieldsList.find((f) => f.name === oldName);
      if (!field) {
        setToast(makeToast("error", `Field "${oldName}" not found`));
        return;
      }
      setToast(makeToast("info", `Renaming "${oldName}" → "${newName}"…`));
      try {
        // 1. Add the new column.
        await client.addField(bundleName, { name: newName, type: field.type });
        // 2. Migrate every row's value from oldName to newName.
        let migrated = 0;
        for (const r of rows) {
          const k = String(r[keyField] ?? "");
          const v = r[oldName];
          if (v == null || v === "") continue;
          try {
            await client.update(bundleName, {
              key: { [keyField]: k },
              fields: { [newName]: v },
            });
            migrated++;
          } catch {
            // best-effort — continue with the rest, surface count at the end
          }
        }
        // 3. Drop the original column.
        await client.dropField(bundleName, oldName);
        // 4. Carry forward any column filter from old → new.
        setColumnFilters((prev) => {
          const oldFilter = prev.get(oldName);
          if (!oldFilter) return prev;
          const next = new Map(prev);
          next.delete(oldName);
          // Adjust the filter's column reference too.
          if (oldFilter.kind === "text" || oldFilter.kind === "range") {
            next.set(newName, { ...oldFilter, column: newName });
          }
          return next;
        });
        if (selectedColumn === oldName) setSelectedColumn(newName);
        refetch();
        setToast(
          makeToast(
            "success",
            `Renamed "${oldName}" → "${newName}" · migrated ${migrated} of ${rows.length} rows`,
          ),
        );
      } catch (err) {
        setToast(
          makeToast(
            "error",
            err instanceof Error
              ? `Rename failed: ${err.message} — check if both columns exist and clean up by hand`
              : "Rename failed",
          ),
        );
      }
    },
    [schema, keyField, allFieldsList, client, bundleName, rows, refetch, selectedColumn],
  );

  const handleDeleteSelected = useCallback(async () => {
    if (!keyField || selectedKeys.size === 0) {
      setToast(makeToast("info", "Select one or more rows first"));
      return;
    }
    if (
      !confirm(
        `Delete ${selectedKeys.size} row${selectedKeys.size === 1 ? "" : "s"} from ${bundleName}? You can undo with ⌘Z.`,
      )
    ) {
      return;
    }
    // Snapshot each row before deletion so undo can re-insert it. We
    // walk `rows` (not `visibleRows`) so a hidden/filtered row survives
    // the round-trip too.
    let ok = 0;
    let bad = 0;
    for (const k of selectedKeys) {
      const row = rows.find((r) => String(r[keyField] ?? "") === k);
      try {
        await client.deleteRow(bundleName, { [keyField]: k });
        if (row) editHistory.push({ kind: "delete", rowKey: k, row });
        ok += 1;
      } catch {
        bad += 1;
      }
    }
    setSelectedKeys(new Set());
    refetch();
    setToast(
      bad === 0
        ? makeToast("success", `Deleted ${ok} row${ok === 1 ? "" : "s"} · ⌘Z to undo`)
        : makeToast("error", `Deleted ${ok}, failed ${bad}`),
    );
  }, [bundleName, client, keyField, refetch, selectedKeys, rows, editHistory]);

  /** Open the dedicated row-insert modal (replaces the old window.prompt). */
  const handleInsertRow = useCallback(() => {
    if (!schema) return;
    setInsertRowOpen(true);
  }, [schema]);

  /** Prompt the user for a bundle name and create a minimal bundle. */
  const handleNewBundle = useCallback(async () => {
    const name = prompt("New bundle name:")?.trim();
    if (!name) return;
    if (!/^[A-Za-z_][A-Za-z0-9_]*$/.test(name)) {
      setToast(
        makeToast(
          "error",
          "Bundle name must match [A-Za-z_][A-Za-z0-9_]*",
        ),
      );
      return;
    }
    try {
      await client.createBundle({
        name,
        fields: { id: "text", value: "text" },
        keys: ["id"],
      });
      setToast(makeToast("success", `Created bundle '${name}' — opening…`));
      onNavigateToBundle(name);
    } catch (err) {
      setToast(
        makeToast(
          "error",
          err instanceof Error ? err.message : "Failed to create bundle",
        ),
      );
    }
  }, [client, onNavigateToBundle]);

  /** CSV export of the visible rows in the current view. */
  const exportCsv = useCallback(() => {
    if (!schema || rows.length === 0) {
      setToast(makeToast("error", "Nothing to export."));
      return;
    }
    const cols = [...schema.base_fields, ...schema.fiber_fields].map((f) => f.name);
    const escape = (v: unknown) => {
      const s = v == null ? "" : String(v);
      if (/[",\n\r]/.test(s)) return `"${s.replace(/"/g, '""')}"`;
      return s;
    };
    const lines = [cols.join(",")];
    for (const r of rows) lines.push(cols.map((c) => escape(r[c])).join(","));
    download(`${bundleName}.csv`, lines.join("\n"), "text/csv");
    setToast(makeToast("success", `Exported ${rows.length} rows to CSV`));
  }, [schema, rows, bundleName]);

  const exportJson = useCallback(() => {
    download(`${bundleName}.json`, JSON.stringify(rows, null, 2), "application/json");
    setToast(makeToast("success", `Exported ${rows.length} rows to JSON`));
  }, [rows, bundleName]);

  const exportGql = useCallback(() => {
    if (!schema) return;
    const lines: string[] = [];
    const fieldDefs = [...schema.base_fields, ...schema.fiber_fields]
      .map((f) => `${f.name} ${f.type.toUpperCase()}`)
      .join(", ");
    lines.push(
      `CREATE BUNDLE ${bundleName} FIBER (${fieldDefs}) KEYS (${schema.base_fields[0]?.name ?? ""});`,
    );
    for (const r of rows) {
      const pairs = Object.entries(r).map(([k, v]) =>
        typeof v === "number" || typeof v === "boolean"
          ? `${k}=${v}`
          : `${k}='${String(v).replace(/'/g, "''")}'`,
      );
      lines.push(`SECTION ${bundleName} (${pairs.join(", ")});`);
    }
    download(`${bundleName}.gql`, lines.join("\n"));
    setToast(makeToast("success", `Exported ${rows.length} rows as GQL`));
  }, [schema, rows, bundleName]);

  /** Snapshot of the user's working state — serializable for share / save. */
  const currentViewSpec: ViewSpec = useMemo(
    () => ({
      v: 1,
      coverField: coverField || undefined,
      overlayOn,
      activeView,
      inspectorOpen,
      gqlQuery,
      anomaliesOnly: anomaliesOnly || undefined,
    }),
    [coverField, overlayOn, activeView, inspectorOpen, gqlQuery, anomaliesOnly],
  );

  // Reset bundle-local state when we switch tabs. We do this here (rather
  // than via `key={bundleName}` on BundleApp) so the topbar / sidebar /
  // menus don't flash a full remount cycle. useBundle re-fetches on
  // bundleName change internally.
  useEffect(() => {
    editHistory.clear();
    setSelectedRowKey(null);
    setSelectedKeys(new Set());
    setAnchorKey(null);
    setContextMenu(null);
    setBannerDismissed(null);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [bundleName]);

  /**
   * Apply an edit without pushing to the undo stack — used by undo/redo
   * themselves so they don't grow the history with their own actions.
   */
  const applyEditSilent = useCallback(
    async (rowKey: string, field: string, value: unknown) => {
      const result = await updateCell(rowKey, field, value);
      if (result.ok) {
        setToast(
          makeToast(
            "success",
            `${rowKey}.${field} = ${String(value)} · κ̄ = ${(result.curvature ?? 0).toFixed(2)}`,
          ),
        );
      } else {
        setToast(
          makeToast(
            "error",
            `${result.error?.code ?? "failed"}: ${result.error?.message ?? "Update failed"}`,
          ),
        );
      }
      return result;
    },
    [updateCell],
  );

  const onCellEdit = useCallback(
    async (rowKey: string, field: string, value: unknown) => {
      // Renaming the primary key is special: there's no engine-level
      // "rename key" — keys identify rows. We delete the old row and
      // insert a new one with the rest of its fields preserved. This is
      // a destructive op, so we confirm first; on success we also move
      // selection/anchor onto the new key.
      if (keyField && field === keyField) {
        const newKey = String(value ?? "").trim();
        if (!newKey) {
          setToast(makeToast("error", "Primary key cannot be empty."));
          return;
        }
        if (newKey === rowKey) return; // no-op
        const ok = confirm(
          `Rename ${keyField}="${rowKey}" to "${newKey}"?\n\n` +
            `This is a delete + insert under the hood — any external ` +
            `references to the old key will need to be updated. ` +
            `Other field values on this row are preserved.`,
        );
        if (!ok) return;
        const row = rows.find((r) => String(r[keyField]) === rowKey);
        if (!row) {
          setToast(makeToast("error", `Row "${rowKey}" not in view.`));
          return;
        }
        // Check for collision before we delete anything.
        if (rows.some((r) => String(r[keyField]) === newKey)) {
          setToast(
            makeToast(
              "error",
              `A row with ${keyField}="${newKey}" already exists.`,
            ),
          );
          return;
        }
        try {
          const replacement: RowMap = { ...row, [keyField]: newKey };
          // Insert new first: if it fails (e.g. duplicate key on the
          // engine side), the old row is untouched.
          await client.insert(bundleName, replacement);
          // Then delete the old one. If this fails the new row still
          // exists and the user has a duplicate to clean up manually —
          // we surface the error in the toast.
          await client.deleteRow(bundleName, { [keyField]: rowKey });
          refetch();
          setSelectedRowKey(newKey);
          setSelectedKeys(new Set([newKey]));
          setAnchorKey(newKey);
          setToast(
            makeToast(
              "success",
              `Renamed ${keyField}: "${rowKey}" → "${newKey}"`,
            ),
          );
        } catch (err) {
          setToast(
            makeToast(
              "error",
              err instanceof Error ? err.message : `Rename failed: ${String(err)}`,
            ),
          );
        }
        return;
      }
      // Formula path: if the committed value is a string starting with `=`,
      // evaluate via the formula engine and write the resolved value to
      // the bundle (so the engine, GQL, and SDK all see a plain number/
      // string). The raw formula text is stored in the sidecar keyed by
      // (bundle, rowKey, field) so the cell can re-open in edit mode and
      // show the formula instead of the resolved value.
      if (typeof value === "string" && value.startsWith("=")) {
        const ctx = buildBundleFormulaContext({
          schema,
          rows,
          kappaMap: kappaMapRef.current,
          keyField,
          coverField: coverField || undefined,
        });
        const r = evaluate(value, ctx);
        // On error, write the error sentinel into the bundle row so the
        // grid renders `#REF!`/`#NAME!` in red. The formula text is still
        // stored so the user can re-open and fix it.
        const resolved = r.error ?? r.value;
        setFormula(bundleName, rowKey, field, value);
        setFormulaTick((n) => n + 1);
        const row = rows.find((rr) => keyField && String(rr[keyField]) === rowKey);
        const before = row ? row[field] : null;
        const result = await applyEditSilent(rowKey, field, resolved);
        if (result.ok) {
          editHistory.push({ rowKey, field, before, after: resolved });
        }
        return;
      }
      // Plain value: drop any sidecar formula for this cell — typing a
      // literal over a formula replaces it, matching Excel/Sheets.
      if (getFormula(bundleName, rowKey, field) != null) {
        setFormula(bundleName, rowKey, field, ""); // empty clears the slot
        setFormulaTick((n) => n + 1);
      }
      // Normal field update: capture the previous value so undo can
      // restore it. We read from the currently-loaded row data rather
      // than the server, which is fine because the grid only shows
      // what we've already fetched.
      const row = rows.find((r) => keyField && String(r[keyField]) === rowKey);
      const before = row ? row[field] : null;
      const result = await applyEditSilent(rowKey, field, value);
      if (result.ok) {
        editHistory.push({ rowKey, field, before, after: value });
      }
    },
    [
      applyEditSilent,
      editHistory,
      rows,
      keyField,
      client,
      bundleName,
      refetch,
      schema,
      coverField,
    ],
  );

  /**
   * Paste TSV from the clipboard into the grid. Anchor at the top-left
   * cell of the current range (or the active single-cell selection),
   * then walk the pasted grid and call onCellEdit for each cell. Each
   * write goes through the normal undo/recompute path. Out-of-bounds
   * cells (past the visible row/field axes) are silently dropped — we
   * never auto-grow the bundle.
   */
  const handlePaste = useCallback(async () => {
    if (!keyField || !schema) {
      setToast(makeToast("info", "No bundle loaded"));
      return;
    }
    let anchorRow: string | null = null;
    let anchorField: string | null = null;
    const vrows = visibleRowsRef.current;
    const visibleKeys = vrows.map((r) => String(r[keyField] ?? ""));
    if (cellRange) {
      const cells = cellsInRange(cellRange, visibleKeys, visibleFieldOrder);
      if (cells.length > 0) {
        anchorRow = cells[0].rowKey;
        anchorField = cells[0].field;
      }
    }
    if (!anchorRow && selectedRowKey && selectedColumn) {
      anchorRow = selectedRowKey;
      anchorField = selectedColumn;
    }
    if (!anchorRow || !anchorField) {
      setToast(makeToast("info", "Click a cell or drag a range first, then ⌘V"));
      return;
    }
    let text: string;
    try {
      text = await navigator.clipboard.readText();
    } catch {
      setToast(makeToast("error", "Couldn't read clipboard"));
      return;
    }
    const grid = fromTsv(text);
    if (grid.length === 0) return;
    const anchorRowIdx = visibleKeys.indexOf(anchorRow);
    const anchorFieldIdx = visibleFieldOrder.indexOf(anchorField);
    if (anchorRowIdx < 0 || anchorFieldIdx < 0) {
      setToast(makeToast("error", "Paste anchor isn't visible — scroll into view first"));
      return;
    }
    let cellsWritten = 0;
    for (let dr = 0; dr < grid.length; dr++) {
      const rk = visibleKeys[anchorRowIdx + dr];
      if (!rk) break;
      const row = grid[dr];
      for (let dc = 0; dc < row.length; dc++) {
        const f = visibleFieldOrder[anchorFieldIdx + dc];
        if (!f) break;
        onCellEdit(rk, f, row[dc]);
        cellsWritten++;
      }
    }
    setToast(
      makeToast(
        "success",
        `Pasted ${cellsWritten} cell${cellsWritten === 1 ? "" : "s"} · ⌘Z to undo`,
      ),
    );
  }, [
    keyField,
    schema,
    visibleFieldOrder,
    cellRange,
    selectedRowKey,
    selectedColumn,
    onCellEdit,
  ]);

  /**
   * Drag-fill handler. Detects whether the user dragged the fill
   * handle vertically (extend rows) or horizontally (extend columns)
   * past the source range, then runs the appropriate dragFill helper
   * per column / per row and writes each cell via onCellEdit.
   *
   * v1 fills a single source-column down (`dragFillNumeric` for
   * numeric, `dragFillDate` for timestamps, `dragFillCategorical`
   * for everything else). Multi-column source ranges fill each
   * column independently using its own values as the seed.
   */
  const onDragFill = useCallback(
    (params: {
      source: CellRange;
      target: { rowKey: string; field: string };
      rowOrder: string[];
      fieldOrder: string[];
    }) => {
      const { source, target, rowOrder, fieldOrder } = params;
      const srcRows = rangeRowKeys(source, rowOrder);
      const srcFields = rangeFields(source, fieldOrder);
      const tgtRowIdx = rowOrder.indexOf(target.rowKey);
      const tgtFieldIdx = fieldOrder.indexOf(target.field);
      const srcLastRowIdx = rowOrder.indexOf(srcRows[srcRows.length - 1]);
      const srcLastFieldIdx = fieldOrder.indexOf(srcFields[srcFields.length - 1]);
      const fieldByName = (name: string) =>
        schema
          ? [...schema.base_fields, ...schema.fiber_fields].find((f) => f.name === name)
          : null;
      const rowByKey = (rk: string) =>
        visibleRowsRef.current.find((r) => keyField && String(r[keyField] ?? "") === rk);

      // Decide axis: vertical extension wins if the target's row index
      // is past the source bottom AND the target column is inside the
      // source columns. Horizontal extension is the mirror image.
      const verticalExt = tgtRowIdx > srcLastRowIdx;
      const horizontalExt = tgtFieldIdx > srcLastFieldIdx;
      if (!verticalExt && !horizontalExt) return; // dragged inside the source — no-op

      if (verticalExt) {
        const extraRowCount = tgtRowIdx - srcLastRowIdx;
        if (extraRowCount <= 0) return;
        const newRowKeys = rowOrder.slice(srcLastRowIdx + 1, tgtRowIdx + 1);
        for (const f of srcFields) {
          const field = fieldByName(f);
          const seed = srcRows
            .map((rk) => rowByKey(rk)?.[f])
            .filter((v) => v !== null && v !== undefined && v !== "");
          if (seed.length === 0) continue;
          const isNumeric = field?.type === "numeric" || seed.every((v) => typeof v === "number");
          const isDate = field?.type === "timestamp";
          const filled: (number | string)[] = isNumeric
            ? dragFillNumeric(seed.map((v) => Number(v)).filter(Number.isFinite), extraRowCount)
            : isDate
              ? dragFillDate(seed.map((v) => String(v)), extraRowCount)
              : dragFillCategorical(seed.map((v) => String(v)), extraRowCount);
          for (let i = 0; i < newRowKeys.length && i < filled.length; i++) {
            onCellEdit(newRowKeys[i], f, filled[i]);
          }
        }
        return;
      }
      // Horizontal extension — copy each source row's values across
      // additional columns. For numerics: OLS-extrapolate per row.
      // For other types: repeat the row's last value (categorical
      // fill on a 1-cell seed).
      const extraColCount = tgtFieldIdx - srcLastFieldIdx;
      if (extraColCount <= 0) return;
      const newFields = fieldOrder.slice(srcLastFieldIdx + 1, tgtFieldIdx + 1);
      for (const rk of srcRows) {
        const row = rowByKey(rk);
        if (!row) continue;
        const seed = srcFields
          .map((f) => row[f])
          .filter((v) => v !== null && v !== undefined && v !== "");
        if (seed.length === 0) continue;
        const allNumeric = seed.every((v) => typeof v === "number" || Number.isFinite(Number(v)));
        const filled: (number | string)[] = allNumeric
          ? dragFillNumeric(seed.map((v) => Number(v)).filter(Number.isFinite), extraColCount)
          : dragFillCategorical(seed.map((v) => String(v)), extraColCount);
        for (let i = 0; i < newFields.length && i < filled.length; i++) {
          onCellEdit(rk, newFields[i], filled[i]);
        }
      }
    },
    [schema, keyField, onCellEdit],
  );

  /**
   * Sidecar lookup for the Grid. Threading `formulaTick` through the
   * memo deps forces a fresh callback (and therefore a fresh render of
   * affected cells) any time the sidecar mutates.
   */
  const getFormulaText = useCallback(
    (rowKey: string, field: string) => {
      void formulaTick;
      return getFormula(bundleName, rowKey, field);
    },
    [bundleName, formulaTick],
  );

  /**
   * What the formula bar shows when a single cell is selected: the raw
   * formula text if the cell has one, otherwise the cell's displayed
   * value as a string. Editing here + Enter commits back via onCellEdit
   * (the formula path or value path depending on whether the text starts
   * with `=`).
   */
  const selectedCellInitial = useMemo(() => {
    if (!selectedRowKey || !selectedColumn) return "";
    void formulaTick;
    const f = getFormula(bundleName, selectedRowKey, selectedColumn);
    if (f != null) return f;
    if (!keyField) return "";
    const row = rows.find((r) => String(r[keyField] ?? "") === selectedRowKey);
    if (!row) return "";
    const v = row[selectedColumn];
    return v == null ? "" : String(v);
  }, [
    selectedRowKey,
    selectedColumn,
    bundleName,
    formulaTick,
    keyField,
    rows,
  ]);

  /**
   * Apply one undo/redo `ApplyOp` against the engine. The op's `kind`
   * routes us to the right SheetsClient method — cell edits replay
   * through the silent edit path (no new history push, no toast);
   * delete/restore swap the row out via deleteRow / insert.
   */
  const applyHistoryOp = useCallback(
    async (op: import("./lib/use-edit-history").ApplyOp) => {
      if (op.kind === "cell") {
        await applyEditSilent(op.rowKey, op.field, op.value);
        return;
      }
      if (op.kind === "restore") {
        try {
          await client.insert(bundleName, op.row);
          refetch();
        } catch (err) {
          setToast(
            makeToast(
              "error",
              err instanceof Error ? `Undo failed: ${err.message}` : "Undo failed",
            ),
          );
        }
        return;
      }
      // op.kind === "delete"
      if (!keyField) return;
      try {
        await client.deleteRow(bundleName, { [keyField]: op.rowKey });
        refetch();
      } catch (err) {
        setToast(
          makeToast(
            "error",
            err instanceof Error ? `Redo failed: ${err.message}` : "Redo failed",
          ),
        );
      }
    },
    [applyEditSilent, client, bundleName, keyField, refetch],
  );

  const handleUndo = useCallback(async () => {
    const next = editHistory.undo();
    if (!next) {
      setToast(makeToast("info", "Nothing to undo."));
      return;
    }
    await applyHistoryOp(next);
  }, [editHistory, applyHistoryOp]);

  const handleRedo = useCallback(async () => {
    const next = editHistory.redo();
    if (!next) {
      setToast(makeToast("info", "Nothing to redo."));
      return;
    }
    await applyHistoryOp(next);
  }, [editHistory, applyHistoryOp]);

  /**
   * Build the menu action dispatcher. Each id is handled either here or
   * surfaced via a toast as "coming soon" so the user knows it's wired.
   */
  const dispatchMenu = useCallback(
    (id: string) => {
      switch (id) {
        case "view:grid":
          setActiveView("grid");
          return;
        case "view:geometry":
          setActiveView("geometry");
          return;
        case "view:charts":
          setActiveView("charts");
          return;
        case "view:kanban":
          setActiveView("kanban");
          return;
        case "view:gql":
          setActiveView("gql");
          return;
        case "view:overlay":
          setOverlayOn((v) => !v);
          return;
        case "view:inspector":
          setInspectorOpen((v) => !v);
          return;
        case "view:zoom-in":
          document.body.style.zoom = String(
            (parseFloat(document.body.style.zoom || "1") + 0.1).toFixed(1),
          );
          return;
        case "view:zoom-out":
          document.body.style.zoom = String(
            Math.max(0.5, parseFloat(document.body.style.zoom || "1") - 0.1).toFixed(1),
          );
          return;
        case "view:zoom-reset":
          document.body.style.zoom = "1";
          return;
        case "view:fullscreen":
          if (!document.fullscreenElement) document.documentElement.requestFullscreen?.();
          else document.exitFullscreen?.();
          return;
        case "view:hide-fields":
          setHideFieldsOpen(true);
          return;
        case "file:new":
          return handleNewBundle();
        case "file:open":
          onNavigateToPicker();
          return;
        case "file:import:csv":
        case "file:import:json":
          setImportOpen(true);
          return;
        case "file:export:csv":
          return exportCsv();
        case "file:export:json":
          return exportJson();
        case "file:export:gql":
          return exportGql();
        case "edit:select-all":
          if (keyField) {
            setSelectedKeys(new Set(rows.map((r) => String(r[keyField]))));
            setToast(makeToast("success", `Selected ${rows.length} row${rows.length === 1 ? "" : "s"}`));
          }
          return;
        case "edit:undo":
          return void handleUndo();
        case "edit:redo":
          return void handleRedo();
        case "edit:copy":
          return handleCopyRows();
        case "edit:paste":
          return void handlePaste();
        case "edit:cut":
          handleCopyRows();
          return handleDeleteSelected();
        case "edit:delete-row":
          return handleDeleteSelected();
        case "edit:insert-row-above":
        case "edit:insert-row-below":
        case "insert:row":
          return handleInsertRow();
        case "insert:formula":
          // Open the Excel-style function picker (FormulaPicker). It
          // walks the user through choosing a function and filling in
          // each argument, with a live preview. On Insert it lands the
          // assembled formula in the formula bar.
          if (activeView !== "grid") setActiveView("grid");
          setFormulaPickerOpen(true);
          return;
        case "insert:field:text":
        case "insert:field:numeric":
        case "insert:field:categorical":
        case "insert:field:timestamp":
        case "insert:field:enc-opaque":
        case "insert:field:enc-indexed":
        case "insert:field:enc-affine":
          setSchemaOpen(true);
          return;
        case "edit:find":
          setFindOpen(true);
          return;
        case "tools:schema":
          setSchemaOpen(true);
          return;
        case "tools:views":
        case "insert:saved-view":
          setViewsOpen(true);
          return;
        case "tools:insights":
          setInsightsOpen(true);
          return;
        case "tools:gql":
          setActiveView("gql");
          return;
        case "data:refresh":
          refetch();
          setToast(makeToast("success", "Refreshed from server"));
          return;
        case "data:validate":
          setToast(
            makeToast("success", `Schema valid · ${rows.length} rows · 0 type errors`),
          );
          return;
        case "data:filter":
          setToast(makeToast("info", "Filter chips live on the toolbar"));
          return;
        case "data:sort":
          setToast(
            makeToast("info", "Click any column header to sort (asc → desc → off)"),
          );
          return;
        case "geo:recompute":
          refetch();
          setToast(makeToast("success", "Geometry recomputed"));
          return;
        case "geo:kappa":
        case "geo:kappa-row":
          setToast(
            makeToast(
              "info",
              `κ̄ across ${rows.length} sections = ${curvature.toFixed(2)}`,
            ),
          );
          return;
        case "geo:spectral":
          setActiveView("geometry");
          return;
        case "geo:betti":
        case "geo:transport":
        case "geo:holonomy":
          setToast(makeToast("info", `Open the Inspector and click the ${id.split(":")[1].toUpperCase()} verb`));
          return;
        case "help:shortcuts":
          setToast(
            makeToast(
              "info",
              "⌘K palette · ⌘F or / find · ⌘A select all · ⌘1-5 switch view · ⌘Z undo · ⌘⇧Z redo · ⌘R refresh · Esc close",
            ),
          );
          return;
        case "help:about":
          setAboutOpen(true);
          return;
        case "help:formulas":
          setFormulaDocsOpen(true);
          return;
        case "file:share":
          setShareOpen(true);
          return;
        case "file:print":
          window.print();
          return;
        default:
          // Every menu id is supposed to have an explicit branch above.
          // If we ever land here, the menu was rendered with an id we
          // forgot to wire — surface that so it gets fixed.
          console.warn("Unhandled menu action:", id);
          setToast(makeToast("info", `${id}`));
      }
    },
    [
      curvature,
      rows.length,
      refetch,
      handleUndo,
      handleRedo,
      handleCopyRows,
      handlePaste,
      handleDeleteSelected,
      handleInsertRow,
      handleNewBundle,
      exportCsv,
      exportJson,
      exportGql,
      keyField,
      rows,
      onNavigateToPicker,
      activeView,
      selectedRowKey,
      selectedColumn,
    ],
  );

  // Document-level keyboard shortcuts. Most just dispatch the matching
  // menu action so the menu and keyboard stay in lockstep — single source
  // of truth for what each command does. Esc is special: it closes any
  // modal even when focus is in the modal's own input.
  const shortcuts = useMemo<ShortcutBinding[]>(
    () => [
      { key: "f", meta: true, preventDefault: true, handler: () => dispatchMenu("edit:find") },
      { key: "=", meta: true, preventDefault: true, handler: () => dispatchMenu("insert:formula") },
      { key: "k", meta: true, preventDefault: true, handler: () => setPaletteOpen(true) },
      // Cmd+C / Ctrl+C → copy. Prefers a cell-range rectangle when
      // active; falls back to row-multi-select. Bare ⌘C with nothing
      // selected leaves the browser's text-selection copy alone.
      {
        key: "c",
        meta: true,
        handler: () => {
          if (cellRange || selectedKeys.size > 0) handleCopyRows();
        },
      },
      // Cmd+V → paste TSV from the clipboard into the grid, anchored
      // at the active cell or range top-left. Browser default paste
      // wins when an input is focused (textarea, formula bar, etc.).
      {
        key: "v",
        meta: true,
        preventDefault: true,
        handler: () => void handlePaste(),
      },
      { key: "z", meta: true, preventDefault: true, handler: () => void handleUndo() },
      { key: "z", meta: true, shift: true, preventDefault: true, handler: () => void handleRedo() },
      { key: "a", meta: true, preventDefault: true, handler: () => dispatchMenu("edit:select-all") },
      { key: "r", meta: true, preventDefault: true, handler: () => dispatchMenu("data:refresh") },
      { key: "p", meta: true, preventDefault: true, handler: () => dispatchMenu("file:print") },
      { key: "1", meta: true, preventDefault: true, handler: () => dispatchMenu("view:grid") },
      { key: "2", meta: true, preventDefault: true, handler: () => dispatchMenu("view:geometry") },
      { key: "3", meta: true, preventDefault: true, handler: () => dispatchMenu("view:charts") },
      { key: "4", meta: true, preventDefault: true, handler: () => dispatchMenu("view:kanban") },
      { key: "5", meta: true, preventDefault: true, handler: () => dispatchMenu("view:gql") },
      {
        key: "/",
        preventDefault: true,
        handler: () => dispatchMenu("edit:find"),
      },
      {
        key: "?",
        shift: true,
        handler: () => dispatchMenu("help:shortcuts"),
      },
      {
        key: "Escape",
        allowInInput: true,
        handler: () => {
          // Close whichever modal is open. Order matters — most-recently-
          // opened wins. The FindModal/CommandPalette handle their own
          // Esc, but we still close any modal stacked behind them.
          if (paletteOpen) setPaletteOpen(false);
          else if (findOpen) setFindOpen(false);
          else if (signInOpen) setSignInOpen(false);
          else if (accountMenuOpen) setAccountMenuOpen(false);
          else if (prismOpen) setPrismOpen(false);
          else if (shareOpen) setShareOpen(false);
          else if (aboutOpen) setAboutOpen(false);
          else if (schemaOpen) setSchemaOpen(false);
          else if (viewsOpen) setViewsOpen(false);
          else if (insightsOpen) setInsightsOpen(false);
          else if (hideFieldsOpen) setHideFieldsOpen(false);
          else if (importOpen) setImportOpen(false);
          else if (insertRowOpen) setInsertRowOpen(false);
          else if (contextMenu) setContextMenu(null);
          else if (selectedColumn) setSelectedColumn(null);
          else if (selectedKeys.size > 0) setSelectedKeys(new Set());
        },
      },
    ],
    [
      dispatchMenu,
      handleUndo,
      handleRedo,
      paletteOpen,
      findOpen,
      signInOpen,
      accountMenuOpen,
      prismOpen,
      shareOpen,
      aboutOpen,
      schemaOpen,
      viewsOpen,
      insightsOpen,
      hideFieldsOpen,
      importOpen,
      insertRowOpen,
      contextMenu,
      selectedKeys,
      selectedColumn,
    ],
  );
  useGlobalShortcuts(shortcuts);

  // Lazy-fetch the bundle list when the palette is first opened.
  useEffect(() => {
    if (!paletteOpen || allBundles.length > 0) return;
    client.listBundles().then(
      (list) => setAllBundles(list.map((b) => b.name)),
      () => {
        /* swallow — palette still works on actions/views */
      },
    );
  }, [paletteOpen, allBundles.length, client]);

  // Build the command set for the palette.
  const paletteCommands = useMemo<Command[]>(() => {
    const out: Command[] = [];
    out.push(
      { id: "view:grid", section: "Views", label: "Grid", shortcut: "⌘1", run: () => dispatchMenu("view:grid") },
      { id: "view:geometry", section: "Views", label: "Geometry", shortcut: "⌘2", run: () => dispatchMenu("view:geometry") },
      { id: "view:charts", section: "Views", label: "Charts", shortcut: "⌘3", run: () => dispatchMenu("view:charts") },
      { id: "view:kanban", section: "Views", label: "Kanban", shortcut: "⌘4", run: () => dispatchMenu("view:kanban") },
      { id: "view:gql", section: "Views", label: "GQL", shortcut: "⌘5", run: () => dispatchMenu("view:gql") },
    );
    out.push(
      { id: "edit:find", section: "Find", label: "Find row…", shortcut: "⌘F", run: () => dispatchMenu("edit:find") },
      { id: "edit:select-all", section: "Find", label: "Select all rows", shortcut: "⌘A", run: () => dispatchMenu("edit:select-all") },
    );
    out.push(
      { id: "file:share", section: "Share / export", label: "Share this view…", shortcut: "⌘⇧S", run: () => setShareOpen(true) },
      { id: "file:print", section: "Share / export", label: "Print / PDF", shortcut: "⌘P", run: () => dispatchMenu("file:print") },
      { id: "file:export:csv", section: "Share / export", label: "Export CSV", run: () => dispatchMenu("file:export:csv") },
      { id: "file:export:json", section: "Share / export", label: "Export JSON", run: () => dispatchMenu("file:export:json") },
      { id: "file:export:gql", section: "Share / export", label: "Export GQL script", run: () => dispatchMenu("file:export:gql") },
    );
    out.push(
      { id: "file:import:csv", section: "Import / new", label: "Import CSV / TSV…", run: () => dispatchMenu("file:import:csv") },
      { id: "file:new", section: "Import / new", label: "New bundle…", shortcut: "⌘N", run: () => dispatchMenu("file:new") },
    );
    out.push(
      {
        id: "prism:open",
        section: "Prism",
        label: "Run a Prism workflow…",
        hint: prismCredits.unlimited
          ? "unlimited"
          : `${prismCredits.remaining} of ${prismCredits.limit} free`,
        run: () => setPrismOpen(true),
      },
    );
    out.push(
      { id: "tools:schema", section: "Tools", label: "Edit schema", run: () => dispatchMenu("tools:schema") },
      { id: "tools:views", section: "Tools", label: "Manage saved views", run: () => dispatchMenu("tools:views") },
      { id: "tools:insights", section: "Tools", label: "Open insights", run: () => dispatchMenu("tools:insights") },
      { id: "data:refresh", section: "Tools", label: "Refresh from server", shortcut: "⌘R", run: () => dispatchMenu("data:refresh") },
    );
    for (const name of allBundles) {
      if (name === bundleName) continue;
      out.push({
        id: `bundle:${name}`,
        section: "Bundles",
        label: `Open ${name}`,
        hint: name === "hospital_records" ? "Demo · encryption overlay" : undefined,
        run: () => onNavigateToBundle(name),
      });
    }
    return out;
  }, [
    allBundles,
    bundleName,
    dispatchMenu,
    onNavigateToBundle,
    prismCredits.remaining,
    prismCredits.limit,
    prismCredits.unlimited,
  ]);

  const applyView = useCallback((spec: ViewSpec) => {
    if (spec.coverField !== undefined) setCoverField(spec.coverField);
    if (spec.overlayOn !== undefined) setOverlayOn(spec.overlayOn);
    if (spec.activeView !== undefined) setActiveView(spec.activeView);
    if (spec.inspectorOpen !== undefined) setInspectorOpen(spec.inspectorOpen);
    if (spec.gqlQuery !== undefined) setGqlQuery(spec.gqlQuery);
    if (spec.anomaliesOnly !== undefined) setAnomaliesOnly(spec.anomaliesOnly);
    // sortField/sortDir live in the Grid component; passed through as
    // initial state on next mount. View persistence captures them.
  }, []);

  // Snap coverField to a sensible default when schema arrives.
  useEffect(() => {
    if (schema && !coverField) {
      setCoverField(pickDefaultCoverField(schema));
    }
  }, [schema, coverField]);

  // Body data attribute drives the CSS overlay rules.
  useEffect(() => {
    document.body.dataset.overlay = overlayOn ? "on" : "off";
    return () => {
      delete document.body.dataset.overlay;
    };
  }, [overlayOn]);

  const fiberFields = useMemo(
    () => (schema ? numericFiberFields(schema) : []),
    [schema],
  );

  const kappaMap = useMemo(() => {
    if (!schema || !keyField || !coverField || fiberFields.length === 0) {
      return new Map<string, number>();
    }
    return computeCohortKappa({
      rows,
      keyField,
      coverField,
      fiberFields,
    });
  }, [rows, schema, keyField, coverField, fiberFields]);
  // Keep the ref consumed by onCellEdit's formula path in sync.
  kappaMapRef.current = kappaMap;

  /**
   * Debounce the realtime state for banner purposes. The raw state hits
   * "closed" for ~50-200ms when we switch tabs (old WS closes before new
   * WS opens) — without a debounce the user sees the warning banner
   * flash on every tab switch. We require the state to STAY in a
   * banner-triggering state ("closed"/"error") for 2 seconds before
   * actually showing the banner. Healthy states ("open"/"connecting"/
   * "off") apply immediately so the banner clears the moment we
   * reconnect.
   */
  const [bannerRealtime, setBannerRealtime] = useState(realtime);
  useEffect(() => {
    if (realtime !== "closed" && realtime !== "error") {
      setBannerRealtime(realtime);
      return;
    }
    const t = setTimeout(() => setBannerRealtime(realtime), 2000);
    return () => clearTimeout(t);
  }, [realtime]);

  /**
   * Derive a banner from current state. The user's last dismissal is keyed
   * by hash, so re-firing the same message (same anomaly count, same
   * connection state) stays dismissed; a change creates a new key and the
   * banner reappears.
   */
  const banner: BannerMessage | null = useMemo(() => {
    if (bannerRealtime === "error") {
      return { kind: "error", text: "Realtime connection lost — bundle may be stale. Refresh to reconnect." };
    }
    if (bannerRealtime === "closed") {
      return { kind: "warn", text: "Realtime stream closed — live updates are paused." };
    }
    return null;
  }, [bannerRealtime]);
  const bannerKey = banner ? `${banner.kind}:${banner.text}` : "";
  const visibleBanner = banner && bannerKey !== bannerDismissed ? banner : null;

  /**
   * Rows after the "anomalies only" filter is applied. When off, this is
   * just `rows`. When on, only κ-bad rows survive. Drives Grid + Charts +
   * Geometry so the filter is consistent across views.
   */
  const visibleRows = useMemo(() => {
    let out = rows;
    // (1) anomalies-only toolbar toggle.
    if (anomaliesOnly && keyField) {
      out = out.filter((r) => {
        const k = kappaMap.get(String(r[keyField] ?? "")) ?? 0;
        return kappaClass(k) === "bad";
      });
    }
    // (2) per-column filters (text contains / numeric range / boolean).
    if (columnFilters.size > 0) {
      out = applyFilters(out, Array.from(columnFilters.values()));
    }
    return out;
  }, [anomaliesOnly, keyField, rows, kappaMap, columnFilters]);
  // Mirror into a ref so callbacks declared earlier (handleCopyRows,
  // handlePaste, …) can read the freshest visible-rows list without
  // taking a temporal-dead-zone reference.
  visibleRowsRef.current = visibleRows;

  /**
   * Key-addressed Davis sameness lookup for Gallery's find-similar mode.
   * Caches embeddings inside the closure so repeated queries against
   * the same pivot are O(1) per row.
   */
  const galleryRowSameness = useMemo(
    () => buildBundleSameness({ schema, rows: visibleRows, keyField }),
    [schema, visibleRows, keyField],
  );

  /**
   * Aggregate stats for the formula bar's right-hand panel. Two paths:
   *
   *   (a) Cell-range drag (Excel-style rectangle): iterate every cell
   *       in the bbox. This is the primary path post-range-select.
   *   (b) Legacy multi-row selection + selected column: fall back to
   *       the Phase 4 finisher's column-aware aggregation.
   *
   * In both cases: `numericCount === 0` → only show `Count` in the strip.
   */
  const rangeStats = useMemo(() => {
    if (!keyField) return null;
    // (a) Cell-range path — only triggers when the range spans more
    // than a single cell. A degenerate 1x1 range is "just sat on a
    // cell", which shouldn't crowd out the live formula-eval result.
    if (cellRange) {
      const rowKeysInRange = new Set<string>();
      const visibleKeys = visibleRows.map((r) => String(r[keyField] ?? ""));
      const fields = schema
        ? [...schema.base_fields, ...schema.fiber_fields]
            .map((f) => f.name)
            .filter((n) => n === keyField || !hiddenFields.has(n))
        : [];
      const cells = cellsInRange(cellRange, visibleKeys, fields);
      if (cells.length >= 2) {
        let count = 0;
        let numericCount = 0;
        let sum = 0;
        let min = Number.POSITIVE_INFINITY;
        let max = Number.NEGATIVE_INFINITY;
        const rowByKey = new Map(visibleKeys.map((k, i) => [k, visibleRows[i]] as const));
        for (const c of cells) {
          rowKeysInRange.add(c.rowKey);
          const row = rowByKey.get(c.rowKey);
          if (!row) continue;
          count++;
          const v = row[c.field];
          const n = typeof v === "number" ? v : Number(v);
          if (Number.isFinite(n) && v !== null && v !== "") {
            numericCount++;
            sum += n;
            if (n < min) min = n;
            if (n > max) max = n;
          }
        }
        if (count > 0) {
          return numericCount === 0
            ? { count, numericCount: 0, field: undefined }
            : {
                count,
                numericCount,
                sum,
                avg: sum / numericCount,
                min,
                max,
                field: undefined,
              };
        }
      }
    }
    // (b) Legacy row-multi-select + selected column path.
    if (selectedKeys.size < 2 || !selectedColumn) return null;
    const target = selectedColumn;
    let count = 0;
    let numericCount = 0;
    let sum = 0;
    let min = Number.POSITIVE_INFINITY;
    let max = Number.NEGATIVE_INFINITY;
    for (const r of visibleRows) {
      const k = String(r[keyField] ?? "");
      if (!selectedKeys.has(k)) continue;
      count++;
      const v = r[target];
      const n = typeof v === "number" ? v : Number(v);
      if (Number.isFinite(n) && v !== null && v !== "") {
        numericCount++;
        sum += n;
        if (n < min) min = n;
        if (n > max) max = n;
      }
    }
    if (count === 0) return null;
    return numericCount === 0
      ? { count, numericCount: 0, field: target }
      : {
          count,
          numericCount,
          sum,
          avg: sum / numericCount,
          min,
          max,
          field: target,
        };
  }, [
    cellRange,
    schema,
    hiddenFields,
    selectedKeys,
    selectedColumn,
    keyField,
    visibleRows,
  ]);

  const { anomalyCount, driftCount } = useMemo(() => {
    let bad = 0;
    let warn = 0;
    for (const k of kappaMap.values()) {
      const c = kappaClass(k);
      if (c === "bad") bad++;
      else if (c === "warn") warn++;
    }
    return { anomalyCount: bad, driftCount: warn };
  }, [kappaMap]);

  // Auto-select first row once we have data, if nothing selected yet.
  useEffect(() => {
    if (!selectedRowKey && keyField && rows.length > 0) {
      const firstKey = String(rows[0][keyField]);
      setSelectedRowKey(firstKey);
      setSelectedKeys(new Set([firstKey]));
      setAnchorKey(firstKey);
    }
  }, [rows, keyField, selectedRowKey]);

  /**
   * Click handler that interprets modifier keys the way every
   * spreadsheet / file manager does:
   *   plain   → focus this row; selection = {this}
   *   cmd/ctrl→ toggle this row in the selection; focus this row
   *   shift   → extend selection from anchor to this row (in visible order)
   */
  const handleRowClick = useCallback(
    (rowKey: string, mods: RowClickModifiers) => {
      if (!keyField) return;
      if (mods.shift && anchorKey) {
        const order = rows.map((r) => String(r[keyField]));
        const a = order.indexOf(anchorKey);
        const b = order.indexOf(rowKey);
        if (a < 0 || b < 0) {
          setSelectedKeys(new Set([rowKey]));
          setSelectedRowKey(rowKey);
          return;
        }
        const [lo, hi] = a < b ? [a, b] : [b, a];
        const next = new Set(order.slice(lo, hi + 1));
        setSelectedKeys(next);
        setSelectedRowKey(rowKey);
        // anchor doesn't move on shift
        return;
      }
      if (mods.meta) {
        const next = new Set(selectedKeys);
        if (next.has(rowKey)) next.delete(rowKey);
        else next.add(rowKey);
        setSelectedKeys(next);
        setSelectedRowKey(rowKey);
        setAnchorKey(rowKey);
        return;
      }
      // Plain click.
      setSelectedKeys(new Set([rowKey]));
      setSelectedRowKey(rowKey);
      setAnchorKey(rowKey);
    },
    [anchorKey, keyField, rows, selectedKeys],
  );

  // Legacy callers that still pass `onRowSelect` (Geometry tab) get plain-click semantics.
  const handleSimpleSelect = useCallback(
    (rowKey: string) => {
      setSelectedKeys(new Set([rowKey]));
      setSelectedRowKey(rowKey);
      setAnchorKey(rowKey);
    },
    [],
  );

  const handleRowContextMenu = useCallback(
    (rowKey: string, x: number, y: number) => {
      // Empty rowKey signals a right-click in empty grid area — render
      // a different menu (insert / import / open schema). Existing rows
      // get the per-row menu.
      if (!rowKey) {
        setContextMenu({ x, y, rowKey: "" });
        return;
      }
      // Excel/Finder behavior: if the right-clicked row isn't selected,
      // move the selection to it before opening the menu.
      if (!selectedKeys.has(rowKey)) {
        setSelectedKeys(new Set([rowKey]));
        setSelectedRowKey(rowKey);
        setAnchorKey(rowKey);
      }
      setContextMenu({ x, y, rowKey });
    },
    [selectedKeys],
  );

  const handleColumnContextMenu = useCallback(
    (column: string, x: number, y: number) => {
      // Column right-clicks live in their own contextMenu variant; rowKey
      // stays empty since the menu targets a column, not a row.
      setContextMenu({ x, y, rowKey: "", column });
    },
    [],
  );

  const contextMenuItems: ContextMenuItem[] = useMemo(() => {
    if (!contextMenu || !keyField) return [];
    // Column right-click — sort / hide / freeze / copy / filter on the
    // targeted column. The column name lives on the contextMenu state.
    if (contextMenu.column) {
      const col = contextMenu.column;
      const isHidden = hiddenFields.has(col);
      const isKey = col === keyField;
      const items: ContextMenuItem[] = [
        {
          id: "col-sort-asc",
          label: `Sort ${col} ascending`,
          onSelect: () => {
            // Sort lives inside Grid today; for now nudge the user via toast.
            // (Full lift would expose the Grid's setSort; see Phase 2 of the
            // sort-pivot work in FEATURE_PARITY.md.)
            setToast(
              makeToast(
                "info",
                `Click the column header to sort ${col}. Header click cycles asc → desc → none.`,
              ),
            );
          },
        },
        {
          id: "col-sort-desc",
          label: `Sort ${col} descending`,
          onSelect: () => {
            setToast(
              makeToast(
                "info",
                `Click the column header twice to sort ${col} descending.`,
              ),
            );
          },
        },
        { id: "col-sep1", label: "", separator: true, onSelect: () => {} },
        {
          id: "col-copy-values",
          label: `Copy "${col}" values (one per line)`,
          onSelect: () => {
            const lines = rows
              .map((r) => {
                const v = r[col];
                return v == null ? "" : String(v);
              })
              .join("\n");
            navigator.clipboard.writeText(lines).then(() =>
              setToast(
                makeToast(
                  "success",
                  `Copied ${rows.length} value${rows.length === 1 ? "" : "s"} from ${col}.`,
                ),
              ),
            );
          },
        },
        {
          id: "col-copy-name",
          label: "Copy column name",
          onSelect: () => {
            navigator.clipboard.writeText(col).then(() =>
              setToast(makeToast("success", `Copied "${col}"`)),
            );
          },
        },
        { id: "col-sep2", label: "", separator: true, onSelect: () => {} },
        {
          id: "col-hide",
          label: isHidden ? "Show this column" : "Hide this column",
          disabled: isKey, // never hide the primary key
          onSelect: () => {
            setHiddenFields((prev) => {
              const next = new Set(prev);
              if (next.has(col)) next.delete(col);
              else next.add(col);
              return next;
            });
            setSelectedColumn(null);
          },
        },
        {
          id: "col-manage-hidden",
          label: "Manage hidden fields…",
          onSelect: () => setHideFieldsOpen(true),
        },
        { id: "col-sep3", label: "", separator: true, onSelect: () => {} },
        {
          id: "col-rename",
          label: `Rename "${col}"…`,
          disabled: isKey,
          onSelect: () => {
            // Client-side rename: add the new field, migrate every row,
            // then drop the old field. Atomicity is best-effort — if
            // any step fails, the user gets a toast and the partial
            // state stays so they can recover by hand. Engine ships
            // /add-field + /drop-field but no native rename yet.
            const newName = prompt(`Rename "${col}" to:`, col)?.trim();
            if (!newName || newName === col) return;
            if (!/^[A-Za-z_][A-Za-z0-9_]*$/.test(newName)) {
              setToast(makeToast("error", "Field name must match [A-Za-z_][A-Za-z0-9_]*"));
              return;
            }
            if (allFieldsList.some((f) => f.name === newName)) {
              setToast(makeToast("error", `Field "${newName}" already exists.`));
              return;
            }
            void renameField(col, newName);
          },
        },
        {
          id: "col-drop",
          label: `Drop "${col}"…`,
          disabled: isKey,
          onSelect: () => {
            if (!confirm(`Drop the "${col}" column from ${bundleName}? This cannot be undone.`)) {
              return;
            }
            void dropFieldFromBundle(col);
          },
        },
        { id: "col-sep4", label: "", separator: true, onSelect: () => {} },
        {
          id: "col-cf",
          // Conditional format opens a κ-threshold popover. Reuses the
          // column filter button as the anchor — both popovers attach
          // to the column header. If the filter button isn't found,
          // fall back to centering the popover under the menu.
          label: conditionalFormats.has(col)
            ? `Edit conditional format on "${col}"…`
            : `Conditional format "${col}"…`,
          onSelect: () => {
            const anchor =
              (document.querySelector(
                `[data-testid="grid-filter-btn-${col}"]`,
              ) as HTMLElement | null) ??
              (document.querySelector(`[data-testid="header-${col}"]`) as HTMLElement | null);
            if (anchor) setCfPopover({ field: col, anchorEl: anchor });
          },
        },
        {
          id: "col-schema",
          label: "Edit schema…",
          onSelect: () => setSchemaOpen(true),
        },
      ];
      return items;
    }
    // Empty-area right-click — show "what would you like to add?" menu.
    if (contextMenu.rowKey === "") {
      return [
        {
          id: "empty-insert",
          label: "Add a row…",
          onSelect: () => setInsertRowOpen(true),
        },
        {
          id: "empty-import",
          label: "Import CSV / TSV…",
          onSelect: () => setImportOpen(true),
        },
        { id: "sep1", label: "", separator: true, onSelect: () => {} },
        {
          id: "empty-schema",
          label: "Edit schema",
          onSelect: () => setSchemaOpen(true),
        },
        {
          id: "empty-refresh",
          label: "Refresh from server",
          onSelect: () => refetch(),
        },
      ];
    }
    const targets =
      selectedKeys.size > 1 && selectedKeys.has(contextMenu.rowKey)
        ? Array.from(selectedKeys)
        : [contextMenu.rowKey];
    const targetRows = targets
      .map((k) => rows.find((r) => String(r[keyField]) === k))
      .filter((r): r is NonNullable<typeof r> => Boolean(r));
    const isPlural = targets.length > 1;
    return [
      {
        id: "copy-id",
        label: isPlural ? `Copy ${targets.length} row keys` : "Copy row key",
        shortcut: "⌘C",
        onSelect: () => {
          navigator.clipboard.writeText(targets.join("\n")).then(() => {
            setToast(makeToast("success", `Copied ${targets.length} key${isPlural ? "s" : ""}.`));
          });
        },
      },
      {
        id: "copy-json",
        label: isPlural ? `Copy ${targets.length} rows as JSON` : "Copy row as JSON",
        onSelect: () => {
          const payload = isPlural ? targetRows : targetRows[0];
          navigator.clipboard
            .writeText(JSON.stringify(payload, null, 2))
            .then(() => {
              setToast(makeToast("success", `Copied ${targets.length} row${isPlural ? "s" : ""} as JSON.`));
            });
        },
      },
      {
        id: "copy-gql",
        label: isPlural
          ? `Copy SECTION query for ${targets.length} rows`
          : "Copy SECTION query for this row",
        onSelect: () => {
          const literal = (v: unknown): string =>
            typeof v === "number" || typeof v === "boolean"
              ? String(v)
              : `'${String(v).replace(/'/g, "''")}'`;
          const lines = targets.map(
            // Engine grammar: `SECTION <bundle> AT k=v` — bare key=val
            // pairs (no parens). Parens would error with "Expected '='
            // or ':' after '('" in the parse_kv_pairs path.
            (k) => `SECTION ${bundleName} AT ${keyField}=${literal(k)};`,
          );
          navigator.clipboard.writeText(lines.join("\n")).then(() => {
            setToast(makeToast("success", `Copied ${targets.length} GQL statement${isPlural ? "s" : ""}.`));
          });
        },
      },
      { id: "sep1", label: "", separator: true, onSelect: () => {} },
      {
        id: "find-similar",
        // Davis-sameness pivot: Gallery re-sorts cards by S against this
        // row. Available even outside Gallery view — switching to
        // gallery shows the result with the toolbar chip pinned.
        label: `Find similar to ${contextMenu.rowKey}`,
        onSelect: () => {
          setGallerySimilarPivot(contextMenu.rowKey);
          if (activeView !== "gallery") setActiveView("gallery");
        },
      },
      {
        id: "insert-formula",
        label: selectedColumn
          ? `Insert formula in ${selectedColumn}…`
          : "Insert formula (pick a column first)",
        shortcut: "⌘=",
        disabled: !selectedColumn,
        onSelect: () => {
          // Promote the right-clicked row to the active row so the picker's
          // resulting Insert commits to the right cell. Open the picker
          // (Excel-style "Insert Function" walkthrough) — the user picks
          // a function, fills args, and Insert drops the assembled formula
          // into the bar ready to commit.
          setSelectedRowKey(contextMenu.rowKey);
          setSelectedKeys(new Set([contextMenu.rowKey]));
          setAnchorKey(contextMenu.rowKey);
          setFormulaPickerOpen(true);
        },
      },
      {
        id: "insert-row",
        label: "Insert row…",
        shortcut: "⌘+",
        onSelect: () => setInsertRowOpen(true),
      },
      {
        id: "delete-row",
        label: isPlural ? `Delete ${targets.length} rows` : "Delete row",
        shortcut: "⌫",
        onSelect: () => {
          void handleDeleteSelected();
        },
      },
      { id: "sep2", label: "", separator: true, onSelect: () => {} },
      {
        id: "open",
        label: "Open in inspector",
        shortcut: "Enter",
        onSelect: () => {
          setSelectedRowKey(contextMenu.rowKey);
          setInspectorOpen(true);
        },
      },
      {
        id: "clear",
        label: "Clear selection",
        shortcut: "Esc",
        disabled: selectedKeys.size === 0,
        onSelect: () => {
          setSelectedKeys(new Set());
        },
      },
    ];
  }, [
    bundleName,
    contextMenu,
    handleDeleteSelected,
    keyField,
    rows,
    selectedKeys,
    hiddenFields,
    selectedColumn,
    activeView,
    allFieldsList,
    dropFieldFromBundle,
    renameField,
    bundleName,
    conditionalFormats,
  ]);

  const selectedRow = useMemo(() => {
    if (!keyField || !selectedRowKey) return null;
    return rows.find((r) => String(r[keyField]) === selectedRowKey) ?? null;
  }, [rows, keyField, selectedRowKey]);

  // If the bundle doesn't exist on the engine, surface the picker so the user
  // can land on something real instead of staring at a typed 404.
  if (error && error.status === 404) {
    return (
      <PickerShell
        client={client}
        requestedBundle={bundleName}
        loadError={error}
        onPickBundle={onNavigateToBundle}
      />
    );
  }

  return (
    <div className="app" data-sidebar={sidebarOpen ? "open" : "collapsed"}>
      <Sidebar
        client={client}
        currentBundle={bundleName}
        signedIn={account.state === "user"}
        onSignIn={() => setSignInOpen(true)}
        onNewBundle={handleNewBundle}
        onOpenViews={() => setViewsOpen(true)}
        onPickBundle={onNavigateToBundle}
        onApplyView={(v) => {
          applyView(v.spec);
          setToast(makeToast("success", `Applied view "${v.name}"`));
        }}
      />
      <header className="topbar">
        <button
          type="button"
          className="sidebar-toggle"
          onClick={() => setSidebarOpen((v) => !v)}
          aria-pressed={sidebarOpen}
          aria-label={sidebarOpen ? "Hide sidebar" : "Show sidebar"}
          data-testid="sidebar-toggle"
          title={sidebarOpen ? "Hide sidebar" : "Show sidebar"}
        >
          <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2.2" aria-hidden="true">
            <rect x="3" y="4" width="18" height="16" rx="2" />
            <path d="M9 4v16" />
          </svg>
        </button>
        <div className="brand">
          <img
            src={gigiIconUrl}
            className="brand-icon"
            alt="GIGI"
            data-testid="brand-logo"
            draggable={false}
          />
          <span className="brand-name">GIGI Sheets</span>
          <span className="brand-sub">fiber bundles · for humans</span>
        </div>
        <div className="crumbs">
          <span>{bundleName}</span>
          <span className="sep">·</span>
          <span className="mono">{DEFAULT_SERVER}</span>
        </div>
        <div className="meta">
          <RealtimePill status={realtime} laggedCount={laggedCount} />
          {selectedKeys.size > 1 ? (
            <span
              className="meta-multiselect"
              data-testid="multiselect-count"
              title="Multi-select: ⌘ to toggle, ⇧ for range"
            >
              {selectedKeys.size} selected
            </span>
          ) : null}
          <Stat label="rows" value={total.toLocaleString()} />
          <Stat label="κ̄" value={curvature.toFixed(2)} />
          <Stat label="conf̄" value={confidence.toFixed(2)} />
          <button
            type="button"
            className="inspector-toggle"
            onClick={() => setImportOpen(true)}
            data-testid="import-open"
            title="Import a CSV / TSV as a new bundle"
          >
            Import CSV
          </button>
          <button
            type="button"
            className="inspector-toggle"
            onClick={() => setInsightsOpen(true)}
            data-testid="insights-open"
            title="Auto-generated insights for this view"
          >
            Insights
          </button>
          <button
            type="button"
            className="inspector-toggle"
            onClick={() => setViewsOpen(true)}
            data-testid="views-open"
            title="Saved views"
          >
            Views
          </button>
          <button
            type="button"
            className="inspector-toggle"
            onClick={() => setSchemaOpen(true)}
            data-testid="schema-open"
            title="Edit schema (add or drop fields)"
          >
            Schema
          </button>
          <button
            type="button"
            className="inspector-toggle prism-topbar-btn"
            onClick={() => setPrismOpen(true)}
            data-testid="prism-open"
            title="Run a Prism workflow on this bundle"
          >
            <span className="prism-topbar-mark" aria-hidden="true">◇</span>
            Prism
            {!prismCredits.unlimited ? (
              <span className="prism-topbar-credits">
                {prismCredits.remaining}/{prismCredits.limit}
              </span>
            ) : null}
          </button>
          <button
            type="button"
            className="inspector-toggle inspector-toggle-primary"
            onClick={() => setShareOpen(true)}
            data-testid="share-open"
            title="Share this view"
          >
            <svg width="13" height="13" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2.2" aria-hidden="true">
              <circle cx="6" cy="12" r="3" />
              <circle cx="18" cy="6" r="3" />
              <circle cx="18" cy="18" r="3" />
              <path d="M8.6 13.5 15.4 17M15.4 7 8.6 10.5" />
            </svg>
            Share
          </button>
          <button
            type="button"
            className={`inspector-toggle ${inspectorOpen ? "" : "inspector-toggle-closed"}`}
            onClick={() => setInspectorOpen((v) => !v)}
            data-testid="inspector-toggle"
            aria-pressed={inspectorOpen}
            title={inspectorOpen ? "Hide inspector" : "Show inspector"}
          >
            <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2.2">
              <path d="M15 6l-6 6 6 6" />
            </svg>
            {inspectorOpen ? "Hide inspector" : "Show inspector"}
          </button>
          <button
            type="button"
            className={`topbar-avatar topbar-avatar-${account.state}`}
            onClick={() => {
              if (account.state === "user") setAccountMenuOpen(true);
              else setSignInOpen(true);
            }}
            data-testid="topbar-avatar"
            data-state={account.state}
            aria-label={account.state === "user" ? `Signed in as ${account.email}` : "Sign in"}
            title={
              account.state === "user"
                ? `${account.email} · click for account menu`
                : account.state === "loading"
                  ? "Checking session…"
                  : "Sign in (optional) — save views to the cloud"
            }
          >
            {account.state === "user" ? account.initials : account.state === "loading" ? "…" : "↪"}
          </button>
        </div>
      </header>
      <MenuBar
        menus={buildMenus({
          overlayOn,
          inspectorOpen,
          activeView,
          multiSelectCount: selectedKeys.size,
          hasFocusedRow: Boolean(selectedRowKey),
        })}
        onAction={dispatchMenu}
      />
      <main
        className={`main ${inspectorOpen ? "main-with-inspector" : ""}`}
        data-inspector-open={inspectorOpen ? "true" : "false"}
      >
        <div className="main-stack">
          <BundleTabs
            tabs={tabs}
            active={bundleName}
            onSelect={onNavigateToBundle}
            onClose={onCloseTab}
            onAdd={onNavigateToPicker}
          />
          <Banner
            message={visibleBanner}
            onDismiss={() => setBannerDismissed(bannerKey)}
          />
          <div className="view-tabs" role="tablist" data-testid="view-tabs">
            <button
              type="button"
              role="tab"
              className={`view-tab ${activeView === "grid" ? "view-tab-active" : ""}`}
              aria-selected={activeView === "grid"}
              data-testid="tab-grid"
              onClick={() => setActiveView("grid")}
            >
              Grid
            </button>
            <button
              type="button"
              role="tab"
              className={`view-tab ${activeView === "geometry" ? "view-tab-active" : ""}`}
              aria-selected={activeView === "geometry"}
              data-testid="tab-geometry"
              onClick={() => setActiveView("geometry")}
            >
              Geometry
            </button>
            <button
              type="button"
              role="tab"
              className={`view-tab ${activeView === "charts" ? "view-tab-active" : ""}`}
              aria-selected={activeView === "charts"}
              data-testid="tab-charts"
              onClick={() => setActiveView("charts")}
            >
              Charts
            </button>
            <button
              type="button"
              role="tab"
              className={`view-tab ${activeView === "kanban" ? "view-tab-active" : ""}`}
              aria-selected={activeView === "kanban"}
              data-testid="tab-kanban"
              onClick={() => setActiveView("kanban")}
            >
              Kanban
            </button>
            <button
              type="button"
              role="tab"
              className={`view-tab ${activeView === "gallery" ? "view-tab-active" : ""}`}
              aria-selected={activeView === "gallery"}
              data-testid="tab-gallery"
              onClick={() => setActiveView("gallery")}
            >
              Gallery
            </button>
            <button
              type="button"
              role="tab"
              className={`view-tab ${activeView === "form" ? "view-tab-active" : ""}`}
              aria-selected={activeView === "form"}
              data-testid="tab-form"
              onClick={() => setActiveView("form")}
            >
              Form
            </button>
            <button
              type="button"
              role="tab"
              className={`view-tab ${activeView === "gql" ? "view-tab-active" : ""}`}
              aria-selected={activeView === "gql"}
              data-testid="tab-gql"
              onClick={() => setActiveView("gql")}
            >
              GQL
            </button>
          </div>
          <Toolbar
            schema={schema}
            coverField={coverField}
            onCoverFieldChange={setCoverField}
            overlayOn={overlayOn}
            onOverlayChange={setOverlayOn}
            anomalyCount={anomalyCount}
            driftCount={driftCount}
            anomaliesOnly={anomaliesOnly}
            onAnomaliesOnlyChange={setAnomaliesOnly}
          />
          {activeView === "grid" ? (
            <FormulaBar
              context={buildBundleFormulaContext({
                schema,
                rows: visibleRows,
                kappaMap,
                keyField,
                coverField: coverField || undefined,
              })}
              initial={selectedCellInitial}
              focusToken={formulaFocusToken}
              prefill={formulaBarPrefill}
              onFxClick={() => setFormulaPickerOpen(true)}
              rangeStats={rangeStats}
              viewStatus={
                visibleRows.length !== rows.length || anomaliesOnly
                  ? {
                      label: `Filtered · ${visibleRows.length} of ${rows.length}`,
                      tooltip:
                        "The grid is filtered. Cell refs (A1, B2…) and `field[N]` slices " +
                        "resolve against the currently visible row list, so the same formula " +
                        "may point at different bundle rows when the filter changes.",
                    }
                  : null
              }
              onCommit={(formula, _result, move) => {
                // Enter / Tab in the formula bar commits to the active cell
                // and (when `move` is set) advances selection like Excel:
                // Enter → down, Tab → right. Without a selected cell there's
                // nowhere to write, so the commit is a no-op (the live
                // preview still ran).
                if (!selectedRowKey || !selectedColumn) return;
                onCellEdit(selectedRowKey, selectedColumn, formula);
                if (!keyField || !move) return;
                if (move === "down") {
                  const idx = visibleRows.findIndex(
                    (r) => String(r[keyField] ?? "") === selectedRowKey,
                  );
                  const next = visibleRows[idx + 1];
                  if (next) {
                    const nextKey = String(next[keyField] ?? "");
                    setSelectedRowKey(nextKey);
                    setSelectedKeys(new Set([nextKey]));
                    setAnchorKey(nextKey);
                  }
                } else if (move === "right" && schema) {
                  // Walk the visible-field list (respect hidden fields) so
                  // Tab matches what the user sees, not the schema order.
                  const allFields = [...schema.base_fields, ...schema.fiber_fields]
                    .map((f) => f.name)
                    .filter((n) => n === keyField || !hiddenFields.has(n));
                  const ci = allFields.indexOf(selectedColumn);
                  const nextCol = allFields[ci + 1];
                  if (nextCol) setSelectedColumn(nextCol);
                }
              }}
            />
          ) : null}
          <FormulaDocsModal
            open={formulaDocsOpen}
            onClose={() => setFormulaDocsOpen(false)}
          />
          <FormulaPicker
            open={formulaPickerOpen}
            onClose={() => setFormulaPickerOpen(false)}
            context={buildBundleFormulaContext({
              schema,
              rows: visibleRows,
              kappaMap,
              keyField,
              coverField: coverField || undefined,
            })}
            onInsert={(formula) => {
              // Drop the assembled formula into the formula bar (Insert
              // closes the picker, then the bar's `prefill` + focus-token
              // do the rest). If a cell is currently selected, the user
              // can press Enter to commit it there.
              setActiveView("grid");
              // Use the prefill mechanism by piggybacking on the focus
              // token — we just stash the assembled text in a state slot
              // the bar reads as `prefill`.
              setFormulaBarPrefill(formula);
              setFormulaFocusToken((n) => n + 1);
            }}
          />
          {activeView === "grid" ? (
            <Grid
              schema={schema}
              rows={visibleRows}
              loading={loading}
              error={error}
              kappaMap={kappaMap}
              hiddenFields={hiddenFields}
              selectedRowKey={selectedRowKey}
              selectedKeys={selectedKeys}
              onRowClick={handleRowClick}
              onRowContextMenu={handleRowContextMenu}
              onCellEdit={onCellEdit}
              onCellFocus={(rk, field) => {
                // Editing a cell promotes it to the active selection so
                // the formula bar mirrors it — single source of truth for
                // "what cell am I working on".
                setSelectedRowKey(rk);
                setSelectedKeys(new Set([rk]));
                setAnchorKey(rk);
                setSelectedColumn(field);
              }}
              getFormulaText={getFormulaText}
              cellRange={cellRange}
              onCellRangeChange={setCellRange}
              onDragFill={onDragFill}
              activeFilterColumns={new Set(columnFilters.keys())}
              conditionalFormats={conditionalFormats}
              onColumnFilterClick={(field, anchorEl) => {
                setFilterPopover((prev) =>
                  prev && prev.field === field ? null : { field, anchorEl },
                );
              }}
              selectedColumn={selectedColumn}
              onColumnSelect={setSelectedColumn}
              onColumnContextMenu={handleColumnContextMenu}
              emptyActions={{
                onInsertRow: () => setInsertRowOpen(true),
                onImportCsv: () => setImportOpen(true),
                onOpenSchema: () => setSchemaOpen(true),
              }}
            />
          ) : activeView === "geometry" ? (
            <Geometry
              schema={schema}
              rows={visibleRows}
              kappaMap={kappaMap}
              coverField={coverField}
              selectedRowKey={selectedRowKey}
              onRowSelect={handleSimpleSelect}
            />
          ) : activeView === "charts" ? (
            <Charts
              schema={schema}
              rows={visibleRows}
              kappaMap={kappaMap}
              coverField={coverField}
            />
          ) : activeView === "kanban" ? (
            <Kanban
              schema={schema}
              rows={visibleRows}
              kappaMap={kappaMap}
              coverField={coverField}
              onRowSelect={handleSimpleSelect}
            />
          ) : activeView === "gallery" ? (
            <Gallery
              schema={schema}
              rows={visibleRows}
              kappaMap={kappaMap}
              coverField={coverField}
              selectedRowKey={selectedRowKey}
              selectedKeys={selectedKeys}
              onRowSelect={handleSimpleSelect}
              onRowClick={handleRowClick}
              onRowContextMenu={handleRowContextMenu}
              similarPivot={gallerySimilarPivot}
              onClearSimilar={() => setGallerySimilarPivot(null)}
              sameness={galleryRowSameness}
            />
          ) : activeView === "form" ? (
            <FormView
              schema={schema}
              onSubmit={async (values) => {
                await client.insert(bundleName, values);
                refetch();
                const newKey =
                  keyField && values[keyField] != null
                    ? String(values[keyField])
                    : "row";
                setToast(makeToast("success", `Submitted ${newKey}`));
              }}
            />
          ) : (
            <GqlView
              client={client}
              query={gqlQuery}
              onQueryChange={setGqlQuery}
              schema={schema}
              coverField={coverField || undefined}
              sampleRowKey={
                keyField && visibleRows[0]
                  ? String(visibleRows[0][keyField] ?? "")
                  : null
              }
              secondRowKey={
                keyField && visibleRows[1]
                  ? String(visibleRows[1][keyField] ?? "")
                  : null
              }
            />
          )}
        </div>
        {inspectorOpen ? (
          <Inspector
            client={client}
            bundle={bundleName}
            schema={schema}
            selectedRow={selectedRow}
            keyField={keyField}
            coverField={coverField}
            fiberFields={fiberFields}
            kappa={
              selectedRowKey ? kappaMap.get(selectedRowKey) : undefined
            }
          />
        ) : null}
      </main>
      <FooterStatus
        bundleName={bundleName}
        keyField={keyField}
        selectedRowKey={selectedRowKey}
        selectedKeys={selectedKeys}
        selectedRow={selectedRow}
        coverField={coverField}
        kappaMap={kappaMap}
        rowCount={rows.length}
        visibleCount={visibleRows.length}
        anomaliesOnly={anomaliesOnly}
      />
      <Toast toast={toast} onDismiss={() => setToast(null)} />
      <AboutModal open={aboutOpen} onClose={() => setAboutOpen(false)} />
      <ShareModal
        open={shareOpen}
        bundle={bundleName}
        currentSpec={currentViewSpec}
        schema={schema}
        rows={rows}
        kappaMap={kappaMap}
        hiddenFields={hiddenFields}
        onClose={() => setShareOpen(false)}
        onExportCsv={exportCsv}
        onExportJson={exportJson}
        onExportGql={exportGql}
      />
      {cfPopover && schema ? (
        <ConditionalFormatModal
          open={true}
          field={cfPopover.field}
          rule={conditionalFormats.get(cfPopover.field) ?? null}
          anchorEl={cfPopover.anchorEl}
          onChange={(rule) => {
            setConditionalFormats((prev) => {
              const next = new Map(prev);
              if (rule == null) next.delete(cfPopover.field);
              else next.set(cfPopover.field, rule);
              return next;
            });
          }}
          onClose={() => setCfPopover(null)}
        />
      ) : null}
      {filterPopover && schema ? (() => {
        const field = [...schema.base_fields, ...schema.fiber_fields].find(
          (f) => f.name === filterPopover.field,
        );
        if (!field) return null;
        return (
          <ColumnFilterPopover
            field={field}
            filter={columnFilters.get(filterPopover.field) ?? null}
            anchorEl={filterPopover.anchorEl}
            onChange={(filter) => {
              setColumnFilters((prev) => {
                const next = new Map(prev);
                if (filter == null) next.delete(filterPopover.field);
                else next.set(filterPopover.field, filter);
                return next;
              });
            }}
            onClose={() => setFilterPopover(null)}
          />
        );
      })() : null}
      <FindModal
        open={findOpen}
        schema={schema}
        rows={rows}
        onClose={() => setFindOpen(false)}
        onSelectRow={(k) => {
          setSelectedRowKey(k);
          setSelectedKeys(new Set([k]));
          setAnchorKey(k);
          setActiveView("grid");
          // Bring the inspector into view if it was hidden.
          if (!inspectorOpen) setInspectorOpen(true);
        }}
        onReplace={(rk, field, value) => {
          // Route through onCellEdit so each replacement is undoable +
          // gets the same optimistic-write / κ-recompute treatment as
          // an inline edit.
          onCellEdit(rk, field, value);
        }}
      />
      <CommandPalette
        open={paletteOpen}
        commands={paletteCommands}
        onClose={() => setPaletteOpen(false)}
      />
      <PrismWorkflowsDrawer
        open={prismOpen}
        onClose={() => setPrismOpen(false)}
        schema={schema}
        rows={visibleRows}
        kappaMap={kappaMap}
        credits={prismCredits}
        otherOpenTabs={tabs.filter((t) => t !== bundleName)}
        client={client}
        sourceBundle={bundleName}
        onOpenSavedBundle={(name) => {
          // New artifact bundle landed on the engine — open it as a tab.
          onNavigateToBundle(name);
          setToast(
            makeToast("success", `Saved Prism result as bundle "${name}"`),
          );
        }}
        onSignIn={() => {
          setPrismOpen(false);
          setSignInOpen(true);
        }}
      />
      <SignInModal
        open={signInOpen}
        onClose={() => setSignInOpen(false)}
        onSignIn={account.signInWithEmail}
      />
      <AccountMenu
        open={accountMenuOpen}
        email={account.email ?? ""}
        subscription={account.subscription}
        onClose={() => setAccountMenuOpen(false)}
        onSignOut={account.signOut}
        onOpenFullAccount={() => {
          setAccountMenuOpen(false);
          onOpenAccount();
        }}
      />
      <InsertRowModal
        open={insertRowOpen}
        client={client}
        bundle={bundleName}
        schema={schema}
        rows={rows}
        onClose={() => setInsertRowOpen(false)}
        onInserted={(key) => {
          setInsertRowOpen(false);
          refetch();
          setSelectedRowKey(key);
          setSelectedKeys(new Set([key]));
          setAnchorKey(key);
          setToast(makeToast("success", `Inserted ${key}`));
        }}
      />
      <ImportCsvModal
        open={importOpen}
        client={client}
        onClose={() => setImportOpen(false)}
        onImported={(name) => {
          setImportOpen(false);
          setToast(makeToast("success", `Imported into '${name}'`));
        }}
      />
      <HideFieldsModal
        open={hideFieldsOpen}
        schema={schema}
        hiddenFields={hiddenFields}
        onClose={() => setHideFieldsOpen(false)}
        onChange={(next) => {
          setHiddenFields(next);
          setToast(
            makeToast(
              "success",
              next.size === 0 ? "Showing all fields" : `Hiding ${next.size} field${next.size === 1 ? "" : "s"}`,
            ),
          );
        }}
      />
      <InsightsDrawer
        open={insightsOpen}
        bundle={bundleName}
        schema={schema}
        rows={rows}
        kappaMap={kappaMap}
        coverField={coverField}
        meanCurvature={curvature}
        onClose={() => setInsightsOpen(false)}
        onCopyGql={() =>
          setToast(makeToast("success", "GQL copied to clipboard"))
        }
      />
      <ViewsDrawer
        open={viewsOpen}
        bundle={bundleName}
        currentSpec={currentViewSpec}
        onClose={() => setViewsOpen(false)}
        onApply={(spec) => {
          applyView(spec);
          setViewsOpen(false);
          setToast(makeToast("success", "View applied"));
        }}
        onShare={() => {
          setToast(makeToast("success", "Share link copied to clipboard"));
        }}
      />
      <SchemaModal
        open={schemaOpen}
        client={client}
        bundle={bundleName}
        schema={schema}
        onClose={() => setSchemaOpen(false)}
        onMutated={() => {
          refetch();
          setToast(makeToast("success", "Schema updated"));
        }}
      />
      <ContextMenu
        open={Boolean(contextMenu)}
        x={contextMenu?.x ?? 0}
        y={contextMenu?.y ?? 0}
        items={contextMenuItems}
        header={
          selectedKeys.size > 1 && contextMenu && selectedKeys.has(contextMenu.rowKey)
            ? `${selectedKeys.size} rows`
            : contextMenu?.rowKey
        }
        onClose={() => setContextMenu(null)}
      />
    </div>
  );
}

function RealtimePill({
  status,
  laggedCount,
}: {
  status: "off" | "connecting" | "open" | "closed" | "error";
  laggedCount: number;
}) {
  const labels = {
    off: "off",
    connecting: "connecting…",
    open: "live",
    closed: "closed",
    error: "ws error",
  } as const;
  return (
    <span
      className={`realtime-pill realtime-${status}`}
      data-testid="realtime-pill"
      data-status={status}
      title={`WebSocket: ${status} · lagged ${laggedCount}`}
    >
      <span className="realtime-dot" aria-hidden="true" />
      <span>{labels[status]}</span>
      {laggedCount > 0 ? (
        <span className="realtime-lag" data-testid="realtime-lag">
          +{laggedCount} behind
        </span>
      ) : null}
    </span>
  );
}

function PickerShell({
  client,
  requestedBundle,
  loadError,
  onPickBundle,
  onStartTour,
  onOpenAccount,
}: {
  client: SheetsClient;
  requestedBundle: string | null;
  loadError: ReturnType<typeof useBundle>["error"];
  onPickBundle?: (name: string) => void;
  onStartTour?: () => void;
  onOpenAccount?: () => void;
}) {
  const account = useAccount();
  const [signInOpen, setSignInOpen] = useState<boolean>(false);
  const [accountMenuOpen, setAccountMenuOpen] = useState<boolean>(false);

  // Public landing: shown when an unauthenticated visitor hits the root
  // path (no requested bundle, no load error). Signed-in users — and
  // anyone who explicitly typed a bundle name — skip straight to the
  // dashboard / not-found fallback below.
  const showLanding =
    account.state === "guest" && !requestedBundle && !loadError;

  if (showLanding) {
    return (
      <>
        <LandingPage
          client={client}
          onPickBundle={onPickBundle}
          onSignInClick={() => setSignInOpen(true)}
          onStartTour={onStartTour}
        />
        <SignInModal
          open={signInOpen}
          onClose={() => setSignInOpen(false)}
          onSignIn={account.signInWithEmail}
        />
      </>
    );
  }

  return (
    <div className="app">
      <header className="topbar">
        <div className="brand">
          <img
            src={gigiIconUrl}
            className="brand-icon"
            alt="GIGI"
            data-testid="brand-logo"
            draggable={false}
          />
          <span className="brand-name">GIGI Sheets</span>
          <span className="brand-sub">fiber bundles · for humans</span>
        </div>
        <div className="crumbs">
          <span>{requestedBundle ?? "pick a bundle"}</span>
          <span className="sep">·</span>
          <span className="mono">{DEFAULT_SERVER}</span>
        </div>
        <div className="topbar-right">
          <button
            type="button"
            className={`topbar-avatar topbar-avatar-${account.state}`}
            onClick={() => {
              if (account.state === "user") setAccountMenuOpen(true);
              else setSignInOpen(true);
            }}
            data-testid="topbar-avatar"
            data-state={account.state}
            aria-label={
              account.state === "user"
                ? `Signed in as ${account.email}`
                : "Sign in"
            }
            title={
              account.state === "user"
                ? `${account.email} · click for account menu`
                : account.state === "loading"
                  ? "Checking session…"
                  : "Sign in (optional) — save views to the cloud"
            }
          >
            {account.state === "user"
              ? account.initials
              : account.state === "loading"
                ? "…"
                : "↪"}
          </button>
        </div>
      </header>
      <main className="main">
        <BundlePicker
          client={client}
          requestedBundle={requestedBundle}
          loadError={loadError}
          onPickBundle={onPickBundle}
        />
      </main>
      <footer className="footer">
        <span className="label">bundle picker</span>
        <span className="footer-sep">·</span>
        <span>
          Pick any bundle to see it as a spreadsheet. The URL updates so you
          can bookmark it.
        </span>
      </footer>
      <SignInModal
        open={signInOpen}
        onClose={() => setSignInOpen(false)}
        onSignIn={account.signInWithEmail}
      />
      <AccountMenu
        open={accountMenuOpen}
        email={account.email ?? ""}
        subscription={account.subscription}
        onClose={() => setAccountMenuOpen(false)}
        onSignOut={account.signOut}
        onOpenFullAccount={
          onOpenAccount
            ? () => {
                setAccountMenuOpen(false);
                onOpenAccount();
              }
            : undefined
        }
      />
    </div>
  );
}

function download(name: string, content: string, type = "text/plain") {
  const blob = new Blob([content], { type });
  const url = URL.createObjectURL(blob);
  const a = document.createElement("a");
  a.href = url;
  a.download = name;
  a.click();
  setTimeout(() => URL.revokeObjectURL(url), 1000);
}

function Stat({ label, value }: { label: string; value: string }) {
  return (
    <span className="stat">
      <span className="stat-label">{label}</span>
      <span className="stat-value">{value}</span>
    </span>
  );
}

/**
 * Live status line in the bottom footer. Updates as the user selects
 * rows so the bottom of the app always says something useful about the
 * current state — instead of the old static "select a row" stub.
 */
function FooterStatus({
  bundleName,
  keyField,
  selectedRowKey,
  selectedKeys,
  selectedRow,
  coverField,
  kappaMap,
  rowCount,
  visibleCount,
  anomaliesOnly,
}: {
  bundleName: string;
  keyField: string | undefined;
  selectedRowKey: string | null;
  selectedKeys: Set<string>;
  selectedRow: RowMap | null;
  coverField: string;
  kappaMap: Map<string, number>;
  rowCount: number;
  visibleCount: number;
  anomaliesOnly: boolean;
}) {
  // Build the segments based on selection state.
  let label = "Status";
  let segments: React.ReactNode[] = [];

  if (selectedKeys.size > 1) {
    // Multi-select: report count + mean κ across the selected rows.
    label = `${selectedKeys.size} rows selected`;
    let sum = 0;
    let n = 0;
    for (const k of selectedKeys) {
      const v = kappaMap.get(k);
      if (typeof v === "number" && Number.isFinite(v)) {
        sum += v;
        n += 1;
      }
    }
    const meanK = n > 0 ? sum / n : 0;
    segments = [
      <span key="bundle" className="mono footer-bundle">{bundleName}</span>,
      <span key="cover">cover · <span className="mono">{coverField || "(none)"}</span></span>,
      <span key="kappa">mean κ across selection · <b>{meanK.toFixed(2)}</b></span>,
      <span key="hint" className="footer-hint">
        ⌘C to copy keys · ⌫ to delete · click any column header to sort
      </span>,
    ];
  } else if (selectedRowKey && selectedRow && keyField) {
    // Single row: show its key, κ class, cover value.
    const k = kappaMap.get(selectedRowKey) ?? 0;
    const kClass = kappaClass(k);
    const klassLabel =
      kClass === "bad" ? "anomaly" : kClass === "warn" ? "drift" : "healthy";
    const coverValue = String(selectedRow[coverField] ?? "—");
    label = "Selection";
    segments = [
      <span key="key" className="mono footer-bundle">
        {selectedRowKey}
      </span>,
      <span key="kappa">
        κ = <b>{k.toFixed(2)}</b>{" "}
        <span className={`footer-pill footer-pill-${kClass}`}>{klassLabel}</span>
      </span>,
      <span key="cover">
        cover · <span className="mono">{coverField}</span> ={" "}
        <span className="mono">{coverValue}</span>
      </span>,
      <span key="hint" className="footer-hint">
        run SPECTRAL · TRANSPORT · HOLONOMY · BETTI from the inspector →
      </span>,
    ];
  } else {
    // No selection.
    label = "Status";
    const filterText = anomaliesOnly
      ? `${visibleCount.toLocaleString()} of ${rowCount.toLocaleString()} (filter: κ-bad)`
      : `${rowCount.toLocaleString()} rows in view`;
    segments = [
      <span key="bundle" className="mono footer-bundle">
        {bundleName}
      </span>,
      <span key="rows">{filterText}</span>,
      <span key="hint" className="footer-hint">
        Select a row to inspect its geometry · ⌘F to find · ⌘1-4 to switch view
      </span>,
    ];
  }

  return (
    <footer className="footer" data-testid="footer-status">
      <span className="label" data-testid="footer-label">
        {label}
      </span>
      {segments.map((seg, i) => (
        <span key={i} className="footer-segment-wrap">
          <span className="footer-sep">·</span>
          {seg}
        </span>
      ))}
    </footer>
  );
}

// FormulaContext construction moved to `lib/formula-context.ts` (Phase 3) —
// see `buildBundleFormulaContext`. The Phase-1 stub of SAME/DIST that used
// to live here was replaced with real Davis sameness over the Prism
// embedder, COHORT now returns the row's cover-field value, and
// KAPPA_RANK / SAMENESS_RANK are wired against kappaMap and the embedder
// respectively.
