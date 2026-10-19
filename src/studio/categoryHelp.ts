import type { Language } from "../i18n";

type Text = Record<Language, string>;

/**
 * Honest per-category help. `installs` says what the pack changes, from the
 * folders the audited archives actually write to; `tip` is an upstream
 * troubleshooting fact and nothing more (the Dota2PornFx wiki,
 * d2pfxwiki.pages.dev/en/troubleshoot.html).
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
  backgrounds: help(
    "Заменяет фон главного меню: макеты, стили и видео главного экрана.",
    "Replaces the main-menu background: the layouts, styles and video of the home screen.",
  ),
  huds: help(
    "Заменяет стили игрового интерфейса (HUD) и его текстуры.",
    "Replaces the in-game HUD styles and its textures.",
  ),
  "versus-screens": help(
    "Заменяет макет экрана «Versus» (экран перед матчем).",
    "Replaces the versus screen layout (the pre-match screen).",
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
    "Заменяет материалы, модели и текстуры ландшафта, не меняя саму карту.",
    "Replaces terrain materials, models and textures without changing the map itself.",
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
  cursors: help(
    "Заменяет курсор мыши в игре: файлы из папки курсоров Dota. Откат возвращает стандартный курсор.",
    "Replaces the in-game mouse cursor: the files of Dota's cursor folder. Restore brings the standard cursor back.",
  ),
  fonts: help(
    "Меняет шрифты интерфейса Dota, как велит инструкция пака: шрифты игры убираются, шрифты пака встают на их место. Кириллица есть. Откат возвращает шрифты игры.",
    "Changes Dota's interface fonts as the pack's guide says: the game's fonts are taken out and the pack's take their place. Cyrillic is included. Restore brings the game's fonts back.",
  ),
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

const notEnabled: Record<string, Text> = {
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

/**
 * What the Panorama proofs established for the categories that carry layouts,
 * styles and video, and how to undo the install. Null for every other category.
 * It states proofs about the files, not a result in the game.
 */
export type PanoramaReview = { proved: Text; notProved: Text; undo: Text };

const PANORAMA_NOT_PROVED: Text = {
  ru: "Чего проверка не доказывает: что макет выглядит правильно или вообще загружается в текущей версии Dota, и что события Valve, на которые он ссылается, ведут себя так, как предполагалось при проверке. В Dota это через BetterFy не запускали. Файлы, которые проверка не смогла подтвердить, отброшены и перечислены ниже с точной причиной.",
  en: "What the check does not prove: that the layout looks right or even loads in the current Dota, or that the Valve events it names behave as assumed when it was reviewed. This has not been run in Dota through BetterFy. Files the check could not prove are dropped and listed below with the exact reason.",
};

const PANORAMA_UNDO: Text = {
  ru: "Как отменить: нажмите «Откатить сборку». BetterFy вернёт файлы игры из резервной копии и проверит результат по хэшам.",
  en: "How to undo: use Restore the game. BetterFy puts the game's files back from its backup and checks the result by hash.",
};

const panorama: Record<string, Text> = {
  backgrounds: {
    ru: "Что проверено: каждый макет разобран как дерево (только известные типы панелей Valve и атрибуты, без скриптов, внешних адресов и обращений к сети), обработчики событий совпадают с заранее просмотренными привязками Valve, каждый стиль прочитан как текст с закрытым набором свойств, видео проверено как контейнер WebM (кадры не декодировались) и устанавливается только если его проигрывает принятый макет.",
    en: "What was proven: each layout was read as a tree (only known Valve panel types and attributes, no scripts, outside addresses or network use), its event handlers match bindings of Valve's own reviewed in advance, each style was read as text with a closed set of properties, and the video was checked as a WebM container (no frame was decoded) and installs only if an accepted layout plays it.",
  },
  huds: {
    ru: "Что проверено: каждый стиль прочитан как текст с закрытым набором свойств, ссылки только на текстуры архива или Valve; текстуры ставятся как обычные скомпилированные ресурсы из panorama/images.",
    en: "What was proven: each style was read as text with a closed set of properties, with references only to textures of the archive or of Valve; textures install as ordinary compiled resources from panorama/images.",
  },
  "versus-screens": {
    ru: "Что проверено: каждый макет разобран как дерево (только известные типы панелей Valve и атрибуты, без скриптов, внешних адресов и обращений к сети); обработчики событий и подключённые скрипты совпадают с заранее просмотренными привязками и скриптами Valve.",
    en: "What was proven: each layout was read as a tree (only known Valve panel types and attributes, no scripts, outside addresses or network use); its event handlers and included scripts match Valve's own bindings and scripts reviewed in advance.",
  },
};

export function panoramaReview(category: string): PanoramaReview | null {
  const proved = panorama[category];
  return proved ? { proved, notProved: PANORAMA_NOT_PROVED, undo: PANORAMA_UNDO } : null;
}
