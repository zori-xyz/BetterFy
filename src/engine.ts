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
  operations: Array<{
    ownerId: string;
    source: string;
    destination: string;
    size: number;
    expectedSha256: string;
  }>;
  conflicts: Array<{ destination: string; contenders: string[] }>;
  spaceEstimate: number;
  executable: boolean;
};

export type BuildReceipt = {
  operationId: string;
  planId: string;
  phase: "ready";
  stagedRoot: string;
  stagedFiles: number;
  stagedBytes: number;
  checksumsVerified: boolean;
};

export type RollbackReceipt = {
  operationId: string;
  phase: "rolled_back";
  removedStaging: boolean;
};

export type EngineOperationSummary = {
  operationId: string;
  planId: string;
  phase: "staging" | "verifying" | "ready" | "failed" | "rolled_back";
  createdAtMs: number;
  stagedFiles: number;
};

export interface EngineBridge {
  discoverGame(): Promise<GameInstallation[]>;
  validateGamePath(path: string): Promise<GameInstallation>;
  planBuild(modIds: string[]): Promise<BuildPlan>;
  buildAndLaunch(
    modIds: string[],
    language: "ru" | "en",
    onProgress: (snapshot: EngineSnapshot) => void,
  ): Promise<BuildReceipt>;
  rollbackOperation(operationId: string): Promise<RollbackReceipt>;
  listOperations(): Promise<EngineOperationSummary[]>;
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
      expectedSha256: "preview-only",
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
    return {
      operationId: `preview-${Date.now()}`,
      planId: "fixture-plan-v1",
      phase: "ready",
      stagedRoot: "BetterFy / preview staging",
      stagedFiles: Math.max(1, modIds.length),
      stagedBytes: 0,
      checksumsVerified: false,
    };
  },
  async rollbackOperation(operationId) {
    await wait(780);
    return { operationId, phase: "rolled_back", removedStaging: true };
  },
  async listOperations() {
    return [];
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
  async buildAndLaunch(modIds, language, onProgress) {
    if (!isTauriRuntime()) return mockEngine.buildAndLaunch(modIds, language, onProgress);
    const { invoke } = await import("@tauri-apps/api/core");
    const normalized = uniqueModIds(modIds);
    onProgress({
      phase: "checking",
      progress: 12,
      message: language === "ru" ? "Проверяем состав" : "Checking build contents",
    });
    const plan = await guardedEngineCall(
      "plan_build",
      invoke<BuildPlan>("plan_build", { request: { modIds: normalized } }),
    );
    if (!plan.executable) {
      throw new EngineFault("invalid_response", "plan_build");
    }
    onProgress({
      phase: "resolving",
      progress: 36,
      message: language === "ru" ? "План зафиксирован" : "Build plan locked",
    });
    onProgress({
      phase: "building",
      progress: 64,
      message: language === "ru" ? "Собираем в staging BetterFy" : "Building in BetterFy staging",
    });
    const receipt = await guardedEngineCall(
      "execute_build",
      invoke<BuildReceipt>("execute_build", { request: { modIds: normalized } }),
    );
    if (!receipt.checksumsVerified || receipt.phase !== "ready") {
      throw new EngineFault("invalid_response", "execute_build");
    }
    onProgress({
      phase: "verifying",
      progress: 90,
      message: language === "ru" ? "Контрольные суммы совпали" : "Checksums verified",
    });
    await wait(420);
    onProgress({
      phase: "ready",
      progress: 100,
      message: language === "ru" ? "Безопасная сборка готова" : "Safe staging build ready",
    });
    return receipt;
  },
  async rollbackOperation(operationId) {
    if (!isTauriRuntime()) return mockEngine.rollbackOperation(operationId);
    const { invoke } = await import("@tauri-apps/api/core");
    return guardedEngineCall(
      "rollback_engine_operation",
      invoke<RollbackReceipt>("rollback_engine_operation", { operationId }),
    );
  },
  async listOperations() {
    if (!isTauriRuntime()) return mockEngine.listOperations();
    const { invoke } = await import("@tauri-apps/api/core");
    const result = await guardedEngineCall(
      "list_engine_operations",
      invoke<EngineOperationSummary[]>("list_engine_operations"),
    );
    if (!Array.isArray(result)) {
      throw new EngineFault("invalid_response", "list_engine_operations");
    }
    return result;
  },
};
