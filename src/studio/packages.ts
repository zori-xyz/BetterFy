import { useSyncExternalStore } from "react";
import type { Language } from "../i18n";

// The same manifest files the Rust engine embeds (src-tauri/packages for game
// tuning; src-tauri/wardrobe/index.json is the interface's copy of the audited
// hero skins in manifests.json, and a Rust test requires it to be derived from
// them), so the interface and the engine can never disagree on what is
// installable. src-tauri/maps/index.json is the same copy for the standard map
// terrains (whole-map packages installed as maps/dota.vpk), derived from
// src-tauri/maps/manifests.json.
type PackageManifest = {
  id: string;
  catalogId: string;
  /** `"wardrobe"` for a hero skin, `"map"` for a map terrain; tuning packages have none. */
  kind?: string;
  /** Replaced by a newer package; kept only so existing builds stay verifiable. */
  supersededBy?: string | null;
  name: Record<Language, string>;
  author: string;
  verifiedLanguages?: string[];
  conflicts?: string[];
  requires?: string[];
  /**
   * Wardrobe items: heroes the skin is for and the audited install sizes. For a
   * map, `installBytes` is the size of maps/dota.vpk and `installFiles` the
   * number of entries in it.
   */
  heroes?: string[];
  installFiles?: number;
  installBytes?: number;
  /** The same install without the files that override a shared Valve file. */
  stripBytes?: number;
  /** How many accepted files override a shared Valve file. */
  sharedFiles?: number;
};

const wardrobeIndex = import.meta.glob<PackageManifest[]>("../../src-tauri/wardrobe/index.json", {
  eager: true,
  import: "default",
});
const mapIndex = import.meta.glob<PackageManifest[]>("../../src-tauri/maps/index.json", {
  eager: true,
  import: "default",
});
const manifests: PackageManifest[] = [
  ...Object.values(
    import.meta.glob<PackageManifest>("../../src-tauri/packages/*.json", {
      eager: true,
      import: "default",
    }),
  ),
  ...Object.values(wardrobeIndex).flat(),
  ...Object.values(mapIndex).flat(),
];

export const installablePackages: PackageManifest[] = manifests
  .filter((manifest) => !manifest.supersededBy)
  .sort((a, b) => a.catalogId.localeCompare(b.catalogId));

const byAnyId = new Map<string, PackageManifest>();
/** Catalog IDs of every installable package; shared with the catalog model. */
export const pilotCatalogIds = new Set<string>();
function index(manifest: PackageManifest) {
  byAnyId.set(manifest.id, manifest);
  if (manifest.supersededBy) return;
  byAnyId.set(manifest.catalogId, manifest);
  pilotCatalogIds.add(manifest.catalogId);
}
manifests.forEach(index);

// The engine may activate a newer signed catalog at runtime. Its summaries
// replace or extend the embedded list; subscribers re-render.
let revision = 0;
const listeners = new Set<() => void>();
export function applyInstallablePackages(summaries: PackageManifest[]) {
  for (const summary of summaries) {
    const existing = installablePackages.findIndex((item) => item.id === summary.id);
    if (existing >= 0) {
      // Drop the previous catalog ID if the package was relabelled.
      const previous = installablePackages[existing];
      if (previous.catalogId !== summary.catalogId) {
        byAnyId.delete(previous.catalogId);
        pilotCatalogIds.delete(previous.catalogId);
      }
      if (summary.supersededBy) installablePackages.splice(existing, 1);
      else installablePackages[existing] = summary;
    } else if (!summary.supersededBy) installablePackages.push(summary);
    index(summary);
  }
  installablePackages.sort((a, b) => a.catalogId.localeCompare(b.catalogId));
  revision += 1;
  listeners.forEach((listener) => listener());
}
export function usePackagesRevision() {
  return useSyncExternalStore(
    (listener) => {
      listeners.add(listener);
      return () => listeners.delete(listener);
    },
    () => revision,
  );
}

/** Catalog ID (`minify-tree-mod`) or engine ID (`minify.tree-mod`). */
/** Upper bound on packages the engine merges into one build (tree_pilot.rs). */
export const MAX_BUNDLE_PACKAGES = 32;

/**
 * The engine ID suffix of the opt-in mode of a wardrobe item that leaves the
 * game's shared files untouched (`wardrobe.<name>.strip`). The engine treats
 * it as a separate package ID, so the mode is part of the reviewed plan.
 */
export const STRIP_SUFFIX = ".strip";

/** The item an ID names and whether it carries the opt-in suffix. */
function splitMode(id: string) {
  return id.endsWith(STRIP_SUFFIX)
    ? { base: id.slice(0, -STRIP_SUFFIX.length), strip: true }
    : { base: id, strip: false };
}

/** Catalog, engine or `.strip` engine ID; the suffix only exists on skins. */
export function findPackage(id: string) {
  const { base, strip } = splitMode(id);
  const manifest = byAnyId.get(base);
  return strip && manifest?.kind !== "wardrobe" ? undefined : manifest;
}

/** Whether the ID selects the opt-in mode that leaves shared game files out. */
export function isSharedKept(id: string) {
  return splitMode(id).strip && findPackage(id) !== undefined;
}

/** The request ID for an item in the chosen mode. */
export function withSharedMode(id: string, keepShared: boolean) {
  const manifest = findPackage(id);
  const base = manifest?.id ?? splitMode(id).base;
  return keepShared && manifest?.kind === "wardrobe" ? `${base}${STRIP_SUFFIX}` : base;
}

/** Upper bound on the merged VPK (mod_bundle.rs): 256 MiB of file data. */
export const MAX_BUNDLE_BYTES = 256 * 1024 * 1024;

/**
 * What the selected skins add to the merged VPK, from the audited sizes. Each
 * item counts in its own mode, so the warning matches the engine's check.
 */
export function wardrobeBytes(ids: string[]) {
  return ids.reduce((total, id) => {
    const manifest = findPackage(id);
    if (manifest?.kind !== "wardrobe") return total;
    const bytes = isSharedKept(id) ? manifest.stripBytes : manifest.installBytes;
    return total + (bytes ?? 0);
  }, 0);
}

/** Whether the package is an audited hero skin rather than a game-tuning mod. */
export function isWardrobePackage(id: string) {
  return findPackage(id)?.kind === "wardrobe";
}

/**
 * Whether the package is a map terrain: a whole-map package written as its own
 * file, `maps/dota.vpk`, not merged into the build's `pakNN_dir.vpk`.
 */
export function isMapPackage(id: string) {
  return findPackage(id)?.kind === "map";
}

export function engineIdFor(catalogId: string) {
  const { base, strip } = splitMode(catalogId);
  const manifest = findPackage(catalogId);
  if (!manifest) return catalogId;
  return strip ? `${manifest.id}${STRIP_SUFFIX}` : (byAnyId.get(base)?.id ?? catalogId);
}

/**
 * Minify's own rules (tree_pilot.rs checks the same): two packages that
 * conflict, or a package whose required package is not selected.
 */
export function bundleRelationProblem(
  ids: string[],
): { kind: "conflict" | "missing"; packageId: string; otherId: string } | null {
  const selected = new Set(ids.map((id) => findPackage(id)?.id ?? id));
  for (const id of selected) {
    const manifest = findPackage(id);
    const conflict = manifest?.conflicts?.find((other) => selected.has(other));
    if (conflict) return { kind: "conflict", packageId: id, otherId: conflict };
    const missing = manifest?.requires?.find((other) => !selected.has(other));
    if (missing) return { kind: "missing", packageId: id, otherId: missing };
  }
  return null;
}

export function packageName(id: string, language: Language) {
  return findPackage(id)?.name[language] ?? id;
}
