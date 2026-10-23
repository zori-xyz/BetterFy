import type { GameLanguage } from "../../engine";

// Every language a build can be installed for. Names follow Steam's own
// language table; "verified" means the founder saw mods load in game on
// Windows through that slot.

export type BuildLanguageOption = {
  value: GameLanguage;
  ru: string;
  en: string;
  native: string;
  verified: boolean;
  noteRu: string;
  noteEn: string;
};

type Details = Omit<BuildLanguageOption, "value">;

/** The engine package that adds the player's own English text to the Dutch slot. */
export const ENGLISH_TEXT_ID = "betterfy.english-text";

const untestedRu = "Текст Dota сменится на этот язык.";
const untestedEn = "Dota text switches to this language.";

const untested = (ru: string, en: string, native: string): Details => ({
  ru,
  en,
  native,
  verified: false,
  noteRu: untestedRu,
  noteEn: untestedEn,
});

// A Record, so a language added to the engine cannot be forgotten here.
const details: Record<GameLanguage, Details> = {
  betterfy: {
    ru: "Английский (своя папка)",
    en: "English (own folder)",
    native: "English",
    verified: false,
    noteRu:
      "Для экспертов. Свою папку BetterFy текущая Dota не принимает: сборка встаёт в файлы, но в игре ничего не меняется. Для английской Dota выбери «Как в Dota сейчас».",
    noteEn:
      "For experts. The current Dota does not accept BetterFy's own folder: the build installs but nothing changes in game. For an English Dota choose \"Same as Dota now\".",
  },
  russian: {
    ru: "Русский",
    en: "Russian",
    native: "Русский",
    verified: true,
    noteRu: "Проверен на Windows: сборка из 24 модов работала в игре и откатилась начисто.",
    noteEn: "Checked on Windows: a 24-mod build worked in game and restored cleanly.",
  },
  dutch: {
    ru: "Нидерландский",
    en: "Dutch",
    native: "Nederlands",
    verified: true,
    noteRu: "Проверен на Windows, откат тоже. Текст Dota станет нидерландским.",
    noteEn: "Checked on Windows, restore included. Dota text switches to Dutch.",
  },
  brazilian: untested("Португальский (Бразилия)", "Portuguese (Brazil)", "Português-Brasil"),
  bulgarian: untested("Болгарский", "Bulgarian", "български език"),
  czech: untested("Чешский", "Czech", "čeština"),
  danish: untested("Датский", "Danish", "Dansk"),
  finnish: untested("Финский", "Finnish", "Suomi"),
  french: untested("Французский", "French", "Français"),
  german: untested("Немецкий", "German", "Deutsch"),
  greek: untested("Греческий", "Greek", "Ελληνικά"),
  hungarian: untested("Венгерский", "Hungarian", "Magyar"),
  italian: untested("Итальянский", "Italian", "Italiano"),
  japanese: untested("Японский", "Japanese", "日本語"),
  koreana: {
    ...untested("Корейский", "Korean", "한국어"),
    noteRu: "Текст Dota сменится на корейский. По данным Steam, у Dota полная корейская озвучка.",
    noteEn: "Dota text switches to Korean. Steam lists full Korean voice-over for Dota.",
  },
  latam: untested(
    "Испанский (Латинская Америка)",
    "Spanish (Latin America)",
    "Español-Latinoamérica",
  ),
  norwegian: untested("Норвежский", "Norwegian", "Norsk"),
  polish: untested("Польский", "Polish", "Polski"),
  portuguese: untested("Португальский (Португалия)", "Portuguese (Portugal)", "Português"),
  romanian: untested("Румынский", "Romanian", "Română"),
  schinese: {
    ...untested("Китайский (упрощённый)", "Chinese (Simplified)", "简体中文"),
    noteRu:
      "Текст Dota сменится на китайский. По данным Steam, у Dota полная озвучка на упрощённом китайском.",
    noteEn:
      "Dota text switches to Chinese. Steam lists full Simplified Chinese voice-over for Dota.",
  },
  spanish: untested("Испанский (Испания)", "Spanish (Spain)", "Español-España"),
  swedish: untested("Шведский", "Swedish", "Svenska"),
  tchinese: untested("Китайский (традиционный)", "Chinese (Traditional)", "繁體中文"),
  thai: untested("Тайский", "Thai", "ไทย"),
  turkish: untested("Турецкий", "Turkish", "Türkçe"),
  ukrainian: untested("Украинский", "Ukrainian", "Українська"),
  vietnamese: untested("Вьетнамский", "Vietnamese", "Tiếng Việt"),
};

const pinned: GameLanguage[] = ["betterfy", "russian", "dutch"];

// English first, then the two slots checked on Windows, then the rest by
// English name.
export const buildLanguages: BuildLanguageOption[] = [
  ...pinned.map((value) => ({ value, ...details[value] })),
  ...(Object.keys(details) as GameLanguage[])
    .filter((value) => !pinned.includes(value))
    .map((value) => ({ value, ...details[value] }))
    .sort((left, right) => left.en.localeCompare(right.en, "en")),
];

export function languageLabel(value: GameLanguage, isRu: boolean): string {
  const option = details[value];
  return option ? (isRu ? option.ru : option.en) : value;
}
