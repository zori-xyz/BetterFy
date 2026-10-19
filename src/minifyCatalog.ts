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

/**
 * Minify's own descriptions talk about Minify's settings, its config folder
 * and the Workshop Tools. Where BetterFy installs a mod differently, the card
 * says how it works here.
 */
const descriptions: Record<string, Record<Language, string>> = {
  "minify-auto-accept-match": {
    ru: "Автоматически принимает найденный матч через заданную задержку. Задержка от 0 до 40 секунд настраивается ползунком в настройках Dota, в разделе Minify. 40 выключает автопринятие.",
    en: "Accepts a found match automatically after a delay. The delay, from 0 to 40 seconds, is set with a slider in Dota's settings, in the Minify section. 40 turns it off.",
  },
  "minify-user-styles": {
    ru: "Набор небольших правок интерфейса от автора Minify: убрать портрет героя, баннеры, анимации и затемнения. Отметь нужные ниже — в сборку попадут только они.",
    en: "A set of small interface tweaks from Minify's author: hide the hero portrait, banners, animations and dims. Tick the ones you want below; only those go into the build.",
  },
  "minify-opendotaguides-guides": {
    ru: "Ставит гайды предметов OpenDotaGuides вместо стандартных: для каждого героя — сборка из свежих матчей. Откат возвращает стандартные гайды.",
    en: "Installs OpenDotaGuides item build guides in place of the default ones: a build from recent matches for every hero. Restore brings the default guides back.",
  },
};

export const minifyMods = data.items.map((item) =>
  descriptions[item.id] ? { ...item, description: descriptions[item.id] } : item,
);

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
