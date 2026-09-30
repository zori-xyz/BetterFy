import type { Language } from "../i18n";

// The same PackageManifest files the Rust engine embeds (src-tauri/packages),
// so the interface and the engine can never disagree on what is installable.
type PackageManifest = {
  id: string;
  catalogId: string;
  name: Record<Language, string>;
  author: string;
  verifiedLanguages?: string[];
  resources: { path: string; bytes: number; sha256: string }[];
};

const manifests = import.meta.glob<PackageManifest>("../../src-tauri/packages/*.json", {
  eager: true,
  import: "default",
});

export const installablePackages = Object.values(manifests).sort((a, b) =>
  a.catalogId.localeCompare(b.catalogId),
);

const byAnyId = new Map(
  installablePackages.flatMap((manifest) => [
    [manifest.id, manifest] as const,
    [manifest.catalogId, manifest] as const,
  ]),
);

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
