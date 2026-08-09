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
  inputs: Array<{
    id: string;
    name: string;
    version: string;
    contentIdentity: string;
  }>;
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

export type RuntimeState = {
  platformSupported: boolean;
  steamRunning: boolean;
  dotaRunning: boolean;
  patchReady: boolean;
};

export type DiagnosticState = "ready" | "attention" | "blocked" | "unsupported";

export type DiagnosticDetail =
  | "windows_supported"
  | "development_only"
  | "game_verified"
  | "game_missing"
  | "demo_game"
  | "runtime_ready"
  | "dota_running"
  | "steam_running"
  | "runtime_unavailable"
  | "profiles_missing"
  | "profiles_need_attention"
  | "profiles_managed"
  | "profiles_ready"
  | "profiles_unavailable"
  | "staging_recovery_available"
  | "staging_history_ready"
  | "staging_clean"
  | "staging_unavailable"
  | "content_store_ready"
  | "content_store_empty"
  | "content_store_invalid";

export type SystemDiagnosticReport = {
  schemaVersion: 1;
  appVersion: string;
  platform: "windows" | "macos" | "other";
  generatedAtMs: number;
  overall: DiagnosticState;
  checks: Array<{
    code: "platform" | "game" | "runtime" | "steam_profiles" | "staging" | "content";
    state: DiagnosticState;
    detail: DiagnosticDetail;
    value: number;
  }>;
};

export type ContentReceipt = {
  packageId: string;
  version: string;
  contentIdentity: string;
  size: number;
  alreadyPresent: boolean;
  signatureStatus: "not_provided" | "unverified" | "verified";
  compatibility: "unknown" | "verified" | "unsupported";
};

export type ContentDownloadStatus = {
  operationId: string;
  packageId: string;
  phase: "queued" | "downloading" | "verifying" | "ready" | "failed" | "cancelled";
  receivedBytes: number;
  expectedBytes: number;
  contentIdentity: string | null;
  errorCode: string | null;
  archiveReport: {
    entries: number;
    files: number;
    expandedBytes: number;
    compressedBytes: number;
  } | null;
};

export type SteamProfileSummary = {
  profileToken: string;
  profileIndex: number;
  status: "ready" | "already_managed" | "launch_option_conflict" | "invalid_config";
};

export type SteamLaunchOptionPreview = {
  profileToken: string;
  changed: boolean;
  beforeSha256: string;
  afterSha256: string;
  confirmationToken: string;
};

export type SteamConfigReceipt = {
  operationId: string | null;
  profileToken: string;
  changed: boolean;
  beforeSha256: string;
  afterSha256: string;
  backupVerified: boolean;
  committed: boolean;
  rolledBack: boolean;
};

export type GameDeploymentReceipt = {
  operationId: string;
  beforeSha256: string | null;
  installedSha256: string;
  backupVerified: boolean;
  committed: boolean;
  rolledBack: boolean;
};

export type GameDeploymentRecovery = {
  inspected: number;
  rolledBack: number;
  markedFailed: number;
};

export interface EngineBridge {
  intakeFixtureContent(packageIds: string[]): Promise<ContentReceipt[]>;
  beginContentDownload(packageId: string): Promise<ContentDownloadStatus>;
  contentDownloadStatus(operationId: string): Promise<ContentDownloadStatus>;
  cancelContentDownload(operationId: string): Promise<ContentDownloadStatus>;
  collectSystemDiagnostics(gamePath: string | null): Promise<SystemDiagnosticReport>;
  discoverGame(): Promise<GameInstallation[]>;
  inspectRuntime(): Promise<RuntimeState>;
  prepareRuntimeForPatch(): Promise<RuntimeState>;
  validateGamePath(path: string): Promise<GameInstallation>;
  planBuild(modIds: string[]): Promise<BuildPlan>;
  buildProfile(
    modIds: string[],
    language: "ru" | "en",
    onProgress: (snapshot: EngineSnapshot) => void,
  ): Promise<BuildReceipt>;
  rollbackOperation(operationId: string): Promise<RollbackReceipt>;
  listOperations(): Promise<EngineOperationSummary[]>;
  listSteamProfiles(): Promise<SteamProfileSummary[]>;
  previewSteamLaunchOptions(profileToken: string): Promise<SteamLaunchOptionPreview>;
  applySteamLaunchOptions(preview: SteamLaunchOptionPreview): Promise<SteamConfigReceipt>;
  rollbackSteamLaunchOptions(operationId: string): Promise<SteamConfigReceipt>;
  recoverSteamLaunchOptions(): Promise<SteamConfigReceipt[]>;
  deployStagedVpk(gamePath: string, receipt: BuildReceipt): Promise<GameDeploymentReceipt>;
  rollbackGameDeployment(gamePath: string, operationId: string): Promise<GameDeploymentReceipt>;
  recoverGameDeployments(gamePath: string): Promise<GameDeploymentRecovery>;
  startSteamAfterProfile(
    profileToken: string,
    operationId: string | null,
  ): Promise<RuntimeState>;
}

const wait = (duration: number) => new Promise((resolve) => window.setTimeout(resolve, duration));
const engineTimeoutMs = 15_000;

export class EngineFault extends Error {
  constructor(
    public readonly code: string,
    public readonly operation: string,
    options?: { cause?: unknown },
  ) {
    super(`${operation}:${code}`, options);
    this.name = "EngineFault";
  }
}

function backendErrorCode(error: unknown) {
  const candidate = typeof error === "string"
    ? error
    : error instanceof Error
      ? error.message
      : "";
  return /^[a-z][a-z0-9_]{2,63}$/.test(candidate) ? candidate : "bridge_error";
}

async function guardedEngineCall<T>(operation: string, task: Promise<T>, timeoutMs = engineTimeoutMs) {
  let timer = 0;
  const timeout = new Promise<never>((_, reject) => {
    timer = window.setTimeout(
      () => reject(new EngineFault("timeout", operation)),
      timeoutMs,
    );
  });
  try {
    return await Promise.race([task, timeout]);
  } catch (error) {
    if (error instanceof EngineFault) throw error;
    throw new EngineFault(backendErrorCode(error), operation, { cause: error });
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
  async intakeFixtureContent(packageIds) {
    return [...new Set(packageIds)].sort().map((packageId) => ({
      packageId,
      version: "1.0.0",
      contentIdentity: `preview:${packageId}`,
      size: 0,
      alreadyPresent: false,
      signatureStatus: "not_provided" as const,
      compatibility: "unknown" as const,
    }));
  },
  async beginContentDownload(packageId) {
    return {
      operationId: `preview-download-${Date.now()}`,
      packageId,
      phase: "failed",
      receivedBytes: 0,
      expectedBytes: 0,
      contentIdentity: null,
      errorCode: "desktop_runtime_required",
      archiveReport: null,
    };
  },
  async contentDownloadStatus() {
    throw new EngineFault("desktop_runtime_required", "content_download_status");
  },
  async cancelContentDownload() {
    throw new EngineFault("desktop_runtime_required", "cancel_content_download");
  },
  async collectSystemDiagnostics() {
    return {
      schemaVersion: 1,
      appVersion: "0.1.0-preview",
      platform: "other",
      generatedAtMs: Date.now(),
      overall: "unsupported",
      checks: [
        { code: "platform", state: "unsupported", detail: "development_only", value: 0 },
        { code: "game", state: "attention", detail: "demo_game", value: 0 },
        { code: "runtime", state: "unsupported", detail: "runtime_unavailable", value: 0 },
        { code: "steam_profiles", state: "unsupported", detail: "profiles_unavailable", value: 0 },
        { code: "staging", state: "ready", detail: "staging_clean", value: 0 },
        { code: "content", state: "ready", detail: "content_store_empty", value: 0 },
      ],
    } satisfies SystemDiagnosticReport;
  },
  async inspectRuntime() {
    return {
      platformSupported: false,
      steamRunning: false,
      dotaRunning: false,
      patchReady: true,
    };
  },
  async prepareRuntimeForPatch() {
    return this.inspectRuntime();
  },
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
      {
        id: "fixture.ambient-violet",
        name: "Ambient Violet",
        version: "1.0.0",
        contentIdentity: "sha256:e2c06353f3a5c99162512e40c6a2e318778d1c73bc0ba79126df5a4beff13c64",
      },
      {
        id: "fixture.ambient-clean",
        name: "Ambient Clean",
        version: "1.0.0",
        contentIdentity: "sha256:1146e6e8922159c697679d77ab6a9b3972409df6552f5929ad93720e3e4e09f0",
      },
    ].filter((input) => modIds.includes(input.id));
    const operations = inputs.map((input, index) => ({
      ownerId: input.id,
      source: index ? "panorama/styles/ambient-clean.css" : "panorama/styles/ambient-violet.css",
      destination: "game/dota/pak01_dir/panorama/styles/betterfy-theme.css",
      size: 61,
      expectedSha256: input.contentIdentity.slice("sha256:".length),
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
  async buildProfile(modIds, language, onProgress) {
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
  async listSteamProfiles() {
    return [];
  },
  async previewSteamLaunchOptions() {
    throw new EngineFault("bridge_error", "preview_steam_launch_options");
  },
  async applySteamLaunchOptions() {
    throw new EngineFault("bridge_error", "apply_steam_launch_options");
  },
  async rollbackSteamLaunchOptions() {
    throw new EngineFault("bridge_error", "rollback_steam_launch_options");
  },
  async recoverSteamLaunchOptions() {
    return [];
  },
  async deployStagedVpk() {
    throw new EngineFault("desktop_runtime_required", "deploy_staged_vpk");
  },
  async rollbackGameDeployment() {
    throw new EngineFault("desktop_runtime_required", "rollback_game_deployment");
  },
  async recoverGameDeployments() {
    throw new EngineFault("desktop_runtime_required", "recover_game_deployments");
  },
  async startSteamAfterProfile() {
    throw new EngineFault("platform_not_supported", "start_steam_after_profile");
  },
};

const isTauriRuntime = () =>
  typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;

export const engineBridge: EngineBridge = {
  async intakeFixtureContent(packageIds) {
    if (!isTauriRuntime()) return mockEngine.intakeFixtureContent(packageIds);
    const { invoke } = await import("@tauri-apps/api/core");
    return guardedEngineCall(
      "intake_fixture_content",
      invoke<ContentReceipt[]>("intake_fixture_content", { request: { packageIds } }),
      20_000,
    );
  },
  async beginContentDownload(packageId) {
    if (!isTauriRuntime()) return mockEngine.beginContentDownload(packageId);
    const { invoke } = await import("@tauri-apps/api/core");
    return guardedEngineCall(
      "begin_content_download",
      invoke<ContentDownloadStatus>("begin_content_download", { packageId: packageId.trim() }),
    );
  },
  async contentDownloadStatus(operationId) {
    if (!isTauriRuntime()) return mockEngine.contentDownloadStatus(operationId);
    const { invoke } = await import("@tauri-apps/api/core");
    return guardedEngineCall(
      "content_download_status",
      invoke<ContentDownloadStatus>("content_download_status", { operationId }),
    );
  },
  async cancelContentDownload(operationId) {
    if (!isTauriRuntime()) return mockEngine.cancelContentDownload(operationId);
    const { invoke } = await import("@tauri-apps/api/core");
    return guardedEngineCall(
      "cancel_content_download",
      invoke<ContentDownloadStatus>("cancel_content_download", { operationId }),
    );
  },
  async collectSystemDiagnostics(gamePath) {
    if (!isTauriRuntime()) return mockEngine.collectSystemDiagnostics(gamePath);
    const { invoke } = await import("@tauri-apps/api/core");
    const report = await guardedEngineCall(
      "collect_system_diagnostics",
      invoke<SystemDiagnosticReport>("collect_system_diagnostics", { gamePath }),
      20_000,
    );
    if (report.schemaVersion !== 1 || !Array.isArray(report.checks)) {
      throw new EngineFault("invalid_response", "collect_system_diagnostics");
    }
    return report;
  },
  async inspectRuntime() {
    if (!isTauriRuntime()) return mockEngine.inspectRuntime();
    const { invoke } = await import("@tauri-apps/api/core");
    return guardedEngineCall(
      "inspect_runtime",
      invoke<RuntimeState>("inspect_runtime"),
    );
  },
  async prepareRuntimeForPatch() {
    if (!isTauriRuntime()) return mockEngine.prepareRuntimeForPatch();
    const { invoke } = await import("@tauri-apps/api/core");
    return guardedEngineCall(
      "prepare_runtime_for_patch",
      invoke<RuntimeState>("prepare_runtime_for_patch", {
        request: { confirmed: true },
      }),
      70_000,
    );
  },
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
  async buildProfile(modIds, language, onProgress) {
    if (!isTauriRuntime()) return mockEngine.buildProfile(modIds, language, onProgress);
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
      invoke<BuildReceipt>("execute_build", {
        request: {
          modIds: normalized,
          expectedPlanId: plan.planId,
          confirmed: true,
        },
      }),
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
  async listSteamProfiles() {
    if (!isTauriRuntime()) return mockEngine.listSteamProfiles();
    const { invoke } = await import("@tauri-apps/api/core");
    const result = await guardedEngineCall(
      "list_steam_profiles",
      invoke<SteamProfileSummary[]>("list_steam_profiles"),
    );
    if (!Array.isArray(result)) {
      throw new EngineFault("invalid_response", "list_steam_profiles");
    }
    return result;
  },
  async previewSteamLaunchOptions(profileToken) {
    if (!isTauriRuntime()) return mockEngine.previewSteamLaunchOptions(profileToken);
    const { invoke } = await import("@tauri-apps/api/core");
    return guardedEngineCall(
      "preview_steam_launch_options",
      invoke<SteamLaunchOptionPreview>("preview_steam_launch_options", { profileToken }),
    );
  },
  async applySteamLaunchOptions(preview) {
    if (!isTauriRuntime()) return mockEngine.applySteamLaunchOptions(preview);
    const { invoke } = await import("@tauri-apps/api/core");
    return guardedEngineCall(
      "apply_steam_launch_options",
      invoke<SteamConfigReceipt>("apply_steam_launch_options", {
        request: {
          profileToken: preview.profileToken,
          confirmationToken: preview.confirmationToken,
          confirmed: true,
        },
      }),
      30_000,
    );
  },
  async rollbackSteamLaunchOptions(operationId) {
    if (!isTauriRuntime()) return mockEngine.rollbackSteamLaunchOptions(operationId);
    const { invoke } = await import("@tauri-apps/api/core");
    return guardedEngineCall(
      "rollback_steam_launch_options",
      invoke<SteamConfigReceipt>("rollback_steam_launch_options", {
        request: { operationId, confirmed: true },
      }),
      30_000,
    );
  },
  async recoverSteamLaunchOptions() {
    if (!isTauriRuntime()) return mockEngine.recoverSteamLaunchOptions();
    const { invoke } = await import("@tauri-apps/api/core");
    const result = await guardedEngineCall(
      "recover_steam_launch_options",
      invoke<SteamConfigReceipt[]>("recover_steam_launch_options", { confirmed: true }),
      30_000,
    );
    if (!Array.isArray(result)) {
      throw new EngineFault("invalid_response", "recover_steam_launch_options");
    }
    return result;
  },
  async deployStagedVpk(gamePath, receipt) {
    if (!isTauriRuntime()) return mockEngine.deployStagedVpk(gamePath, receipt);
    const { invoke } = await import("@tauri-apps/api/core");
    return guardedEngineCall(
      "deploy_staged_vpk",
      invoke<GameDeploymentReceipt>("deploy_staged_vpk", {
        request: {
          gamePath,
          stagedOperationId: receipt.operationId,
          expectedPlanId: receipt.planId,
          confirmed: true,
        },
      }),
      90_000,
    );
  },
  async rollbackGameDeployment(gamePath, operationId) {
    if (!isTauriRuntime()) return mockEngine.rollbackGameDeployment(gamePath, operationId);
    const { invoke } = await import("@tauri-apps/api/core");
    return guardedEngineCall(
      "rollback_game_deployment",
      invoke<GameDeploymentReceipt>("rollback_game_deployment", {
        request: { gamePath, operationId, confirmed: true },
      }),
      90_000,
    );
  },
  async recoverGameDeployments(gamePath) {
    if (!isTauriRuntime()) return mockEngine.recoverGameDeployments(gamePath);
    const { invoke } = await import("@tauri-apps/api/core");
    return guardedEngineCall(
      "recover_game_deployments",
      invoke<GameDeploymentRecovery>("recover_game_deployments", {
        request: { gamePath, confirmed: true },
      }),
      90_000,
    );
  },
  async startSteamAfterProfile(profileToken, operationId) {
    if (!isTauriRuntime()) {
      return mockEngine.startSteamAfterProfile(profileToken, operationId);
    }
    const { invoke } = await import("@tauri-apps/api/core");
    return guardedEngineCall(
      "start_steam_after_profile",
      invoke<RuntimeState>("start_steam_after_profile", {
        request: {
          profileToken,
          operationId,
          confirmed: true,
        },
      }),
      30_000,
    );
  },
};
