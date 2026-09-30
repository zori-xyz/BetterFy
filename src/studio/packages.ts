import { useSyncExternalStore } from "react";
import type { Language } from "../i18n";

// The same PackageManifest files the Rust engine embeds (src-tauri/packages),
// so the interface and the engine can never disagree on what is installable.
type PackageManifest = {
  id: string;
  catalogId: string;
  name: Record<Language, string>;
  author: string;
  verifiedLanguages?: string[];
};

const manifests = import.meta.glob<PackageManifest>("../../src-tauri/packages/*.json", {
  eager: true,
  import: "default",
});

export const installablePackages: PackageManifest[] = Object.values(manifests).sort((a, b) =>
  a.catalogId.localeCompare(b.catalogId),
);

const byAnyId = new Map<string, PackageManifest>();
/** Catalog IDs of every installable package; shared with the catalog model. */
export const pilotCatalogIds = new Set<string>();
function index(manifest: PackageManifest) {
  byAnyId.set(manifest.id, manifest);
  byAnyId.set(manifest.catalogId, manifest);
  pilotCatalogIds.add(manifest.catalogId);
}
installablePackages.forEach(index);

// The engine may activate a newer signed catalog at runtime. Its summaries
// replace or extend the embedded list; subscribers re-render.
let revision = 0;
const listeners = new Set<() => void>();
export function applyInstallablePackages(summaries: PackageManifest[]) {
  for (const summary of summaries) {
    const existing = installablePackages.findIndex((item) => item.id === summary.id);
    if (existing >= 0) installablePackages[existing] = summary;
    else installablePackages.push(summary);
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
export function findPackage(id: string) {
  return byAnyId.get(id);
}

export function engineIdFor(catalogId: string) {
  return findPackage(catalogId)?.id ?? catalogId;
}

export function packageName(id: string, language: Language) {
  return findPackage(id)?.name[language] ?? id;
}
