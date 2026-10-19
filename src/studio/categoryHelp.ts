import type { Language } from "../i18n";

type Text = Record<Language, string>;

/**
 * Honest per-category help. `installs` says what the pack changes, from the
 * folders the audited archives actually write to; `tip` is an upstream
 * troubleshooting fact and nothing more (the Dota2PornFx wiki,
 * d2pfxwiki.pages.dev/en/troubleshoot.html). Nothing here claims a pack works
 * in the game: no item has been run in Dota through BetterFy yet.
 */
export type CategoryHelp = {
  installs: Text;
  tip?: Text;
};

const EMBLEM_TIP: Text = {
  ru: "Если эмблема не видна: откройте консоль до матча и выполните r_draw_selected_ring 0, затем r_draw_selected_ring 1. Проверьте, что мод «Misc Optimization» в Minify выключен. (Совет авторов проекта Dota2PornFx.)",
  en: "If an emblem does not show: open the console before the match and run r_draw_selected_ring 0, then r_draw_selected_ring 1. Make sure Minify's Misc Optimization mod is not on. (Advice from the Dota2PornFx project.)",
};

const SHADER_TIP: Text = {
  ru: "Если шейдер выглядит в клетку, выключите настройку графики World Lighting. (Совет авторов проекта Dota2PornFx.)",
  en: "If a shader looks checkered, turn off the World Lighting graphics setting. (Advice from the Dota2PornFx project.)",
};

const help = (ru: string, en: string, tip?: Text): CategoryHelp => ({
  installs: { ru, en },
  ...(tip ? { tip } : {}),
});

const categories: Record<string, CategoryHelp> = {
  heroes: help(
    "Заменяет модели, текстуры и эффекты героя. Файлы ставятся так, как их собрал автор.",
    "Replaces a hero's models, textures and effects. Files install as the author made them.",
  ),
  "hero-items": help(
    "Заменяет модели, текстуры и эффекты предметов героя.",
    "Replaces models, textures and effects of a hero's items.",
  ),
  ancient: help(
    "Заменяет модели и эффекты главных зданий (Древних).",
    "Replaces models and effects of the Ancients (the main buildings).",
  ),
  announcers: help(
    "Только описания звуковых событий: они выбирают, какие звуки играть, и не содержат аудио.",
    "Sound-event definitions only: they choose which sounds play and carry no audio of their own.",
  ),
  couriers: help(
    "Заменяет модель, материалы и эффекты курьера.",
    "Replaces a courier's model, materials and effects.",
  ),
  "creep-deny": help(
    "Заменяет эффект, который показывается при добивании своего крипа.",
    "Replaces the effect shown when you deny a creep.",
  ),
  creeps: help(
    "Заменяет модели, анимации и текстуры крипов. Пакеты бывают тяжёлыми.",
    "Replaces creep models, animations and textures. Packs can be large.",
  ),
  emblems: help(
    "Заменяет эффекты эмблемы под выделенным героем.",
    "Replaces the emblem effects under a selected unit.",
    EMBLEM_TIP,
  ),
  "hero-sounds": help(
    "Заменяет звуки оружия и атак героев.",
    "Replaces heroes' weapon and attack sounds.",
  ),
  herofx: help(
    "Заменяет эффекты героев (частицы, несколько значков и звуков).",
    "Replaces hero effects (particles, a few icons and sounds).",
  ),
  "high-five": help(
    "Заменяет эффекты и модели High Five.",
    "Replaces High Five effects and models.",
  ),
  "item-effects": help(
    "Заменяет эффекты предметов (частицы, материалы, текстуры).",
    "Replaces item effects (particles, materials, textures).",
  ),
  "item-icons": help(
    "Меняет только значки предметов в интерфейсе (текстуры Panorama).",
    "Changes item icons in the interface only (Panorama textures).",
  ),
  "mega-kill": help(
    "Заменяет голосовые реплики серий убийств и их описания звуковых событий.",
    "Replaces kill-streak voice lines and their sound-event definitions.",
  ),
  music: help(
    "Заменяет музыкальные треки. Пакеты очень большие.",
    "Replaces music tracks. Packs are very large.",
  ),
  optimization: help(
    "Заменяет модели и эффекты на версии из пакета.",
    "Replaces models and effects with the pack's own versions.",
  ),
  pings: help(
    "Заменяет эффекты пингов на карте.",
    "Replaces map ping effects.",
  ),
  "ranged-attack": help(
    "Заменяет эффекты дальних атак и снарядов.",
    "Replaces ranged attack and projectile effects.",
  ),
  ranks: help(
    "Меняет только значки рангов в интерфейсе (текстуры Panorama).",
    "Changes rank icons in the interface only (Panorama textures).",
  ),
  river: help(
    "Заменяет материалы воды в реке.",
    "Replaces the river's water materials.",
  ),
  roshan: help(
    "Заменяет модель и материалы Рошана.",
    "Replaces Roshan's model and materials.",
  ),
  shaders: help(
    "Заменяет материалы и текстуры шейдеров.",
    "Replaces shader materials and textures.",
    SHADER_TIP,
  ),
  sounds: help(
    "Заменяет звуковые файлы (музыку, голоса, окружение).",
    "Replaces sound files (music, voices, ambience).",
  ),
  terrains: help(
    "Заменяет материалы, модели и текстуры ландшафта. Целиком карту (maps/dota.vpk) этот пакет не меняет.",
    "Replaces terrain materials, models and textures. It does not replace the map itself (maps/dota.vpk).",
  ),
  "ti-bp-effects": help(
    "Заменяет эффекты из наборов TI и боевого пропуска.",
    "Replaces effects from TI and Battle Pass sets.",
  ),
  towers: help("Заменяет модель башни.", "Replaces a tower's model."),
  trees: help(
    "Заменяет модели, материалы и текстуры деревьев. Пакеты бывают тяжёлыми.",
    "Replaces tree models, materials and textures. Packs can be large.",
  ),
  wards: help("Заменяет модели вардов.", "Replaces ward models."),
};

const fallback: CategoryHelp = help(
  "Заменяет файлы игры файлами из пакета.",
  "Replaces game files with the pack's files.",
);

export function categoryHelp(category: string): CategoryHelp {
  return categories[category] ?? fallback;
}

/** True for categories whose archives BetterFy installs (the audited allowlist). */
export const enabledCategories: ReadonlySet<string> = new Set(Object.keys(categories));

const PANORAMA: Text = {
  ru: "Не включено: это макеты и стили Panorama. Для них нужно отдельное подтверждение безопасности, оно ещё готовится.",
  en: "Not enabled: these are Panorama layouts and styles. They need a separate safety proof, which is still being built.",
};

const notEnabled: Record<string, Text> = {
  backgrounds: PANORAMA,
  huds: PANORAMA,
  "versus-screens": PANORAMA,
  cursors: {
    ru: "Не включено: это отдельные файлы без VPK, и проверенного места установки для них пока нет.",
    en: "Not enabled: these are loose files with no VPK, and there is no verified install target for them yet.",
  },
  fonts: {
    ru: "Не включено: это отдельные файлы без VPK, и проверенного места установки для них пока нет.",
    en: "Not enabled: these are loose files with no VPK, and there is no verified install target for them yet.",
  },
  pedestal: {
    ru: "Недоступно: файлов этого набора нет ни на одном из зеркал источника.",
    en: "Unavailable: the files of this item exist on neither of the source's mirrors.",
  },
  tormentor: {
    ru: "Недоступно: файлов этого набора нет ни на одном из зеркал источника.",
    en: "Unavailable: the files of this item exist on neither of the source's mirrors.",
  },
};

/**
 * Why a catalog card cannot be installed, or null when its category is
 * enabled. The authority is the audited allowlist (`pilotCatalogIds`); this
 * only words the reason, so a card the allowlist lacks for another reason
 * (blocked or too large to audit) falls through to null.
 */
export function notEnabledReason(category: string): Text | null {
  return notEnabled[category] ?? null;
}
