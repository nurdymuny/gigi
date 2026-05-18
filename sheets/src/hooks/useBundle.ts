import { useCallback, useEffect, useRef, useState } from "react";
import {
  SheetsClient,
  SheetsClientError,
  applyEventToRows,
  type BundleSchema,
  type RowMap,
  type SectionQuery,
} from "../lib/gigi-client";

export interface MutationResult {
  ok: boolean;
  /** Set on failure. */
  error?: SheetsClientError;
  /** Updated κ from the engine response (bundle-level). */
  curvature?: number;
  /** Updated conf from the engine response. */
  confidence?: number;
}

export type RealtimeStatus = "off" | "connecting" | "open" | "closed" | "error";

export interface BundleState {
  schema: BundleSchema | null;
  rows: RowMap[];
  total: number;
  curvature: number;
  confidence: number;
  loading: boolean;
  error: SheetsClientError | null;
  /** Optimistic mutation: applied immediately, rolls back on failure. */
  updateCell: (rowKey: string, field: string, value: unknown) => Promise<MutationResult>;
  /** Re-fetch schema + rows. Useful after schema mutations. */
  refetch: () => void;
  /** WebSocket subscription status. */
  realtime: RealtimeStatus;
  /** Cumulative lag-notice count (engine emits NOTICE lagged=N under burst). */
  laggedCount: number;
}

const INITIAL = {
  schema: null as BundleSchema | null,
  rows: [] as RowMap[],
  total: 0,
  curvature: 0,
  confidence: 0,
  loading: true,
  error: null as SheetsClientError | null,
  realtime: "off" as RealtimeStatus,
  laggedCount: 0,
};

type InnerState = typeof INITIAL;

function replaceAt<T>(arr: T[], idx: number, next: T): T[] {
  const out = arr.slice();
  out[idx] = next;
  return out;
}

export interface UseBundleOptions {
  /** Subscribe to WS mutations for live updates. Default true. */
  realtime?: boolean;
}

export function useBundle(
  client: SheetsClient,
  bundle: string,
  query: SectionQuery = {},
  opts: UseBundleOptions = {},
): BundleState {
  const realtimeEnabled = opts.realtime ?? true;
  const [state, setState] = useState<InnerState>(INITIAL);
  const stateRef = useRef(state);
  stateRef.current = state;
  const [refetchTick, setRefetchTick] = useState(0);

  const queryKey = JSON.stringify(query);

  const refetch = useCallback(() => setRefetchTick((t) => t + 1), []);

  useEffect(() => {
    let cancelled = false;
    setState((s) => ({ ...s, loading: true, error: null }));

    Promise.all([client.schema(bundle), client.section(bundle, JSON.parse(queryKey))])
      .then(([schema, section]) => {
        if (cancelled) return;
        setState((s) => ({
          ...s,
          schema,
          rows: section.rows,
          total: section.total,
          curvature: section.curvature,
          confidence: section.confidence,
          loading: false,
          error: null,
        }));
      })
      .catch((err: unknown) => {
        if (cancelled) return;
        const e =
          err instanceof SheetsClientError
            ? err
            : new SheetsClientError(
                err instanceof Error ? err.message : String(err),
                "network_error",
              );
        setState((s) => ({ ...INITIAL, realtime: s.realtime, loading: false, error: e }));
      });

    return () => {
      cancelled = true;
    };
  }, [client, bundle, queryKey, refetchTick]);

  // WebSocket subscription — folds incoming mutations into rows.
  // Only opens once the schema has loaded successfully. A bundle that
  // 404s never gets a socket, so we don't thrash the engine or the
  // browser console with a ghost handshake.
  const schemaReady = state.schema !== null;
  useEffect(() => {
    if (!realtimeEnabled || !schemaReady) {
      setState((s) => ({ ...s, realtime: "off" }));
      return;
    }
    setState((s) => ({ ...s, realtime: "connecting" }));
    const sub = client.subscribe(
      bundle,
      (frame) => {
        const cur = stateRef.current;
        const keyField = cur.schema?.base_fields[0]?.name;
        if (!keyField) return;
        if (frame.kind === "notice") {
          setState((s) => ({ ...s, laggedCount: s.laggedCount + frame.lagged }));
          return;
        }
        // Event — only act on events for this bundle (defense in depth).
        if (frame.bundle !== bundle) return;
        setState((s) => ({
          ...s,
          rows: applyEventToRows(s.rows, keyField, frame),
          curvature: frame.kappa || s.curvature,
          confidence: frame.confidence || s.confidence,
        }));
      },
      (status) => {
        setState((s) => ({
          ...s,
          realtime:
            status === "open" ? "open" : status === "close" ? "closed" : "error",
        }));
      },
    );
    return () => sub.close();
  }, [client, bundle, realtimeEnabled, schemaReady]);

  const updateCell = useCallback(
    async (rowKey: string, field: string, value: unknown): Promise<MutationResult> => {
      const current = stateRef.current;
      const keyField = current.schema?.base_fields[0]?.name;
      if (!keyField) {
        const error = new SheetsClientError(
          "Bundle has no key field — cannot update",
          "no_key",
        );
        return { ok: false, error };
      }
      const rowIdx = current.rows.findIndex(
        (r) => String(r[keyField]) === rowKey,
      );
      if (rowIdx === -1) {
        const error = new SheetsClientError(
          `Row with ${keyField}='${rowKey}' not in current view`,
          "no_key",
        );
        return { ok: false, error };
      }

      const original = current.rows[rowIdx];
      const optimistic: RowMap = { ...original, [field]: value };

      // Apply optimistic state immediately.
      setState((s) => ({ ...s, rows: replaceAt(s.rows, rowIdx, optimistic) }));

      try {
        const result = await client.update(bundle, {
          key: { [keyField]: original[keyField] },
          fields: { [field]: value },
          returning: true,
        });
        setState((s) => ({
          ...s,
          rows: replaceAt(s.rows, rowIdx, result.data ?? optimistic),
          curvature: result.curvature,
          confidence: result.confidence,
        }));
        return {
          ok: true,
          curvature: result.curvature,
          confidence: result.confidence,
        };
      } catch (err) {
        // Roll back to the original row.
        setState((s) => ({ ...s, rows: replaceAt(s.rows, rowIdx, original) }));
        const error =
          err instanceof SheetsClientError
            ? err
            : new SheetsClientError(
                err instanceof Error ? err.message : String(err),
                "network_error",
              );
        return { ok: false, error };
      }
    },
    [client, bundle],
  );

  return { ...state, updateCell, refetch };
}

// Re-export for callers that want to detect "stale data" UX.
export type { RowMap };
