import { getStorageItem, setStorageItem } from "./storage";

export type PresetSource = "local" | "betterfy";

export type BetterFyPreset = {
  schemaVersion: 1;
  id: string;
  name: string;
  description: string;
  author: string;
  version: string;
  modIds: string[];
  wardrobeIds: string[];
  source: PresetSource;
  readOnly: boolean;
  createdAt: string;
  updatedAt: string;
};

export type SavePresetRequest = {
  id?: string;
  name: string;
  description: string;
  modIds: string[];
  wardrobeIds?: string[];
};

const storageKey = "betterfy:config-presets:v1";
const maxItems = 256;

const uniqueIds = (ids: string[]) =>
  [...new Set(ids.map((id) => id.trim()).filter(Boolean))].slice(0, maxItems);

const now = () => new Date().toISOString();

export const workshopPresets: BetterFyPreset[] = [
  {
    schemaVersion: 1,
    id: "betterfy.focus-performance",
    name: "Focus / Performance",
    description: "Убирает тяжёлые и отвлекающие слои мира, сохраняя читаемую основу игры.",
    author: "BetterFy",
    version: "1.0.0",
    modIds: [
      "minify-misc-optimization",
      "minify-remove-foilage",
      "minify-remove-river",
      "minify-remove-weather-effects",
      "minify-remove-hero-renders",
    ],
    wardrobeIds: [],
    source: "betterfy",
    readOnly: true,
    createdAt: "2026-08-02T00:00:00.000Z",
    updatedAt: "2026-08-02T00:00:00.000Z",
  },
  {
    schemaVersion: 1,
    id: "betterfy.clean-interface",
    name: "Clean Interface",
    description: "Спокойный интерфейс: меньше фонового шума, прозрачный HUD и компактная сетка героев.",
    author: "BetterFy",
    version: "1.0.0",
    modIds: [
      "minify-remove-main-menu-background",
      "minify-remove-showcases",
      "minify-transparent-hud",
      "minify-revamp-hero-grid-layout",
      "minify-show-networth",
    ],
    wardrobeIds: [],
    source: "betterfy",
    readOnly: true,
    createdAt: "2026-08-02T00:00:00.000Z",
    updatedAt: "2026-08-02T00:00:00.000Z",
  },
  {
    schemaVersion: 1,
    id: "betterfy.quiet-match",
    name: "Quiet Match",
    description: "Снижает аудионагрузку: окружение, насмешки и голосовые реплики отключаются отдельно.",
    author: "BetterFy",
    version: "1.0.0",
    modIds: [
      "minify-mute-ambient-sounds",
      "minify-mute-taunt-sounds",
      "minify-mute-voice-line-sounds",
    ],
    wardrobeIds: [],
    source: "betterfy",
    readOnly: true,
    createdAt: "2026-08-02T00:00:00.000Z",
    updatedAt: "2026-08-02T00:00:00.000Z",
  },
];

const isTauriRuntime = () =>
  typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;

const readBrowserPresets = (): BetterFyPreset[] => {
  try {
    const parsed = JSON.parse(getStorageItem(storageKey) ?? "[]");
    return Array.isArray(parsed)
      ? parsed.filter((item): item is BetterFyPreset =>
          item?.schemaVersion === 1 && item?.source === "local" && typeof item?.id === "string")
      : [];
  } catch {
    return [];
  }
};

const writeBrowserPresets = (presets: BetterFyPreset[]) => {
  if (!setStorageItem(storageKey, JSON.stringify(presets))) throw new Error("preset_store_unavailable");
};

const normalizeRequest = (request: SavePresetRequest): SavePresetRequest => ({
  ...request,
  name: request.name.trim().slice(0, 64),
  description: request.description.trim().slice(0, 280),
  modIds: uniqueIds(request.modIds),
  wardrobeIds: uniqueIds(request.wardrobeIds ?? []),
});

export const presetBridge = {
  async list(): Promise<BetterFyPreset[]> {
    if (!isTauriRuntime()) return [...readBrowserPresets(), ...workshopPresets];
    const { invoke } = await import("@tauri-apps/api/core");
    return invoke<BetterFyPreset[]>("list_presets");
  },

  async save(input: SavePresetRequest): Promise<BetterFyPreset> {
    const request = normalizeRequest(input);
    if (!request.name || request.modIds.length + (request.wardrobeIds?.length ?? 0) === 0) {
      throw new Error("preset_invalid");
    }
    if (isTauriRuntime()) {
      const { invoke } = await import("@tauri-apps/api/core");
      return invoke<BetterFyPreset>("save_preset", { request });
    }
    const presets = readBrowserPresets();
    const previous = request.id ? presets.find((preset) => preset.id === request.id) : undefined;
    const timestamp = now();
    const preset: BetterFyPreset = {
      schemaVersion: 1,
      id: previous?.id ?? `local.${Date.now().toString(36)}`,
      name: request.name,
      description: request.description,
      author: "Local profile",
      version: previous?.version ?? "1.0.0",
      modIds: request.modIds,
      wardrobeIds: request.wardrobeIds ?? [],
      source: "local",
      readOnly: false,
      createdAt: previous?.createdAt ?? timestamp,
      updatedAt: timestamp,
    };
    writeBrowserPresets([...presets.filter((item) => item.id !== preset.id), preset]);
    return preset;
  },

  async remove(id: string): Promise<void> {
    if (isTauriRuntime()) {
      const { invoke } = await import("@tauri-apps/api/core");
      await invoke("delete_preset", { id });
      return;
    }
    writeBrowserPresets(readBrowserPresets().filter((preset) => preset.id !== id));
  },

  async export(preset: BetterFyPreset): Promise<string> {
    if (isTauriRuntime()) {
      const { invoke } = await import("@tauri-apps/api/core");
      return invoke<string>("export_preset", { id: preset.id });
    }
    return JSON.stringify(preset, null, 2);
  },

  async import(serialized: string): Promise<BetterFyPreset> {
    if (isTauriRuntime()) {
      const { invoke } = await import("@tauri-apps/api/core");
      return invoke<BetterFyPreset>("import_preset", { serialized });
    }
    const parsed = JSON.parse(serialized) as Partial<BetterFyPreset>;
    if (parsed.schemaVersion !== 1 || typeof parsed.name !== "string") throw new Error("preset_invalid");
    return this.save({
      name: parsed.name,
      description: typeof parsed.description === "string" ? parsed.description : "",
      modIds: Array.isArray(parsed.modIds) ? parsed.modIds.filter((id): id is string => typeof id === "string") : [],
      wardrobeIds: Array.isArray(parsed.wardrobeIds)
        ? parsed.wardrobeIds.filter((id): id is string => typeof id === "string")
        : [],
    });
  },
};
