import { MAX_BUNDLE_PACKAGES, pilotCatalogIds } from "./packages";
import { wardrobeCatalogItems, wardrobeCategoryLabels } from "../modCatalog";
import { minifyMods, minifyCategoryLabels, minifyPreviewUrl, minifySource } from "../minifyCatalog";
import rawCatalog from "../webCatalog.json";
import type { Language } from "../i18n";

export type Domain = "wardrobe" | "game";
export const pilotModIds = pilotCatalogIds;

export function isPilotMod(id: string) {
  return pilotModIds.has(id);
}

export function deliveryLabel(status: "pilot" | "preview", language: Language) {
  if (status === "pilot") return language === "ru" ? "WINDOWS-ПИЛОТ" : "WINDOWS PILOT";
  return language === "ru" ? "ТОЛЬКО ПРЕВЬЮ" : "PREVIEW ONLY";
}

export function isPilotSelection(ids: string[]) {
  return (
    ids.length > 0 &&
    ids.length <= Math.min(pilotModIds.size, MAX_BUNDLE_PACKAGES) &&
    new Set(ids).size === ids.length &&
    ids.every(isPilotMod)
  );
}

export function getSelectionDelivery(ids: string[]) {
  const known = ids.flatMap((id) => (modById.has(id) ? [modById.get(id)!] : []));
  // Game mods and the audited wardrobe items install through the same engine;
  // pilotModIds holds exactly the ones the engine knows.
  const gameIds = known
    .filter((mod) => mod.domain === "game" || isPilotMod(mod.id))
    .map((mod) => mod.id);
  const previewCount = known.filter((mod) => !isPilotMod(mod.id)).length;
  return {
    gameIds,
    pilotReady: isPilotSelection(gameIds),
    previewCount,
  };
}

export type StudioMod = {
  id: string;
  domain: Domain;
  name: Record<Language, string>;
  category: string;
  categoryName: Record<Language, string>;
  group: string;
  description: Record<Language, string>;
  image: string | null;
  author: string;
  source: string;
  date: number;
  tags: string[];
  variants: { name: string; image: string | null; color: string }[];
  slot: string | null;
};
type RawItem = {
  id?: string;
  group?: string | null;
  category?: string;
  styles?: { label?: string; preview?: string; color?: string }[];
};
const rawById = new Map((rawCatalog.items as RawItem[]).map((item) => [item.id, item]));
const previewRoot = "https://raw.githubusercontent.com/h6rd/Dota2PornFxWeb/main/assets/previews/";
const singleSlots = new Set([
  "terrains",
  "river",
  "trees",
  "cursors",
  "huds",
  "fonts",
  "announcers",
  "music",
  "ranks",
  "roshan",
  "ancient",
  "towers",
]);

export const mods: StudioMod[] = [
  ...wardrobeCatalogItems.map((item): StudioMod => {
    const raw = rawById.get(item.id);
    return {
      id: item.id,
      domain: "wardrobe",
      name: { ru: item.metadata.name, en: item.metadata.name },
      category: item.metadata.category,
      categoryName: item.metadata.categoryLabel,
      group: item.metadata.group,
      description: item.presentation.description,
      image: item.presentation.previewUrl,
      // Cards without a named author show the catalog's public name; the
      // upstream project stays in provenance and behind "Open source".
      author: item.provenance.author ?? "BetterFy Web",
      source: item.provenance.sourceUrl,
      date: item.verification.updatedAt ?? 0,
      tags: item.metadata.tags,
      variants: (raw?.styles ?? []).map((style) => ({
        name: style.label ?? "Style",
        color: style.color ?? "#b3a6a0",
        image: style.preview
          ? `${previewRoot}${item.metadata.category}/${encodeURIComponent(style.preview)}`
          : item.presentation.previewUrl,
      })),
      slot: singleSlots.has(item.metadata.category)
        ? item.metadata.category
        : raw?.group
          ? `${item.metadata.category}:${raw.group}`
          : null,
    };
  }),
  ...minifyMods.map(
    (item): StudioMod => ({
      id: item.id,
      domain: "game",
      name: item.name,
      category: item.category,
      categoryName: minifyCategoryLabels[item.category],
      group: item.category,
      description: item.description,
      image: minifyPreviewUrl(item.preview),
      author: item.author,
      source: `${minifySource.url}/tree/${minifySource.commit}/${encodeURI(item.sourcePath)}`,
      date: 0,
      tags: [],
      variants: [],
      slot: null,
    }),
  ),
];
export const modById = new Map(mods.map((mod) => [mod.id, mod]));
export const wardrobeGroups = [
  { id: "all", ru: "Всё", en: "All" },
  { id: "characters", ru: "Герои", en: "Heroes" },
  { id: "world", ru: "Мир", en: "World" },
  { id: "effects", ru: "Эффекты", en: "Effects" },
  { id: "interface", ru: "Интерфейс", en: "Interface" },
  { id: "audio", ru: "Звук", en: "Audio" },
];
export const gameGroups = Object.entries(minifyCategoryLabels).map(([id, labels]) => ({
  id,
  ...labels,
}));
export const categoryLabel = (id: string, language: Language) =>
  wardrobeCategoryLabels[id]?.[language] ?? id;
export const featuredIds = [
  "heroes-juggernaut-arcana-purple",
  "heroes-shadow-fiend-white",
  "heroes-lina-crystal-empress",
  "terrains-ti6-immortal-gardens",
  "heroes-ice-phoenix",
  "couriers-onibi",
];
export const featuredMods = featuredIds.flatMap((id) =>
  modById.has(id) ? [modById.get(id)!] : [],
);
export function getConflicts(ids: string[]) {
  const slots = new Map<string, StudioMod[]>();
  ids.forEach((id) => {
    const mod = modById.get(id);
    if (mod?.slot) slots.set(mod.slot, [...(slots.get(mod.slot) ?? []), mod]);
  });
  return [...slots.values()].filter((items) => items.length > 1);
}
