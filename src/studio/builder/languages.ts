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

const untestedRu = "Ещё не проверен в игре на Windows. Текст Dota сменится на этот язык.";
const untestedEn = "Not checked in game on Windows yet. Dota text switches to this language.";

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
    ru: "Английский",
    en: "English",
    native: "English",
    verified: false,
    noteRu:
      "Своя папка BetterFy без перевода, поэтому Dota должна остаться на английском. Так делал Minify до патча Dota 23.07.2026; после него способ не проверен.",
    noteEn:
      "BetterFy's own folder with no translation, so Dota should stay in English. Minify worked this way before Dota's 23 July 2026 patch; not checked since.",
  },
  russian: {
    ru: "Русский",
    en: "Russian",
    native: "Русский",
    verified: true,
    noteRu: "Проверен на Windows: сборка из четырёх модов работала в игре.",
    noteEn: "Checked on Windows: a four-mod build worked in game.",
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
    noteRu: "Ещё не проверен в игре на Windows. По данным Steam, у Dota полная корейская озвучка.",
    noteEn: "Not checked in game on Windows yet. Steam lists full Korean voice-over for Dota.",
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
      "Ещё не проверен в игре на Windows. По данным Steam, у Dota полная озвучка на упрощённом китайском.",
    noteEn:
      "Not checked in game on Windows yet. Steam lists full Simplified Chinese voice-over for Dota.",
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
