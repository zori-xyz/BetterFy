import { useSyncExternalStore } from "react";
import {
  EngineFault,
  mockEngine,
  type EngineBridge,
  type GameLanguage,
  type TreePilotDownloadStatus,
  type TreePilotPlan,
} from "../../engine";
import { getStorageItem, setStorageItem } from "../../storage";
import { engineIdFor, findPackage } from "../packages";

// The browser preview has no Rust engine. To show the whole builder there,
// this bridge plays the Windows flow with timers: nothing is downloaded and
// no file is written. Every screen that uses it says "Demo".

export const isDesktopRuntime = () =>
  typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;
export const demoActive = () => !isDesktopRuntime();

export type DemoScenario = "success" | "dota_open" | "download_fails" | "steam_fails";

type DemoSettings = { scenario: DemoScenario; developer: boolean };
const key = "betterfy:builder-demo";
let settings: DemoSettings = (() => {
  try {
    const stored = JSON.parse(getStorageItem(key) ?? "{}");
    return {
      scenario: ["success", "dota_open", "download_fails", "steam_fails"].includes(stored.scenario)
        ? stored.scenario
        : "success",
      developer: stored.developer === true,
    };
  } catch {
    return { scenario: "success", developer: false };
  }
})();
const listeners = new Set<() => void>();

export function setDemoSettings(next: Partial<DemoSettings>) {
  settings = { ...settings, ...next };
  setStorageItem(key, JSON.stringify(settings));
  listeners.forEach((listener) => listener());
}

export function useDemoSettings() {
  return useSyncExternalStore(
    (listener) => {
      listeners.add(listener);
      return () => listeners.delete(listener);
    },
    () => settings,
  );
}

const wait = (ms: number) => new Promise((resolve) => window.setTimeout(resolve, ms));

function seeded(value: string, min: number, max: number) {
  let hash = 7;
  for (const char of value) hash = (hash * 31 + char.charCodeAt(0)) >>> 0;
  return min + (hash % (max - min + 1));
}

// Resource counts come from the real manifests where they list resources;
// blacklist packages resolve against game files at runtime, so the demo uses
// a stable stand-in.
function resourcesFor(packageId: string) {
  const manifest = findPackage(packageId) as { resources?: unknown[] } | undefined;
  return Array.isArray(manifest?.resources) && manifest.resources.length
    ? manifest.resources.length
    : seeded(packageId, 40, 420);
}

function demoPlan(ids: string[]): TreePilotPlan {
  const packageIds = ids.map(engineIdFor);
  const contributions = packageIds.map((packageId, index) => {
    const input = resourcesFor(packageId);
    const shadowed = index > 0 && packageIds.length > 2 ? Math.min(3, input - 1) : 0;
    return {
      packageId,
      inputResources: input,
      effectiveResources: input - shadowed,
      duplicateResources: 0,
      shadowedResources: shadowed,
    };
  });
  const resourceCount = contributions.reduce((sum, item) => sum + item.effectiveResources, 0);
  const overridden = contributions.reduce((sum, item) => sum + item.shadowedResources, 0);
  return {
    planId: `sha256:demo${packageIds.join("").length}`,
    packageId: packageIds[0] ?? "",
    packageIds,
    sourceCommit: "3a85572029f2c264e2a17cee1c9b54ce93e4fd93",
    targetFile: "pak66_dir.vpk",
    resourceCount,
    resourceBytes: resourceCount * 2048,
    vpkBytes: resourceCount * 1310 + 4096,
    vpkSha256: "d3m0c0ffee5a1t" + "0".repeat(50),
    bundlePlanId: "demo",
    packageCount: packageIds.length,
    duplicateResources: 0,
    overriddenResources: overridden,
    duplicates: [],
    overrides: overridden
      ? [
          {
            path: "materials/hud/minimap_overlay.vtex_c",
            winnerPackageId: packageIds[0],
            shadowedPackageIds: packageIds.slice(1, 2),
          },
        ]
      : [],
    contributions,
    compatibility: "unknown",
    distribution: "internal_pilot",
    deployEnabled: true,
  };
}

let stressPoint: "after_prepared" | "after_replace" | null = null;
let steamStress = false;
let download: (TreePilotDownloadStatus & { startedAt: number; ids: string[] }) | null = null;
let installed: {
  operationId: string;
  language: GameLanguage;
  packageIds: string[];
  steamOperationId: string | null;
} | null = null;

function downloadStatus(): TreePilotDownloadStatus | null {
  if (!download) return null;
  if (download.phase !== "downloading" && download.phase !== "verifying") return download;
  const elapsed = Date.now() - download.startedAt;
  const total = download.totalResources;
  const verified = Math.min(total, Math.round((elapsed / 4200) * total));
  if (settings.scenario === "download_fails" && verified > total * 0.45) {
    download = {
      ...download,
      phase: "failed",
      verifiedResources: verified,
      errorCode: "download_transport_failed",
    };
    return download;
  }
  if (verified >= total) {
    download = {
      ...download,
      phase: "ready",
      verifiedResources: total,
      plan: demoPlan(download.ids),
    };
    return download;
  }
  return {
    ...download,
    phase: verified > total * 0.7 ? "verifying" : "downloading",
    verifiedResources: verified,
  };
}

export const demoBridge: EngineBridge = {
  ...mockEngine,
  async inspectRuntime() {
    return { platformSupported: true, steamRunning: true, dotaRunning: false, patchReady: true };
  },
  async prepareRuntimeForPatch() {
    await wait(900);
    if (settings.scenario === "dota_open")
      throw new EngineFault("runtime_busy", "prepare_runtime_for_patch");
    return { platformSupported: true, steamRunning: false, dotaRunning: false, patchReady: true };
  },
  async listSteamProfiles() {
    return [{ profileToken: "demo-profile", profileIndex: 1, status: "ready" }];
  },
  async beginTreePilotDownload(packageIds) {
    const total = packageIds.reduce((sum, id) => sum + resourcesFor(engineIdFor(id)), 0);
    download = {
      phase: "downloading",
      verifiedResources: 0,
      totalResources: total,
      errorCode: null,
      plan: null,
      startedAt: Date.now(),
      ids: packageIds,
    };
    return downloadStatus()!;
  },
  async treePilotDownloadStatus() {
    return downloadStatus();
  },
  async cancelTreePilotDownload() {
    download = download ? { ...download, phase: "cancelled" } : null;
    return download!;
  },
  async previewTreeLanguage() {
    await wait(300);
  },
  async previewSteamLaunchOptions(profileToken, language = "russian") {
    return {
      profileToken,
      language,
      changed: true,
      beforeSha256: "demo-before",
      afterSha256: "demo-after",
      confirmationToken: "demo",
    };
  },
  async installTreePilot(_path, ids, _plan, language) {
    await wait(2400);
    installed = {
      operationId: `demo-${Date.now()}`,
      language,
      packageIds: ids.map(engineIdFor),
      steamOperationId: null,
    };
    return {
      operationId: installed.operationId,
      language,
      beforeSha256: null,
      installedSha256: "demo",
      backupVerified: true,
      committed: true,
      rolledBack: false,
      bundlePlanId: "demo",
      packageIds: installed.packageIds,
    };
  },
  async applyTreeSteamLaunchOptions(request) {
    await wait(1100);
    if (settings.scenario === "steam_fails")
      throw new EngineFault("steam_config_plan_stale", "apply_tree_steam_launch_options");
    const operationId = `demo-steam-${Date.now()}`;
    if (installed) installed.steamOperationId = operationId;
    return {
      operationId,
      profileToken: request.profileToken,
      changed: true,
      beforeSha256: "demo-before",
      afterSha256: "demo-after",
      backupVerified: true,
      committed: true,
      rolledBack: false,
    };
  },
  async startSteamAfterTreePilot() {
    await wait(900);
    return { platformSupported: true, steamRunning: true, dotaRunning: false, patchReady: false };
  },
  async startSteamAfterRestore() {
    await wait(700);
    return { platformSupported: true, steamRunning: true, dotaRunning: false, patchReady: false };
  },
  async currentTreePilot() {
    if (!installed) return null;
    return {
      operationId: installed.operationId,
      language: installed.language,
      beforeSha256: null,
      installedSha256: "demo",
      backupVerified: true,
      committed: true,
      rolledBack: false,
      bundlePlanId: "demo",
      packageIds: installed.packageIds,
      packageVerified: true,
      steamOperationId: installed.steamOperationId,
      steamProfileToken: installed.steamOperationId ? "demo-profile" : null,
      steamRecoveryRequired: false,
      profile: null,
      dotaPatched: false,
    };
  },
  async rollbackSteamLaunchOptions(operationId) {
    await wait(700);
    if (installed) installed.steamOperationId = null;
    return {
      operationId,
      profileToken: "demo-profile",
      changed: true,
      beforeSha256: "demo-before",
      afterSha256: "demo-after",
      backupVerified: true,
      committed: true,
      rolledBack: true,
    };
  },
  async rollbackGameDeployment(_path, operationId) {
    await wait(1600);
    const language = installed?.language ?? "russian";
    installed = null;
    download = null;
    return {
      operationId,
      language,
      beforeSha256: null,
      installedSha256: "demo",
      backupVerified: true,
      committed: true,
      rolledBack: true,
      bundlePlanId: "demo",
      packageIds: [],
    };
  },
  async recoverGameDeployments() {
    await wait(800);
    const point = stressPoint;
    stressPoint = null;
    if (point === "after_prepared") return { inspected: 1, rolledBack: 0, markedFailed: 1 };
    if (point === "after_replace") return { inspected: 1, rolledBack: 1, markedFailed: 0 };
    return { inspected: 0, rolledBack: 0, markedFailed: 0 };
  },
  async recoverSteamLaunchOptions() {
    if (!steamStress) return [];
    steamStress = false;
    return [
      {
        operationId: "demo-steam-stress",
        profileToken: "demo-profile",
        changed: true,
        beforeSha256: "demo-before",
        afterSha256: "demo-after",
        backupVerified: true,
        committed: false,
        rolledBack: true,
      },
    ];
  },
  async saveTreePilotEvidence() {
    return { fileName: "betterfy-demo-report.json", entries: 1 };
  },
  async treePilotStressCapabilities() {
    return settings.developer
      ? { enabled: true, failurePoints: ["after_prepared", "after_replace"] }
      : { enabled: false, failurePoints: [] };
  },
  async installTreePilotStress(_path, _ids, _plan, _language, failurePoint) {
    await wait(1400);
    if (!settings.developer)
      throw new EngineFault("stress_test_disabled", "install_tree_pilot_stress");
    stressPoint = failurePoint;
    throw new EngineFault("injected_failure", "install_tree_pilot_stress");
  },
  async applyTreeSteamLaunchOptionsStress() {
    await wait(900);
    if (!settings.developer)
      throw new EngineFault("stress_test_disabled", "apply_tree_steam_launch_options_stress");
    steamStress = true;
    throw new EngineFault("injected_failure", "apply_tree_steam_launch_options_stress");
  },
};
