import { useSyncExternalStore } from "react";
import type { Language } from "../i18n";

// The same manifest files the Rust engine embeds (src-tauri/packages for game
// tuning, src-tauri/wardrobe for audited hero skins), so the interface and the
// engine can never disagree on what is installable.
type PackageManifest = {
  id: string;
  catalogId: string;
  /** Present on wardrobe items (`"wardrobe"`); tuning packages have none. */
  kind?: string;
  /** Replaced by a newer package; kept only so existing builds stay verifiable. */
  supersededBy?: string | null;
  name: Record<Language, string>;
  author: string;
  verifiedLanguages?: string[];
  conflicts?: string[];
  requires?: string[];
};

const manifests = {
  ...import.meta.glob<PackageManifest>("../../src-tauri/packages/*.json", {
    eager: true,
    import: "default",
  }),
  ...import.meta.glob<PackageManifest>("../../src-tauri/wardrobe/*.json", {
    eager: true,
    import: "default",
  }),
};

export const installablePackages: PackageManifest[] = Object.values(manifests)
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
Object.values(manifests).forEach(index);

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

export function findPackage(id: string) {
  return byAnyId.get(id);
}

/** Whether the package is an audited hero skin rather than a game-tuning mod. */
export function isWardrobePackage(id: string) {
  return findPackage(id)?.kind === "wardrobe";
}

export function engineIdFor(catalogId: string) {
  return findPackage(catalogId)?.id ?? catalogId;
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
