import snapshot from "./minifyCatalog.json";
import type { Language } from "./i18n";

export type MinifyCategory = "all" | "core" | "world" | "interface" | "audio" | "custom" | "utility";

export type MinifyCatalogItem = {
  id: string;
  sourceName: string;
  name: Record<Language, string>;
  description: Record<Language, string>;
  category: Exclude<MinifyCategory, "all">;
  author: string;
  preview: string | null;
  sourcePath: string;
  evidence: {
    blacklistEntries: number;
    includedFiles: number;
    stylingRules: number;
    scripts: number;
  };
};

const data = snapshot as {
  source: string;
  commit: string;
  snapshotDate: string;
  items: MinifyCatalogItem[];
};

export const minifySource = {
  url: data.source,
  commit: data.commit,
  snapshotDate: data.snapshotDate,
};

export const minifyMods = data.items;

export const minifyCategoryLabels: Record<MinifyCategory, Record<Language, string>> = {
  all: { ru: "Все", en: "All" },
  core: { ru: "Оптимизация", en: "Optimization" },
  world: { ru: "Карта и мир", en: "Map & world" },
  interface: { ru: "HUD и меню", en: "HUD & menus" },
  audio: { ru: "Звук", en: "Audio" },
  custom: { ru: "Свои настройки", en: "Customization" },
  utility: { ru: "Инструменты", en: "Utilities" },
};

export const minifyPreviewUrl = (preview: string | null) =>
  preview ? new URL(`./assets/minify/${preview}`, import.meta.url).href : null;
