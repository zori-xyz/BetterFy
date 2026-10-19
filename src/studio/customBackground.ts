import { useSyncExternalStore } from "react";
import { getStorageItem, setStorageItem } from "../storage";

/**
 * The background picked for Minify's Custom Backgrounds: one of the game's
 * own loading screens or the player's video. The engine reads the choice
 * from the package ID (src-tauri/src/custom_backgrounds.rs).
 */
export const CUSTOM_BACKGROUNDS_ID = "minify.custom-backgrounds";
export const CUSTOM_BACKGROUNDS_CATALOG_ID = "minify-custom-backgrounds";

/** The engine's list, in its order, with a readable name. */
export const loadingScreens: Array<{ id: string; label: string }> = [
  ...[1, 2, 3, 4, 5].map((n) => ({ id: `international_2025_ls_${n}`, label: `The International 2025 · ${n}` })),
  ...[1, 2, 3, 4, 5].map((n) => ({ id: `international_2024_ls_${n}`, label: `The International 2024 · ${n}` })),
  ...[1, 2, 3, 4, 5, 6].map((n) => ({ id: `international_2023_ls_${n}`, label: `The International 2023 · ${n}` })),
  ...[1, 2, 3, 4, 5].map((n) => ({ id: `compendium2021_${n}_ls`, label: `Compendium 2021 · ${n}` })),
  { id: "aghs_2021_roshan_loadingscreen", label: "Aghanim's Labyrinth 2021 · Roshan" },
  { id: "diretide_2022_candy_loading_screen", label: "Diretide 2022 · Candy" },
  { id: "diretide_2022_greevil_loading_screen", label: "Diretide 2022 · Greevil" },
  { id: "frostivus_2018_loadingscreen", label: "Frostivus 2018" },
  ...[1, 2, 3].map((n) => ({ id: `fall_major_2016_loadscreen_${n}`, label: `Fall Major 2016 · ${n}` })),
];

export type BackgroundChoice =
  | { kind: "screen"; id: string }
  | { kind: "video"; packageId: string; bytes: number };

const STORAGE_KEY = "betterfy:custom-background";
const listeners = new Set<() => void>();
const VIDEO_ID = /^minify\.custom-backgrounds\.video-[0-9a-f]{64}-[1-9][0-9]*$/;

function read(): BackgroundChoice {
  try {
    const value = JSON.parse(getStorageItem(STORAGE_KEY) ?? "null") as BackgroundChoice | null;
    if (value?.kind === "screen" && loadingScreens.some((screen) => screen.id === value.id)) return value;
    if (value?.kind === "video" && VIDEO_ID.test(value.packageId)) return value;
  } catch {
    // fall through to the default
  }
  // Minify's own default: the third TI 2025 loading screen.
  return { kind: "screen", id: "international_2025_ls_3" };
}

let current = read();

export function setBackgroundChoice(next: BackgroundChoice) {
  current = next;
  setStorageItem(STORAGE_KEY, JSON.stringify(next));
  listeners.forEach((listener) => listener());
}

export function useBackgroundChoice() {
  return useSyncExternalStore(
    (listener) => {
      listeners.add(listener);
      return () => listeners.delete(listener);
    },
    () => current,
  );
}

export function backgroundRequestId(choice = current) {
  return choice.kind === "screen"
    ? `${CUSTOM_BACKGROUNDS_ID}.ls-${choice.id}`
    : choice.packageId;
}

/** A background variant ID names the same package as the bare ID. */
export function backgroundBase(id: string) {
  return id.startsWith(`${CUSTOM_BACKGROUNDS_ID}.ls-`) || VIDEO_ID.test(id) ? CUSTOM_BACKGROUNDS_ID : id;
}
