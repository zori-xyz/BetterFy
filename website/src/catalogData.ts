// The catalog as the website shows it, loaded after the first screen: the
// same data the app ships, so the counts and lists never drift from it.
import { categoryLabel, isInstallable, mods } from "../../src/studio/model";
import type { Language } from "./copy";

export { default as LookCarousel } from "../../src/studio/LookCarousel";

const installable = mods.filter((mod) => isInstallable(mod.id));
const looks = installable.filter((mod) => mod.domain === "wardrobe");
const tuning = installable.filter((mod) => mod.domain === "game");

export const looksCount = looks.length;
export const tuningCount = tuning.length;

/** Tuning mods by their category, in catalog order. */
export function tuningGroups(language: Language) {
  const groups = new Map<string, { title: string; names: string[] }>();
  for (const mod of tuning) {
    const group = groups.get(mod.category) ?? { title: categoryLabel(mod.category, language), names: [] };
    group.names.push(mod.name[language]);
    groups.set(mod.category, group);
  }
  return [...groups.entries()].map(([id, group]) => ({ id, ...group }));
}

/** A few previews for a tile, picked at random each visit. */
export function samplePreviews(domain: "wardrobe" | "game", count: number) {
  const pool = (domain === "wardrobe" ? looks : tuning).filter((mod) => mod.image);
  const picked: string[] = [];
  const used = new Set<number>();
  while (picked.length < Math.min(count, pool.length)) {
    const index = Math.floor(Math.random() * pool.length);
    if (used.has(index)) continue;
    used.add(index);
    picked.push(pool[index].image as string);
  }
  return picked;
}
