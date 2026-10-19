import { useSyncExternalStore } from "react";
import {
  EngineFault,
  mockEngine,
  type EngineBridge,
  type GameLanguage,
  type TreePilotDownloadStatus,
  type TreePilotPlan,
  type WardrobePlanItem,
} from "../../engine";
import { getStorageItem, setStorageItem } from "../../storage";
import { engineIdFor, findPackage, isSharedKept, isWardrobePackage } from "../packages";

// The browser preview has no Rust engine. To show the whole builder there,
// this bridge plays the Windows flow with timers: nothing is downloaded and
// no file is written. Every screen that uses it says "Demo".

export const isDesktopRuntime = () =>
  typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;
export const demoActive = () => !isDesktopRuntime();

export type DemoScenario =
  | "success"
  | "dota_open"
  | "download_fails"
  | "steam_fails"
  | "game_updated"
  | "minify_replaced"
  | "minify_beside";

type DemoSettings = { scenario: DemoScenario; developer: boolean };
const key = "betterfy:builder-demo";
let settings: DemoSettings = (() => {
  try {
    const stored = JSON.parse(getStorageItem(key) ?? "{}");
    return {
      scenario: [
        "success",
        "dota_open",
        "download_fails",
        "steam_fails",
        "game_updated",
        "minify_replaced",
        "minify_beside",
      ].includes(
        stored.scenario,
      )
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

// Shaped like the one archive audited so far. The demo does not run the
// analyzer: these lists and counts only show how the review looks.
const demoShared = [
  ...[
    "default_color_tga_11bf3739",
    "default_color_tga_47b1af3d",
    "default_colorwarp3d_z000_tga_8805a83f",
    "default_detail2_tga_70788555",
    "default_detail_tga_d55685d8",
    "default_detailmask_tga_4450da58",
    "default_diffusewarp_tga_7b504261",
    "default_eyes_cube_psd_f281aa41",
    "default_fresnelwarprim_tga_d9279d65",
    "default_mask_tga_4528a0a9",
    "default_mask_tga_7a9c85be",
    "default_normal_tga_7652cb",
    "default_refl_tga_c1b9c83e",
    "default_specwarp_tga_436583b9",
    "dota_default_cube_tga_c95513b9",
  ].map((name) => `materials/default/${name}.vtex_c`),
  ...["glossy_cube_tga_4cca90d9", "gold_day_cube_psd_c6b0aa8a", "glossy_red_cube_tga_82cb7b24"].map(
    (name) => `materials/models/cubemaps/${name}.vtex_c`,
  ),
  ...["glow", "rope", "rope_energy", "smoke", "trail"].map(
    (name) => `materials/particle/basic_${name}.vtex_c`,
  ),
  "materials/models/particle/net_color.vmat_c",
  ...["ambient", "explosion", "projectile", "rope", "trail"].map(
    (name) => `particles/basic_${name}/basic_${name}.vpcf_c`,
  ),
  "particles/models/basic_trail/basic_trail.vpcf_c",
  "particles/status_fx/status_effect_keeper_spirit_form.vpcf_c",
];

// The demo's one skin in both modes: everything the archive carries (the
// default) or without the shared game files (the opt-in). Class counts are
// illustrative; the install totals are the audited ones.
const DEMO_SKIN = {
  heroScoped: { files: 214, bytes: 1343162 },
  shared: { files: 27, bytes: 4147352 },
  install: { files: 996, bytes: 57855637 },
  strip: { files: 969, bytes: 53708285 },
};

// A pack with no hero (a courier, a sound pack): installs whole, no opt-in mode.
const DEMO_PACK = {
  shared: { files: 3, bytes: 61440 },
  install: { files: 24, bytes: 1887436 },
};

function demoPackItem(packageId: string, category: string): WardrobePlanItem {
  const sharedPaths = demoShared
    .slice(0, DEMO_PACK.shared.files)
    .map((path) => ({ path, bytes: 4096 }));
  return {
    packageId,
    catalogId: findPackage(packageId)?.catalogId ?? packageId,
    category,
    scope: "none",
    heroes: [],
    sourceUrls: [
      "https://raw.githubusercontent.com/h6rd/Dota2PornFxWeb/main/assets/files/",
      "https://huggingface.co/datasets/hrdq/Dota2PornFx/",
    ],
    archiveSha256: "3f2b6a1c9d8e7f405162738495a6b7c8d9e0f1a2b3c4d5e6f708192a3b4c5d6e",
    catalogSnapshot: "demo",
    permission: "founder_reported_author_consent",
    verifiedLanguages: [],
    compatibilityNote: null,
    sharedPolicy: "install",
    gameChecked: true,
    report: {
      heroes: [],
      sharedPolicy: "install",
      entries: 25,
      totalBytes: 1900000,
      archiveIdentity: "a1".repeat(32),
      heroScoped: { files: 0, bytes: 0 },
      authorOwned: {
        files: DEMO_PACK.install.files - DEMO_PACK.shared.files,
        bytes: DEMO_PACK.install.bytes - DEMO_PACK.shared.bytes,
      },
      shared: DEMO_PACK.shared,
      rejected: { files: 0, bytes: 0 },
      ignored: { files: 1, bytes: 120 },
      renamed: 0,
      imageWrappers: 0,
      rejectedPaths: [{ path: "models/courier/readme.txt", reason: "not_game_file", bytes: 120 }],
      sharedPaths,
      newInGame: [],
      hostile: false,
      install: {
        files: DEMO_PACK.install.files,
        bytes: DEMO_PACK.install.bytes,
        identity: "b2".repeat(32),
        strippedShared: 0,
        blocked: null,
      },
    },
  };
}

function demoWardrobeItem(packageId: string): WardrobePlanItem {
  const manifest = findPackage(packageId);
  if (manifest?.scope === "none") return demoPackItem(packageId, manifest.category ?? "couriers");
  const keepShared = isSharedKept(packageId);
  const sharedPaths = demoShared
    .slice(0, DEMO_SKIN.shared.files)
    .map((path) => ({ path, bytes: 4096 }));
  const kept = keepShared ? DEMO_SKIN.strip : DEMO_SKIN.install;
  return {
    packageId,
    catalogId: findPackage(packageId)?.catalogId ?? packageId,
    category: "heroes",
    scope: "hero",
    heroes: ["keeper_of_the_light"],
    sourceUrls: [
      "https://raw.githubusercontent.com/h6rd/Dota2PornFxWeb/main/assets/files/heroes/",
      "https://huggingface.co/datasets/hrdq/Dota2PornFx/",
    ],
    archiveSha256: "f9f0c2f63a9989ea6fd2779cfea1676a2e07bd94eabab9e0c2127a68e7e268a7",
    catalogSnapshot: "demo",
    permission: "founder_reported_author_consent",
    verifiedLanguages: [],
    compatibilityNote: null,
    sharedPolicy: keepShared ? "strip" : "install",
    gameChecked: true,
    report: {
      heroes: ["keeper_of_the_light"],
      sharedPolicy: keepShared ? "strip" : "install",
      entries: 1010,
      totalBytes: 58047211,
      archiveIdentity: "6dfba0d998fb954c36a1bb7a844e92b7bf6805d5f691ff9560525d86bbe70e57",
      heroScoped: DEMO_SKIN.heroScoped,
      authorOwned: {
        files: DEMO_SKIN.install.files - DEMO_SKIN.heroScoped.files - DEMO_SKIN.shared.files,
        bytes:
          DEMO_SKIN.install.bytes - DEMO_SKIN.heroScoped.bytes - DEMO_SKIN.shared.bytes,
      },
      shared: DEMO_SKIN.shared,
      rejected: { files: 0, bytes: 0 },
      ignored: { files: 2, bytes: 20000 },
      renamed: 0,
      imageWrappers: 11,
      rejectedPaths: [
        { path: "particles/darkness/flame.vpcf", reason: "uncompiled_source", bytes: 7152 },
        { path: "particles/darkness/sparks.vpcf", reason: "uncompiled_source", bytes: 31744 },
      ],
      sharedPaths,
      newInGame: [],
      hostile: false,
      install: {
        files: kept.files,
        bytes: kept.bytes,
        identity: keepShared
          ? "371151d7332ba44baf2dce9ad86a32bbaa23aa19a5730be418607d630adbe521"
          : "c9c9ac4bc822df19c6daf8b01ffecbf44c09dde8f051423a0ab3273ee5da0dc1",
        strippedShared: keepShared ? sharedPaths.length : 0,
        blocked: null,
      },
    },
  };
}

function demoSkinBytes(packageIds: string[]) {
  return packageIds.filter(isWardrobePackage).reduce((total, id) => {
    if (findPackage(id)?.scope === "none") return total + DEMO_PACK.install.bytes;
    return total + (isSharedKept(id) ? DEMO_SKIN.strip.bytes : DEMO_SKIN.install.bytes);
  }, 0);
}

function demoPlan(ids: string[]): TreePilotPlan {
  const packageIds = ids.map(engineIdFor);
  const wardrobe = packageIds.filter(isWardrobePackage).map(demoWardrobeItem);
  const contributions = packageIds.map((packageId, index) => {
    const input = isWardrobePackage(packageId)
      ? findPackage(packageId)?.scope === "none"
        ? DEMO_PACK.install.files
        : isSharedKept(packageId)
          ? DEMO_SKIN.strip.files
          : DEMO_SKIN.install.files
      : resourcesFor(packageId);
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
    resourceBytes: resourceCount * 2048 + demoSkinBytes(packageIds),
    vpkBytes: resourceCount * 1310 + 4096 + demoSkinBytes(packageIds),
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
    wardrobe,
    // A second selected item that writes a path the skin writes: the demo
    // shows the confirmation the engine would ask for.
    wardrobeConflicts:
      wardrobe.length > 0 && packageIds.length > wardrobe.length
        ? [
            {
              path: "materials/models/heroes/keeper_of_the_light/keeper_body.vmat_c",
              winnerPackageId: packageIds[0],
              shadowedPackageIds: packageIds.slice(1, 2),
            },
          ]
        : [],
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
  // The founder's Steam reports Russian for Dota, so the demo does too.
  async detectDotaLanguage() {
    await wait(450);
    return { steamLanguage: "russian", language: "russian" };
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
  // The language folder as the demo scenarios see it: empty, with Minify's
  // pak66 beside BetterFy's pak65, or with BetterFy's file replaced.
  async inspectLanguageSlot(_path, language = "russian") {
    await wait(200);
    const day = 24 * 60 * 60 * 1000;
    const minify = [
      { name: "pak66_000.vpk", bytes: 41_943_040, modifiedMs: Date.now() - day, owner: "minify_like" as const },
      { name: "pak66_dir.vpk", bytes: 18_432, modifiedMs: Date.now() - day, owner: "minify_like" as const },
    ];
    if (settings.scenario === "minify_replaced")
      return {
        language,
        folderExists: true,
        files: minify,
        filesTruncated: false,
        owned: { name: "pak66_dir.vpk", state: "replaced" as const },
        nextName: "pak65_dir.vpk",
        slotFull: false,
      };
    if (settings.scenario === "game_updated")
      return {
        language,
        folderExists: true,
        files: [
          { name: "pak65_dir.vpk", bytes: 9_437_184, modifiedMs: Date.now() - 3600_000, owner: "unknown" as const },
        ],
        filesTruncated: false,
        owned: { name: "pak65_dir.vpk", state: "replaced" as const },
        nextName: "pak64_dir.vpk",
        slotFull: false,
      };
    const ours = installed
      ? [
          {
            name: "pak65_dir.vpk",
            bytes: 7_340_032,
            modifiedMs: Date.now(),
            owner: "betterfy" as const,
          },
        ]
      : [];
    const beside = settings.scenario === "minify_beside" ? minify : [];
    return {
      language,
      folderExists: beside.length + ours.length > 0,
      files: [...ours, ...beside].sort((a, b) => a.name.localeCompare(b.name)),
      filesTruncated: false,
      owned: installed ? { name: "pak65_dir.vpk", state: "intact" as const } : null,
      nextName: installed ? null : "pak65_dir.vpk",
      slotFull: false,
    };
  },
  async installTreePilot(_path, ids, _plan, language) {
    if (settings.scenario === "game_updated")
      throw new EngineFault("deployment_conflict", "install_tree_pilot");
    if (settings.scenario === "minify_replaced")
      throw new EngineFault("deployment_target_foreign", "install_tree_pilot");
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
      vpkName: "pak65_dir.vpk",
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
      vpkName: "pak65_dir.vpk",
      packageIds: installed.packageIds,
      packageVerified: true,
      steamOperationId: installed.steamOperationId,
      steamProfileToken: installed.steamOperationId ? "demo-profile" : null,
      steamRecoveryRequired: false,
      profile: null,
      dotaPatched: false,
    };
  },
  async releaseStaleDeployment() {
    await wait(900);
    if (settings.scenario === "game_updated") setDemoSettings({ scenario: "success" });
    else if (settings.scenario === "minify_replaced") setDemoSettings({ scenario: "minify_beside" });
    return { operationId: "demo-stale", language: "russian", reason: "changed" };
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
      vpkName: "pak65_dir.vpk",
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
