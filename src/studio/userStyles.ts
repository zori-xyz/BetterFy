import { useSyncExternalStore } from "react";
import type { Language } from "../i18n";
import { getStorageItem, setStorageItem } from "../storage";

/**
 * Minify's User Styles: optional snippets of one pinned file, in file order
 * (src-tauri/src/user_styles.rs splits the same file the same way). The
 * engine installs the snippets named by the package ID
 * `minify.user-styles.pick-<hex>`, one bit per snippet.
 */
export const USER_STYLES_ID = "minify.user-styles";
export const USER_STYLES_CATALOG_ID = "minify-user-styles";

export const userStyleSnippets: Array<Record<Language, string>> = [
  { ru: "Убрать портрет героя в HUD", en: "Remove the hero portrait from the HUD" },
  { ru: "Убрать фильтры характеристик в поиске магазина", en: "Remove stat filters from shop search" },
  { ru: "Убрать баннер Dota Plus в последних матчах", en: "Remove the Dota Plus banner from recent matches" },
  { ru: "Убрать фон у кнопки «Играть»", en: "Remove the Play button backgrounds" },
  { ru: "Убрать лишнее в описании предметов", en: "Remove clutter from item details" },
  { ru: "Убрать анимации камня безумия", en: "Remove madstone animations" },
  { ru: "Подсказки без анимации появления", en: "Tooltips without the fade-in" },
  { ru: "Убрать затемнение и размытие за окнами", en: "Remove dims and blurs behind popups" },
  { ru: "Шире панель стадии подготовки", en: "Wider pregame panel after picking" },
  { ru: "Свернуть лишние разделы в списке друзей", en: "Collapse some friends list sections" },
  { ru: "Цветные кнопки в меню предмета", en: "Colored item action menu buttons" },
  { ru: "Магазин меньше и без анимации", en: "Smaller shop without its animation" },
  { ru: "Убрать серый слой Monster Hunter в сетке героев", en: "Remove the gray Monster Hunter overlay in the hero grid" },
  { ru: "Убрать 3D-модель в значке героя", en: "Remove the 3D model from hero badges" },
  { ru: "Убрать сезонную панель способностей", en: "Remove the seasonal ability bar" },
  { ru: "Убрать детали последнего матча на главной", en: "Remove last match details from the home page" },
  { ru: "Убрать блоки событий на главной", en: "Remove event containers from the home page" },
];

const STORAGE_KEY = "betterfy:user-styles";
const listeners = new Set<() => void>();

function readMask() {
  const value = Number.parseInt(getStorageItem(STORAGE_KEY) ?? "0", 10);
  const all = (1 << userStyleSnippets.length) - 1;
  return Number.isFinite(value) && value > 0 ? value & all : 0;
}

let mask = readMask();

export function setUserStylesMask(next: number) {
  mask = next & ((1 << userStyleSnippets.length) - 1);
  setStorageItem(STORAGE_KEY, String(mask));
  listeners.forEach((listener) => listener());
}

export function useUserStylesMask() {
  return useSyncExternalStore(
    (listener) => {
      listeners.add(listener);
      return () => listeners.delete(listener);
    },
    () => mask,
  );
}

/** The engine ID for the current pick; the bare ID when nothing is picked. */
export function userStylesRequestId(current = mask) {
  return current > 0 ? `${USER_STYLES_ID}.pick-${current.toString(16)}` : USER_STYLES_ID;
}

/** `minify.user-styles.pick-<hex>` names the same package as the bare ID. */
export function userStylesBase(id: string) {
  return /^minify\.user-styles\.pick-[0-9a-f]+$/.test(id) ? USER_STYLES_ID : id;
}
