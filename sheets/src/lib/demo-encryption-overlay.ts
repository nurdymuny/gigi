/**
 * Client-side encryption overlay for demo bundles.
 *
 * The engine spec (addendum E-S8a) commits to returning `encryption` per
 * field on `/schema`, but that engine work hasn't landed yet. Until it
 * does, this module lets the UI *demonstrate* gauge encryption visually
 * on demo bundles: when a demo is imported, the loader registers a
 * `field → mode` map here. The gigi-client merges the overlay into every
 * schema fetch so the Grid + Inspector render encrypted fields properly
 * (masked OPAQUE values, INDEXED-style lookups, AFFINE numeric badges).
 *
 * Scope is intentionally narrow:
 *   - Only fires for bundles whose name appears in the registry.
 *   - Real engine-side encryption (once shipped) takes precedence — the
 *     overlay only fills in fields the server didn't already mark.
 *   - Backed by localStorage so the overlay survives page reloads without
 *     re-importing the demo.
 */

// Type-only import to avoid runtime circular dependency with gigi-client.
import type { BundleSchema, FieldDescriptor } from "./gigi-client";
/* eslint-disable @typescript-eslint/consistent-type-imports */

export type EncryptionMode = "opaque" | "indexed" | "affine";

const STORAGE_KEY = "gigi.sheets.demo_encryption";

type Registry = Record<string, Record<string, EncryptionMode>>;

function readRegistry(): Registry {
  if (typeof localStorage === "undefined") return {};
  const raw = localStorage.getItem(STORAGE_KEY);
  if (!raw) return {};
  try {
    const parsed = JSON.parse(raw);
    return parsed && typeof parsed === "object" ? (parsed as Registry) : {};
  } catch {
    return {};
  }
}

function writeRegistry(reg: Registry): void {
  if (typeof localStorage === "undefined") return;
  localStorage.setItem(STORAGE_KEY, JSON.stringify(reg));
}

/**
 * Register an encryption overlay for a bundle. Called by the demo loader
 * after a successful import. Overwrites any prior overlay for the same
 * bundle.
 */
export function registerOverlay(
  bundle: string,
  fields: Record<string, EncryptionMode>,
): void {
  const reg = readRegistry();
  reg[bundle] = { ...fields };
  writeRegistry(reg);
}

/** Wipe the overlay for one bundle (e.g. if the user drops it). */
export function clearOverlay(bundle: string): void {
  const reg = readRegistry();
  if (bundle in reg) {
    delete reg[bundle];
    writeRegistry(reg);
  }
}

/** Inspect the overlay for a bundle. Returns null if none is registered. */
export function getOverlay(
  bundle: string,
): Record<string, EncryptionMode> | null {
  const reg = readRegistry();
  return reg[bundle] ?? null;
}

/**
 * Apply the overlay (if any) to a freshly-fetched schema. Server-side
 * encryption metadata is preserved; the overlay only fills in fields where
 * `encryption` is undefined or "none". Pure — does not mutate the input.
 */
export function applyOverlay(schema: BundleSchema): BundleSchema {
  const overlay = getOverlay(schema.name);
  if (!overlay) return schema;
  const tag = (f: FieldDescriptor): FieldDescriptor => {
    const fromOverlay = overlay[f.name];
    if (!fromOverlay) return f;
    // If the engine already supplied a non-"none" encryption, defer to it.
    if (f.encryption && f.encryption !== "none") return f;
    return { ...f, encryption: fromOverlay };
  };
  return {
    ...schema,
    base_fields: schema.base_fields.map(tag),
    fiber_fields: schema.fiber_fields.map(tag),
  };
}
