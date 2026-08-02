import type { Language } from "./i18n";

export type ModCategory = "performance" | "interface" | "audio" | "visual";

type LocalizedText = {
  ru: string;
  en: string;
};

export type CatalogMod = {
  id: string;
  name: LocalizedText;
  shortDescription: LocalizedText;
  description: LocalizedText;
  result: LocalizedText;
  installHint: LocalizedText;
  category: ModCategory;
  footprint: string;
  kind: "VPK" | "Panorama" | "Audio";
  version: string;
  updated: LocalizedText;
  author: string;
  featured?: boolean;
  conflictsWith?: string[];
  tags: LocalizedText[];
  accent: "mint" | "sky" | "gold" | "coral" | "violet";
};

export const categoryLabels: Record<ModCategory, LocalizedText> = {
  performance: { ru: "Оптимизация", en: "Performance" },
  interface: { ru: "Интерфейс", en: "Interface" },
  audio: { ru: "Звук", en: "Audio" },
  visual: { ru: "Визуал", en: "Visuals" },
};

export const catalog: CatalogMod[] = [
  {
    id: "base-attacks",
    name: { ru: "Чистые атаки", en: "Clean Attacks" },
    shortDescription: {
      ru: "Меньше шума от базовых атак и снарядов.",
      en: "Less visual noise from basic attacks and projectiles.",
    },
    description: {
      ru: "Смягчает самые яркие следы снарядов и вспышки попаданий, сохраняя направление атаки и игровые сигналы.",
      en: "Softens the brightest projectile trails and impact flashes while preserving attack direction and gameplay cues.",
    },
    result: {
      ru: "В драке легче следить за своим героем и летящими снарядами.",
      en: "Projectiles and your own hero are easier to track during fights.",
    },
    installHint: {
      ru: "BetterFy заменит только частицы атак внутри отдельного VPK.",
      en: "BetterFy replaces only attack particles inside an isolated VPK.",
    },
    category: "performance",
    footprint: "24 MB",
    kind: "VPK",
    version: "2.4.1",
    updated: { ru: "обновлён сегодня", en: "updated today" },
    author: "BetterFy Lab",
    featured: true,
    tags: [
      { ru: "Читаемость", en: "Clarity" },
      { ru: "Без FPS-потерь", en: "No FPS loss" },
    ],
    accent: "mint",
  },
  {
    id: "spell-clarity",
    name: { ru: "Чистые способности", en: "Spell Clarity" },
    shortDescription: {
      ru: "Облегчённые эффекты без потери информации.",
      en: "Lighter spell effects without losing information.",
    },
    description: {
      ru: "Убирает вторичные блики и дым из популярных способностей. Основная форма, радиус и момент применения остаются заметными.",
      en: "Removes secondary bloom and smoke from popular spells. Core shape, radius, and cast timing stay visible.",
    },
    result: {
      ru: "В массовой драке проще различать радиус и момент применения способности.",
      en: "Spell radius and cast timing are easier to read during crowded fights.",
    },
    installHint: {
      ru: "Эффекты собираются поверх оригиналов и легко отключаются.",
      en: "Effects are layered over originals and can be disabled instantly.",
    },
    category: "performance",
    footprint: "41 MB",
    kind: "VPK",
    version: "1.8.0",
    updated: { ru: "3 дня назад", en: "3 days ago" },
    author: "Mira",
    tags: [
      { ru: "Тимфайты", en: "Team fights" },
      { ru: "+FPS", en: "+FPS" },
    ],
    accent: "mint",
  },
  {
    id: "quiet-world",
    name: { ru: "Тихий мир", en: "Quiet World" },
    shortDescription: {
      ru: "Чище звук карты, важные сигналы на месте.",
      en: "A cleaner soundscape with key cues intact.",
    },
    description: {
      ru: "Снижает громкость фоновых звуков карты, декоративных существ и окружения, не затрагивая способности и уведомления.",
      en: "Lowers ambient map sounds, decorative creatures, and environmental noise without touching spells or notifications.",
    },
    result: {
      ru: "Лучше слышны телепорты, способности и команды.",
      en: "Teleports, abilities, and voice comms become easier to hear.",
    },
    installHint: {
      ru: "Звуковой профиль можно менять без пересборки остальных модов.",
      en: "The audio profile can be changed without rebuilding other mods.",
    },
    category: "audio",
    footprint: "6 MB",
    kind: "Audio",
    version: "3.1.2",
    updated: { ru: "вчера", en: "yesterday" },
    author: "BetterFy Lab",
    tags: [
      { ru: "Фокус", en: "Focus" },
      { ru: "Аудио", en: "Audio" },
    ],
    accent: "gold",
  },
  {
    id: "transparent-hud",
    name: { ru: "Воздушный HUD", en: "Airy HUD" },
    shortDescription: {
      ru: "Больше пространства игре, меньше тяжёлых панелей.",
      en: "More room for the game, fewer heavy panels.",
    },
    description: {
      ru: "Пересобирает нижнюю панель, делает фон легче и усиливает контраст ключевых показателей: здоровья, маны и кулдаунов.",
      en: "Rebuilds the bottom bar, lightens its background, and strengthens key health, mana, and cooldown indicators.",
    },
    result: {
      ru: "HUD занимает меньше места, а здоровье, мана и кулдауны остаются заметными.",
      en: "The HUD takes up less space while health, mana, and cooldowns remain easy to see.",
    },
    installHint: {
      ru: "Panorama-файлы проверяются после каждого обновления Dota 2.",
      en: "Panorama files are verified after every Dota 2 update.",
    },
    category: "interface",
    footprint: "18 MB",
    kind: "Panorama",
    version: "0.9.7",
    updated: { ru: "обновлён сегодня", en: "updated today" },
    author: "BetterFy Lab",
    featured: true,
    conflictsWith: ["compact-draft"],
    tags: [
      { ru: "HUD", en: "HUD" },
      { ru: "16:9 · 21:9", en: "16:9 · 21:9" },
    ],
    accent: "sky",
  },
  {
    id: "compact-draft",
    name: { ru: "Компактный драфт", en: "Compact Draft" },
    shortDescription: {
      ru: "Плотный и спокойный экран выбора героев.",
      en: "A denser, calmer hero selection screen.",
    },
    description: {
      ru: "Уменьшает вторичные панели во время драфта и собирает ключевую информацию ближе к выбранному герою.",
      en: "Reduces secondary draft panels and brings essential information closer to the selected hero.",
    },
    result: {
      ru: "Выбор героя быстрее считывается на любом экране.",
      en: "Hero selection reads faster on any display.",
    },
    installHint: {
      ru: "Использует альтернативный набор Panorama-компонентов.",
      en: "Uses an alternative set of Panorama components.",
    },
    category: "interface",
    footprint: "12 MB",
    kind: "Panorama",
    version: "1.2.3",
    updated: { ru: "5 дней назад", en: "5 days ago" },
    author: "Nullframe",
    conflictsWith: ["transparent-hud"],
    tags: [
      { ru: "Драфт", en: "Draft" },
      { ru: "Компактно", en: "Compact" },
    ],
    accent: "sky",
  },
  {
    id: "dark-terrain",
    name: { ru: "Сумрачный ландшафт", en: "Twilight Terrain" },
    shortDescription: {
      ru: "Тёмная карта с ясным контрастом героев.",
      en: "A darker map with clear hero contrast.",
    },
    description: {
      ru: "Авторская цветокоррекция окружения BetterFy: глубокие тени, холодная река и мягкие источники света без потери видимости.",
      en: "BetterFy's authored environment grade: deep shadows, a cooler river, and soft light sources without reduced visibility.",
    },
    result: {
      ru: "Карта становится темнее, но герои и игровые объекты сохраняют контраст.",
      en: "The map becomes darker while heroes and game objects keep their contrast.",
    },
    installHint: {
      ru: "Ландшафт хранится отдельно и не перезаписывает оригинальную карту.",
      en: "Terrain stays isolated and never overwrites the original map.",
    },
    category: "visual",
    footprint: "67 MB",
    kind: "VPK",
    version: "1.0.4",
    updated: { ru: "новинка", en: "new" },
    author: "BetterFy Production",
    featured: true,
    tags: [
      { ru: "Карта", en: "Terrain" },
      { ru: "BetterFy Production", en: "BetterFy Production" },
    ],
    accent: "coral",
  },
];

export const localizeMod = (mod: CatalogMod, language: Language) => ({
  ...mod,
  name: mod.name[language],
  shortDescription: mod.shortDescription[language],
  description: mod.description[language],
  result: mod.result[language],
  installHint: mod.installHint[language],
  updated: mod.updated[language],
  tags: mod.tags.map((tag) => tag[language]),
  categoryLabel: categoryLabels[mod.category][language],
});

export type LocalizedMod = ReturnType<typeof localizeMod>;
