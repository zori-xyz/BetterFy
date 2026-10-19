import { useSyncExternalStore } from "react";
import type { CustomFontInfo } from "../engine";
import { getStorageItem, setStorageItem } from "../storage";

/**
 * The font the player picked for Minify's Custom Fonts. The engine keeps the
 * file and names it in the package ID (`minify.custom-fonts.font-<sha>-<size>`,
 * src-tauri/src/custom_fonts.rs); the interface remembers that ID.
 */
export const CUSTOM_FONTS_ID = "minify.custom-fonts";
export const CUSTOM_FONTS_CATALOG_ID = "minify-custom-fonts";

const STORAGE_KEY = "betterfy:custom-font";
const listeners = new Set<() => void>();

function read(): CustomFontInfo | null {
  try {
    const value = JSON.parse(getStorageItem(STORAGE_KEY) ?? "null") as CustomFontInfo | null;
    return value && /^minify\.custom-fonts\.font-[0-9a-f]{64}-[1-9][0-9]*$/.test(value.packageId)
      ? value
      : null;
  } catch {
    return null;
  }
}

let current = read();

export function setCustomFont(next: CustomFontInfo | null) {
  current = next;
  setStorageItem(STORAGE_KEY, JSON.stringify(next));
  listeners.forEach((listener) => listener());
}

export function useCustomFont() {
  return useSyncExternalStore(
    (listener) => {
      listeners.add(listener);
      return () => listeners.delete(listener);
    },
    () => current,
  );
}

/** The engine ID for the picked font; the bare ID when none is picked. */
export function customFontRequestId(font = current) {
  return font?.packageId ?? CUSTOM_FONTS_ID;
}

/** `minify.custom-fonts.font-<sha>-<size>` names the same package as the bare ID. */
export function customFontBase(id: string) {
  return /^minify\.custom-fonts\.font-[0-9a-f]{64}-[1-9][0-9]*$/.test(id) ? CUSTOM_FONTS_ID : id;
}
