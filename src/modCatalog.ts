import snapshot from "./webCatalog.json";
import type { Language } from "./i18n";

type RawLink = {
  type?: string;
  url?: string;
};

type RawStyle = {
  label?: string;
  color?: string;
  preview?: string;
  file?: string;
};

type RawCatalogItem = {
  id?: string;
  name?: string;
  category?: string;
  group?: string | null;
  preview?: string | null;
  file?: string | null;
  type?: string;
  tags?: Record<string, boolean | undefined>;
  links?: RawLink[];
  date?: number | null;
  styles?: RawStyle[];
};

export type ModCatalogGroup = "all" | "characters" | "interface" | "audio" | "world" | "effects" | "extras";

export type CatalogMetadata = {
  name: string;
  category: string;
  group: Exclude<ModCatalogGroup, "all">;
  categoryLabel: Record<Language, string>;
  tags: string[];
  styleCount: number;
};

export type CatalogPresentation = {
  description: Record<Language, string>;
  previewUrl: string | null;
};

export type CatalogArtifact = {
  fileName: string | null;
  fileKind: "VPK" | "ZIP" | "Archive";
  downloadUrl: null;
  byteSize: null;
  sha256: null;
};

export type CatalogProvenance = {
  author: string | null;
  sourceUrl: string;
  sourceName: string;
  snapshotDate: string;
  permission: "unknown";
};

export type CatalogVerification = {
  updatedAt: number | null;
  compatibility: "unknown";
  lastVerifiedAt: null;
  trustRationale: string;
};

export type BetterFyCatalogMod = {
  id: string;
  metadata: CatalogMetadata;
  presentation: CatalogPresentation;
  artifact: CatalogArtifact;
  provenance: CatalogProvenance;
  verification: CatalogVerification;
};

const PREVIEW_ROOT =
  "https://raw.githubusercontent.com/h6rd/Dota2PornFxWeb/main/assets/previews/";

const groupCategories: Record<Exclude<ModCatalogGroup, "all">, Set<string>> = {
  characters: new Set(["heroes", "hero-items", "couriers"]),
  interface: new Set([
    "backgrounds",
    "cursors",
    "emblems",
    "fonts",
    "huds",
    "item-icons",
    "pings",
    "ranks",
    "versus-screens",
  ]),
  audio: new Set(["announcers", "hero-sounds", "mega-kill", "music", "sounds"]),
  world: new Set([
    "ancient",
    "creep-deny",
    "creeps",
    "high-five",
    "pedestal",
    "river",
    "roshan",
    "terrains",
    "tormentor",
    "towers",
    "trees",
    "wards",
  ]),
  effects: new Set(["optimization", "shaders", "item-effects", "herofx", "ti-bp-effects", "ranged-attack"]),
  extras: new Set(["other", "tools", "news"]),
};

export const wardrobeCategoryLabels: Record<string, Record<Language, string>> = {
  heroes: { ru: "Герои", en: "Heroes" },
  "hero-items": { ru: "Предметы героев", en: "Hero items" },
  couriers: { ru: "Курьеры", en: "Couriers" },
  "item-effects": { ru: "Эффекты предметов", en: "Item effects" },
  herofx: { ru: "Эффекты героев", en: "Hero effects" },
  "ti-bp-effects": { ru: "Эффекты TI и BP", en: "TI & BP effects" },
  backgrounds: { ru: "Фоны", en: "Backgrounds" },
  cursors: { ru: "Курсоры", en: "Cursors" },
  emblems: { ru: "Эмблемы", en: "Emblems" },
  fonts: { ru: "Шрифты", en: "Fonts" },
  huds: { ru: "HUD", en: "HUD" },
  "item-icons": { ru: "Иконки предметов", en: "Item icons" },
  pings: { ru: "Пинги", en: "Pings" },
  ranks: { ru: "Ранги", en: "Ranks" },
  "versus-screens": { ru: "Versus-экраны", en: "Versus screens" },
  announcers: { ru: "Дикторы", en: "Announcers" },
  "hero-sounds": { ru: "Реплики героев", en: "Hero sounds" },
  "mega-kill": { ru: "Mega-kill", en: "Mega-kill" },
  music: { ru: "Музыка", en: "Music" },
  sounds: { ru: "Звуки", en: "Sounds" },
  ancient: { ru: "Древние", en: "Ancients" },
  "creep-deny": { ru: "Добивание крипов", en: "Creep deny" },
  creeps: { ru: "Крипы", en: "Creeps" },
  "high-five": { ru: "High Five", en: "High Five" },
  pedestal: { ru: "Пьедесталы", en: "Pedestals" },
  "ranged-attack": { ru: "Атаки", en: "Ranged attacks" },
  river: { ru: "Река", en: "River" },
  roshan: { ru: "Рошан", en: "Roshan" },
  terrains: { ru: "Ландшафты", en: "Terrains" },
  tormentor: { ru: "Терзатели", en: "Tormentors" },
  towers: { ru: "Башни", en: "Towers" },
  trees: { ru: "Деревья", en: "Trees" },
  wards: { ru: "Варды", en: "Wards" },
  optimization: { ru: "Оптимизация", en: "Optimization" },
  shaders: { ru: "Шейдеры", en: "Shaders" },
  other: { ru: "Другое", en: "Other" },
  tools: { ru: "Инструменты", en: "Tools" },
  news: { ru: "Новости", en: "News" },
};

const groupForCategory = (category: string) =>
  (Object.entries(groupCategories).find(([, categories]) => categories.has(category))?.[0] ??
    null) as Exclude<ModCatalogGroup, "all"> | null;

const clean = (value: unknown, fallback = "") =>
  typeof value === "string" ? value.replace(/[\u0000-\u001f]/g, " ").trim().slice(0, 180) : fallback;

const authorFrom = (links: RawLink[] | undefined) => {
  const link = links?.find((candidate) => candidate.type === "author" || candidate.type === "modded");
  if (!link?.url) return null;
  const value = clean(link.url);
  if (!value) return null;
  if (!/^https?:\/\//i.test(value)) return value;
  try {
    const url = new URL(value);
    return url.pathname.split("/").filter(Boolean).at(-1) ?? url.hostname;
  } catch {
    return value;
  }
};

const fileKind = (fileName: string | null): CatalogArtifact["fileKind"] => {
  if (fileName?.toLowerCase().endsWith(".vpk")) return "VPK";
  if (fileName?.toLowerCase().endsWith(".zip")) return "ZIP";
  return "Archive";
};

const describe = (
  name: string,
  category: string,
  group: CatalogMetadata["group"],
): Record<Language, string> => {
  const label = wardrobeCategoryLabels[category] ?? { ru: category, en: category };
  if (group === "interface") {
    return {
      ru: `${name} меняет раздел «${label.ru}». BetterFy покажет исходное превью и сохранит мод отдельным элементом сборки.`,
      en: `${name} changes ${label.en}. BetterFy keeps the source preview and adds it as a separate build item.`,
    };
  }
  if (group === "audio") {
    return {
      ru: `${name} заменяет звуковое оформление категории «${label.ru}». Послушать результат и проверить совместимость нужно до сборки.`,
      en: `${name} replaces audio in ${label.en}. Preview the result and verify compatibility before building.`,
    };
  }
  if (group === "world") {
    return {
      ru: `${name} меняет визуальную часть мира Dota 2: ${label.ru.toLowerCase()}. BetterFy не смешивает его со скинами героев.`,
      en: `${name} changes the Dota 2 world layer: ${label.en.toLowerCase()}. BetterFy keeps it separate from hero skins.`,
    };
  }
  return {
    ru: `${name} относится к категории «${label.ru}». Эффект добавляется в сборку отдельно, а совместимость пока не подтверждена.`,
    en: `${name} belongs to ${label.en}. The effect is added separately and compatibility is not yet verified.`,
  };
};

const catalogSnapshot = snapshot as unknown as {
  snapshot: string;
  source: string;
  items: RawCatalogItem[];
};

const rawItems = catalogSnapshot.items;

export const wardrobeCatalogItems: BetterFyCatalogMod[] = rawItems
  .map((item) => {
    const id = clean(item.id);
    const name = clean(item.name);
    const category = clean(item.category);
    const group = groupForCategory(category);
    if (!id || !name || !group || item.type !== "mod") return null;

    const preview = clean(item.preview);
    const fileName = clean(item.file) || null;
    return {
      id,
      metadata: {
        name,
        category,
        group,
        categoryLabel: wardrobeCategoryLabels[category] ?? { ru: category, en: category },
        tags: Object.entries(item.tags ?? {})
          .filter(([, enabled]) => enabled)
          .map(([tag]) => tag),
        styleCount: Array.isArray(item.styles) ? item.styles.length : 0,
      },
      presentation: {
        description: describe(name, category, group),
        previewUrl: preview
          ? `${PREVIEW_ROOT}${encodeURIComponent(category)}/${encodeURIComponent(preview)}`
          : null,
      },
      artifact: {
        fileName,
        fileKind: fileKind(fileName),
        downloadUrl: null,
        byteSize: null,
        sha256: null,
      },
      provenance: {
        author: authorFrom(item.links),
        sourceUrl: catalogSnapshot.source,
        sourceName: "Dota2PornFxWeb",
        snapshotDate: catalogSnapshot.snapshot,
        permission: "unknown" as const,
      },
      verification: {
        updatedAt: typeof item.date === "number" ? item.date : null,
        compatibility: "unknown" as const,
        lastVerifiedAt: null,
        trustRationale: "Imported upstream metadata; artifact and compatibility are not verified.",
      },
    };
  })
  .filter((item): item is BetterFyCatalogMod => item !== null)
  .sort((left, right) => (right.verification.updatedAt ?? 0) - (left.verification.updatedAt ?? 0));

export const modGroupLabels: Record<ModCatalogGroup, Record<Language, string>> = {
  all: { ru: "Все моды", en: "All mods" },
  characters: { ru: "Герои", en: "Heroes" },
  interface: { ru: "Интерфейс", en: "Interface" },
  audio: { ru: "Звук", en: "Audio" },
  world: { ru: "Мир", en: "World" },
  effects: { ru: "Эффекты", en: "Effects" },
  extras: { ru: "Другое", en: "Other" },
};
