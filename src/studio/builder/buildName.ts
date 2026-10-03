import { useEffect, useState } from "react";
import type { Language } from "../../i18n";
import { getStorageItem, setStorageItem } from "../../storage";
import { modById, type StudioMod } from "../model";

// A build gets a name from what is in it: three ambient mutes read as
// "Quiet in the Tavern", a tree and river pass as "A Forest Without Trees".
// The name is stable for the same composition, and the dice button walks
// through the other fitting names.

type Name = [ru: string, en: string];

// Names that describe one specific mod. They win when that mod is in the build.
const byMod: Record<string, Name[]> = {
  "minify-mute-ambient-sounds": [["Тишина в таверне", "Quiet in the Tavern"]],
  "minify-mute-default-announcer": [["Диктор в отпуске", "Announcer on Vacation"]],
  "minify-mute-voice-line-sounds": [["Герои молчат", "Heroes Keep Quiet"]],
  "minify-mute-taunt-sounds": [["Без насмешек", "No Taunts Allowed"]],
  "minify-remove-pings": [["Без пингов, без тильта", "No Pings, No Tilt"]],
  "minify-revert-ping-sounds": [["Старые добрые пинги", "Good Old Pings"]],
  "minify-tree-mod": [["Лес без деревьев", "A Forest Without Trees"]],
  "minify-remove-river": [["Сухое русло", "Dry Riverbed"]],
  "minify-remove-weather-effects": [["Всегда ясно", "Always Clear Skies"]],
  "minify-dark-terrain": [["Ночная смена", "Night Shift"]],
  "minify-remove-foilage": [["Стриженый газон", "Trimmed Lawn"]],
  "minify-show-networth": [["Считаем золото", "Counting Gold"]],
  "minify-transparent-hud": [["HUD-невидимка", "Ghost HUD"]],
  "minify-revamp-hero-grid-layout": [["Новый драфт", "Fresh Draft"]],
  "minify-repopulate-unit-query-hud": [["Всё о цели", "Know Your Target"]],
  "minify-auto-accept-match": [["Ни одной пропущенной игры", "Never Miss a Match"]],
  "minify-stat-site-buttons": [["Аналитик", "The Analyst"]],
  "minify-remove-sprays": [["Без граффити", "No Graffiti"]],
  "minify-remove-main-menu-background": [["Пустое меню", "Bare Menu"]],
};

// Names for the build's dominant theme.
const byTheme: Record<string, Name[]> = {
  audio: [
    ["Тишина в таверне", "Quiet in the Tavern"],
    ["Режим библиотеки", "Library Mode"],
    ["Беззвучный курьер", "Silent Courier"],
  ],
  world: [
    ["Чистая карта", "Clean Map"],
    ["Ясная видимость", "Clear Sight"],
  ],
  interface: [
    ["Чистый HUD", "Clean HUD"],
    ["Всё на виду", "Everything in Sight"],
  ],
  core: [
    ["Лёгкие драки", "Light Fights"],
    ["Меньше шума на экране", "Less Screen Noise"],
  ],
  utility: [["Мелкие удобства", "Small Comforts"]],
  custom: [["Своя настройка", "Your Own Setup"]],
  looks: [
    ["Примерочная", "Fitting Room"],
    ["Новый гардероб", "New Wardrobe"],
  ],
  couriers: [["Курьер при параде", "Courier in Uniform"]],
  terrains: [["Своя карта", "Custom Map"]],
  mixed: [
    ["Своя Dota", "Your Own Dota"],
    ["Полный апгрейд", "Full Upgrade"],
    ["Сборка на все случаи", "Build for Everything"],
  ],
};

const themeLabels: Record<string, Name> = {
  audio: ["Звук", "Audio"],
  world: ["Карта", "Map"],
  interface: ["HUD", "HUD"],
  core: ["Оптимизация", "Optimization"],
  utility: ["Удобства", "Utilities"],
  custom: ["Настройки", "Settings"],
  looks: ["Облики", "Looks"],
  couriers: ["Курьеры", "Couriers"],
  terrains: ["Ландшафты", "Terrains"],
};

function themeOf(mod: StudioMod) {
  if (mod.domain === "game") return mod.category;
  if (mod.category === "couriers" || mod.category === "terrains") return mod.category;
  return "looks";
}

function hash(value: string) {
  let result = 2166136261;
  for (let index = 0; index < value.length; index += 1) {
    result ^= value.charCodeAt(index);
    result = Math.imul(result, 16777619);
  }
  return result >>> 0;
}

export type BuildIdentity = {
  /** Every fitting name, most specific first. */
  names: Name[];
  /** Themes present, most represented first. */
  themes: string[];
  signature: string;
};

export function buildIdentity(ids: string[]): BuildIdentity {
  const chosen = ids.flatMap((id) => (modById.has(id) ? [modById.get(id)!] : []));
  const counts = new Map<string, number>();
  chosen.forEach((mod) => counts.set(themeOf(mod), (counts.get(themeOf(mod)) ?? 0) + 1));
  const themes = [...counts.entries()].sort((a, b) => b[1] - a[1]).map(([theme]) => theme);
  const names: Name[] = [];
  const add = (list: Name[] | undefined) =>
    list?.forEach((name) => {
      if (!names.some((existing) => existing[0] === name[0])) names.push(name);
    });
  if (themes.length >= 3) add(byTheme.mixed);
  // A single mod, or a build dominated by one theme, is named by its mods.
  if (themes.length <= 2) chosen.forEach((mod) => add(byMod[mod.id]));
  themes.forEach((theme) => add(byTheme[theme]));
  if (!names.length) add(byTheme.mixed);
  return { names, themes, signature: [...ids].sort().join("|") };
}

export function themeSummary(themes: string[], language: Language) {
  return themes
    .slice(0, 3)
    .map((theme) => themeLabels[theme]?.[language === "ru" ? 0 : 1] ?? theme)
    .join(" · ");
}

const rollKey = "betterfy:build-name-roll";

function storedRolls(): Record<string, number> {
  try {
    const value = JSON.parse(getStorageItem(rollKey) ?? "{}");
    return value && typeof value === "object" ? value : {};
  } catch {
    return {};
  }
}

/** The build's current name and a way to roll the next one. */
export function useBuildName(ids: string[], language: Language) {
  const identity = buildIdentity(ids);
  const [rolls, setRolls] = useState(storedRolls);
  useEffect(() => {
    // Keep the stored map small: only the current composition matters.
    setStorageItem(
      rollKey,
      JSON.stringify(
        identity.signature in rolls ? { [identity.signature]: rolls[identity.signature] } : {},
      ),
    );
  }, [rolls, identity.signature]);
  const offset = rolls[identity.signature] ?? 0;
  const index = (hash(identity.signature) + offset) % Math.max(1, identity.names.length);
  const name = identity.names[index] ?? (["Моя сборка", "My build"] as Name);
  return {
    name: name[language === "ru" ? 0 : 1],
    summary: themeSummary(identity.themes, language),
    canRoll: identity.names.length > 1,
    roll: () =>
      setRolls((previous) => ({
        [identity.signature]: (previous[identity.signature] ?? 0) + 1,
      })),
  };
}

/** The current name without React, for the save dialog default. */
export function currentBuildName(ids: string[], language: Language) {
  const identity = buildIdentity(ids);
  const offset = storedRolls()[identity.signature] ?? 0;
  const index = (hash(identity.signature) + offset) % Math.max(1, identity.names.length);
  return identity.names[index]?.[language === "ru" ? 0 : 1] ?? null;
}
