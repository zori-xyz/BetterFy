export type EnginePhase =
  | "idle"
  | "checking"
  | "resolving"
  | "building"
  | "verifying"
  | "ready";

export type EngineSnapshot = {
  phase: EnginePhase;
  progress: number;
  message: string;
};

export type GameInstallation = {
  path: string;
  executablePath: string;
  steamLibrary: string;
  client: string;
  source: "auto" | "manual" | "demo";
  verified: boolean;
};

export type BuildPlan = {
  planId: string;
  dryRun: boolean;
  inputs: Array<{ id: string; name: string; version: string }>;
  operations: Array<{ ownerId: string; source: string; destination: string; size: number }>;
  conflicts: Array<{ destination: string; contenders: string[] }>;
  spaceEstimate: number;
  executable: boolean;
};

export interface EngineBridge {
  discoverGame(): Promise<GameInstallation[]>;
  validateGamePath(path: string): Promise<GameInstallation>;
  planBuild(modIds: string[]): Promise<BuildPlan>;
  buildAndLaunch(
    modIds: string[],
    language: "ru" | "en",
    onProgress: (snapshot: EngineSnapshot) => void,
  ): Promise<void>;
}

const wait = (duration: number) => new Promise((resolve) => window.setTimeout(resolve, duration));
const engineTimeoutMs = 15_000;

export class EngineFault extends Error {
  constructor(
    public readonly code: "timeout" | "bridge_error" | "invalid_response",
    public readonly operation: string,
    options?: { cause?: unknown },
  ) {
    super(`${operation}:${code}`, options);
    this.name = "EngineFault";
  }
}

async function guardedEngineCall<T>(operation: string, task: Promise<T>) {
  let timer = 0;
  const timeout = new Promise<never>((_, reject) => {
    timer = window.setTimeout(
      () => reject(new EngineFault("timeout", operation)),
      engineTimeoutMs,
    );
  });
  try {
    return await Promise.race([task, timeout]);
  } catch (error) {
    if (error instanceof EngineFault) throw error;
    throw new EngineFault("bridge_error", operation, { cause: error });
  } finally {
    window.clearTimeout(timer);
  }
}

const uniqueModIds = (modIds: string[]) =>
  [...new Set(modIds.map((id) => id.trim()).filter(Boolean))];

const demoInstallation: GameInstallation = {
  path: "C:\\Program Files (x86)\\Steam\\steamapps\\common\\dota 2 beta",
  executablePath: "…\\game\\bin\\win64\\dota2.exe",
  steamLibrary: "Steam / default",
  client: "Prototype preview",
  source: "demo",
  verified: false,
};

export const mockEngine: EngineBridge = {
  async discoverGame() {
    await wait(1450);
    if (new URLSearchParams(window.location.search).has("game-missing")) return [];
    return [demoInstallation];
  },
  async validateGamePath(path) {
    await wait(780);
    const normalized = path.trim();
    if (!normalized) throw new Error("invalid_game_path");
    return { ...demoInstallation, path: normalized, source: "demo" };
  },
  async planBuild(modIds) {
    await wait(780);
    modIds = uniqueModIds(modIds);
    const inputs = [
      { id: "fixture.ambient-violet", name: "Ambient Violet", version: "1.0.0" },
      { id: "fixture.ambient-clean", name: "Ambient Clean", version: "1.0.0" },
    ].filter((input) => modIds.includes(input.id));
    const operations = inputs.map((input, index) => ({
      ownerId: input.id,
      source: index ? "panorama/styles/ambient-clean.css" : "panorama/styles/ambient-violet.css",
      destination: "game/dota/pak01_dir/panorama/styles/betterfy-theme.css",
      size: index ? 1536 : 2048,
    }));
    const contenders = operations.map((operation) => operation.ownerId);
    return {
      planId: "fixture-plan-v1",
      dryRun: true,
      inputs,
      operations,
      conflicts: contenders.length > 1
        ? [{ destination: operations[0].destination, contenders }]
        : [],
      spaceEstimate: operations.reduce((total, operation) => total + operation.size, 0),
      executable: operations.length > 0 && contenders.length < 2,
    };
  },
  async buildAndLaunch(modIds, language, onProgress) {
    modIds = uniqueModIds(modIds);
    const stages: EngineSnapshot[] =
      language === "ru"
        ? [
            { phase: "checking", progress: 12, message: "Проверяем игровой профиль" },
            { phase: "resolving", progress: 34, message: `Готовим моды: ${modIds.length}` },
            { phase: "building", progress: 68, message: "Собираем VPK-пакет" },
            { phase: "verifying", progress: 88, message: "Проверяем результат" },
            { phase: "ready", progress: 100, message: "Сборка готова" },
          ]
        : [
            { phase: "checking", progress: 12, message: "Checking game profile" },
            { phase: "resolving", progress: 34, message: `Preparing mods: ${modIds.length}` },
            { phase: "building", progress: 68, message: "Building VPK package" },
            { phase: "verifying", progress: 88, message: "Verifying the result" },
            { phase: "ready", progress: 100, message: "Build is ready" },
          ];

    for (const stage of stages) {
      onProgress(stage);
      await wait(stage.phase === "building" ? 900 : 560);
    }
  },
};

const isTauriRuntime = () =>
  typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;

export const engineBridge: EngineBridge = {
  async discoverGame() {
    if (!isTauriRuntime()) return mockEngine.discoverGame();
    const { invoke } = await import("@tauri-apps/api/core");
    const result = await guardedEngineCall(
      "discover_game",
      invoke<GameInstallation[]>("discover_game"),
    );
    if (!Array.isArray(result)) throw new EngineFault("invalid_response", "discover_game");
    return result;
  },
  async validateGamePath(path) {
    if (!isTauriRuntime()) return mockEngine.validateGamePath(path);
    const { invoke } = await import("@tauri-apps/api/core");
    const normalizedPath = path.trim();
    if (!normalizedPath) throw new EngineFault("invalid_response", "validate_game_path");
    return guardedEngineCall(
      "validate_game_path",
      invoke<GameInstallation>("validate_game_path", { path: normalizedPath }),
    );
  },
  async planBuild(modIds) {
    if (!isTauriRuntime()) return mockEngine.planBuild(modIds);
    const { invoke } = await import("@tauri-apps/api/core");
    return guardedEngineCall(
      "plan_build",
      invoke<BuildPlan>("plan_build", { request: { modIds: uniqueModIds(modIds) } }),
    );
  },
  buildAndLaunch: mockEngine.buildAndLaunch,
};
