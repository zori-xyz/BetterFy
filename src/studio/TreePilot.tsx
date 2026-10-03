import { useEffect, useState } from "react";
import {
  ArrowRight,
  Check,
  Copy,
  Download,
  FileCheck2,
  FlaskConical,
  LoaderCircle,
  RotateCcw,
  ShieldCheck,
  TriangleAlert,
} from "lucide-react";
import {
  EngineFault,
  engineBridge,
  type DeploymentEvidenceReport,
  type DeploymentStressFailurePoint,
  type GameInstallation,
  type GameLanguage,
  type SteamProfileSummary,
  type TreePilotDownloadStatus,
  type TreePilotPlan,
} from "../engine";
import { useLocale } from "../i18n";
import { deliveryLabel, modById } from "./model";
import {
  MAX_BUNDLE_PACKAGES,
  engineIdFor,
  findPackage,
  bundleRelationProblem,
  installablePackages,
  packageName,
} from "./packages";
import { setEngineActive } from "./engineActivity";

const codeOf = (error: unknown) =>
  error instanceof EngineFault
    ? error.code
    : error instanceof Error
      ? error.message
      : String(error);
const explainError = (code: string, isRu: boolean) => {
  const messages: Record<string, [string, string]> = {
    runtime_busy: ["Закрой Dota 2 и Steam, затем повтори.", "Close Dota 2 and Steam, then retry."],
    dota_close_unavailable: [
      "Не удалось мягко закрыть Dota 2. Закрой игру вручную и повтори.",
      "Dota 2 could not be closed gracefully. Close it manually and retry.",
    ],
    shutdown_timeout: [
      "Dota 2 или Steam не завершились за 30 секунд. Закрой их вручную и повтори.",
      "Dota 2 or Steam did not close within 30 seconds. Close them manually and retry.",
    ],
    steam_start_failed: [
      "Файл установлен, но Steam не запустился. Запусти Steam вручную и проверь параметры запуска.",
      "The file was installed, but Steam did not start. Open Steam manually and check launch options.",
    ],
    steam_start_timeout: [
      "Файл установлен, но BetterFy не дождался запуска Steam. Проверь Steam вручную.",
      "The file was installed, but BetterFy did not detect Steam starting. Check Steam manually.",
    ],
    platform_not_supported: [
      "Установка доступна только на Windows.",
      "Installation is Windows-only.",
    ],
    download_dns_failed: [
      "Нет доступа к источнику. Проверь подключение к сети.",
      "The source is unreachable. Check your connection.",
    ],
    download_transport_failed: [
      "Не удалось скачать ресурс. Повтори подготовку — проверенные файлы сохранятся.",
      "A resource could not be downloaded. Retry; verified files are kept.",
    ],
    package_conflict: [
      "Эти моды конфликтуют между собой. Оставь в сборке только один из них.",
      "These mods conflict. Keep only one of them in the build.",
    ],
    package_dependency_missing: [
      "Одному из модов нужен другой мод. Добавь его в сборку.",
      "One of the mods needs another mod. Add it to the build.",
    ],
    game_path_required: [
      "Для этой сборки нужна подключённая Dota 2: BetterFy сверяется с файлами твоей игры.",
      "This build needs a connected Dota 2: BetterFy checks it against your game files.",
    ],
    game_archive_unreadable: [
      "Не удалось прочитать архивы Dota 2. Проверь файлы игры в Steam и повтори.",
      "Dota 2 archives could not be read. Verify game files in Steam and retry.",
    ],
    blacklist_resolution_missing: [
      "Сборку нужно подготовить заново: список изменений для этой версии игры не найден.",
      "Prepare the build again: the change list for this game version is missing.",
    ],
    blacklist_resolution_stale: [
      "Сборку нужно подготовить заново: список изменений устарел.",
      "Prepare the build again: the change list is out of date.",
    ],
    build_plan_stale: [
      "План устарел. Подготовь сборку заново.",
      "The plan is stale. Prepare the build again.",
    ],
    deployment_target_foreign: [
      "Целевой файл занят другой модификацией. BetterFy не будет его заменять.",
      "Another modification owns the target. BetterFy will not overwrite it.",
    ],
    deployment_language_change_requires_restore: [
      "Сначала откати установленную сборку, затем выбери другой язык.",
      "Restore the installed build before choosing another language.",
    ],
    language_folder_unavailable: [
      "В установке Dota нет папки выбранного языка с gameinfo.gi. Выбери другой язык или восстанови файлы игры через Steam.",
      "This Dota installation lacks the selected language folder with gameinfo.gi. Choose another language or verify game files in Steam.",
    ],
    launch_option_conflict: [
      "В Steam уже указан другой язык. Восстанови прежние параметры BetterFy или измени их вручную перед установкой.",
      "Steam already specifies a different language. Restore BetterFy's previous settings or change them manually before installing.",
    ],
    deployment_conflict: [
      "Файл игры изменился после установки. Автоматический откат остановлен.",
      "The game file changed after installation. Automatic restore was stopped.",
    ],
    backup_failed: [
      "Резервная копия BetterFy недоступна. Автоматический откат заблокирован; не заменяй файл вручную, пока не проверишь состояние установки.",
      "BetterFy's backup is unavailable. Automatic restore is blocked; inspect the installation before changing the file manually.",
    ],
    backup_verification_failed: [
      "Резервная копия не прошла проверку. BetterFy не будет восстанавливать её поверх файла игры.",
      "The backup failed verification. BetterFy will not restore it over the game file.",
    ],
    rollback_conflict: [
      "Файл игры изменился после установки. Автоматический откат остановлен.",
      "The game file changed after installation. Automatic restore was stopped.",
    ],
    steam_profile_conflict: [
      "Этот Steam-профиль нельзя менять автоматически. Выбери другой или проверь параметры запуска вручную.",
      "This Steam profile cannot be changed automatically. Choose another or inspect its launch options.",
    ],
    steam_activation_not_ready: [
      "Параметры Steam не подтверждены. Не считай мод активным в игре.",
      "Steam settings were not verified. Do not assume the mod is active in game.",
    ],
    steam_recovery_required: [
      "Найдено прерванное изменение Steam. Сначала восстанови его в диагностике.",
      "An interrupted Steam change was found. Recover it in diagnostics first.",
    ],
    clipboard_unavailable: [
      "Не удалось скопировать параметр. Выдели и скопируй его вручную.",
      "Could not copy the option. Select and copy it manually.",
    ],
    stress_requires_unmanaged_profile: [
      "Для этого теста выбери Steam-профиль, где BetterFy ещё не добавлял выбранный язык.",
      "Choose a Steam profile where BetterFy has not already added the selected language.",
    ],
    stress_recovery_unverified: [
      "Тестовое восстановление не дало ожидаемого подтверждения. Не запускай Dota и скопируй отчёт.",
      "Test recovery did not produce the expected proof. Do not launch Dota; copy the report.",
    ],
    stress_injection_did_not_fire: [
      "Тестовая точка отказа не сработала. Обычная установка не подтверждена этим тестом.",
      "The test failure point did not fire. This test did not verify the normal installation.",
    ],
    stress_test_disabled: [
      "Контролируемый стресс-тест доступен только во внутренней Windows-сборке.",
      "Controlled stress testing is available only in the internal Windows build.",
    ],
  };
  return (
    messages[code]?.[isRu ? 0 : 1] ??
    (isRu
      ? `Операция остановлена (${code}). Проверь состояние установки перед запуском игры.`
      : `Operation stopped (${code}). Check installation state before launching the game.`)
  );
};

export default function TreePilot({
  ids,
  installation,
  preview,
}: {
  ids: string[];
  installation: GameInstallation;
  preview: boolean;
}) {
  const { isRu, language } = useLocale();
  const [plan, setPlan] = useState<TreePilotPlan | null>(null);
  const [operationId, setOperationId] = useState<string | null>(null);
  const [verifiedInstall, setVerifiedInstall] = useState(false);
  const [packageVerified, setPackageVerified] = useState(false);
  const [steamOperationId, setSteamOperationId] = useState<string | null>(null);
  const [steamRecoveryRequired, setSteamRecoveryRequired] = useState(false);
  const [phase, setPhase] = useState<"idle" | "download" | "install" | "steam" | "restore">("idle");
  const [verifiedResources, setVerifiedResources] = useState(0);
  const [totalResources, setTotalResources] = useState(0);
  const [recovering, setRecovering] = useState(false);
  const [recoveryMessage, setRecoveryMessage] = useState("");
  const [profiles, setProfiles] = useState<SteamProfileSummary[]>([]);
  const [selectedProfile, setSelectedProfile] = useState("");
  const [selectedLanguage, setSelectedLanguage] = useState<GameLanguage | "">("");
  const [installedLanguage, setInstalledLanguage] = useState<GameLanguage | null>(null);
  const [installedPackageIds, setInstalledPackageIds] = useState<string[]>([]);
  const [steamStarted, setSteamStarted] = useState(false);
  const [steamRestarted, setSteamRestarted] = useState(false);
  const [copied, setCopied] = useState(false);
  const [stressEnabled, setStressEnabled] = useState(false);
  const [stressPhase, setStressPhase] = useState<DeploymentStressFailurePoint | null>(null);
  const [stressMessage, setStressMessage] = useState("");
  const [evidence, setEvidence] = useState<DeploymentEvidenceReport | null>(null);
  const [reportCopied, setReportCopied] = useState(false);
  const [savedReport, setSavedReport] = useState("");
  const [reportError, setReportError] = useState("");
  const [error, setError] = useState("");
  const [dotaPatched, setDotaPatched] = useState(false);
  // Only work that touches game files or Steam blocks the rest of the app.
  // Set on start and cleared in each operation's `finally`, so leaving this
  // screen mid-operation cannot clear it early.
  useEffect(() => {
    if (phase === "install" || phase === "steam" || phase === "restore" || recovering)
      setEngineActive(true);
  }, [phase, recovering]);
  const catalogName = (id: string) => modById.get(id)?.name[language];
  const packageLabel = (id: string) => packageName(id, language);
  const supportedBundle =
    ids.length > 0 &&
    ids.length <= Math.min(installablePackages.length, MAX_BUNDLE_PACKAGES) &&
    ids.every((id) => findPackage(id) !== undefined) &&
    new Set(ids).size === ids.length &&
    !bundleRelationProblem(ids);
  const relation = bundleRelationProblem(ids);
  const bundleName = !ids.length
    ? isRu
      ? "Установленная сборка"
      : "Installed build"
    : ids.length > 1
      ? isRu
        ? `Сборка · ${ids.length} мода`
        : `Build · ${ids.length} mods`
      : (catalogName(ids[0]) ?? "Tree Mod");
  const windows = /Windows/i.test(navigator.userAgent);
  const desktop = "__TAURI_INTERNALS__" in window;
  // Managing an existing installation (restore, Steam, recovery, evidence)
  // must not depend on what is currently selected: changing the selection
  // after installing would otherwise hide the only way back.
  const canManage = desktop && windows && !preview && installation.verified;
  const canInstall = canManage && supportedBundle;
  // Packages whose manifest records no in-game verification yet.
  const unverified = ids.filter((id) => !findPackage(id)?.verifiedLanguages?.length);
  function applyDownloadStatus(status: TreePilotDownloadStatus | null) {
    if (!status) return;
    setVerifiedResources(status.verifiedResources);
    setTotalResources(status.totalResources);
    if (status.phase === "ready") {
      const expectedIds = ids.map(engineIdFor);
      if (
        status.plan?.planId.startsWith("sha256:") &&
        status.plan.packageIds.join("|") === expectedIds.join("|")
      ) {
        setPlan(status.plan);
        setError("");
      } else {
        setPlan(null);
        setError("");
      }
      setPhase("idle");
    } else if (status.phase === "failed" || status.phase === "cancelled") {
      setError(status.phase === "cancelled" ? "" : (status.errorCode ?? "download_failed"));
      setPhase("idle");
    } else setPhase("download");
  }
  useEffect(() => {
    if (!desktop) return;
    if (!operationId) {
      setPlan(null);
      setVerifiedResources(0);
      setTotalResources(0);
    }
    let active = true;
    engineBridge
      .treePilotDownloadStatus()
      .then((status) => {
        if (active) applyDownloadStatus(status);
      })
      .catch(() => undefined);
    return () => {
      active = false;
    };
  }, [desktop, ids.join("|"), operationId]);
  useEffect(() => {
    if (phase !== "download") return;
    let active = true;
    const timer = window.setInterval(() => {
      engineBridge
        .treePilotDownloadStatus()
        .then((status) => {
          if (active) applyDownloadStatus(status);
        })
        .catch((cause) => {
          if (active) {
            setError(codeOf(cause));
            setPhase("idle");
          }
        });
    }, 450);
    return () => {
      active = false;
      window.clearInterval(timer);
    };
  }, [phase]);
  useEffect(() => {
    if (!desktop || !windows || preview || !installation.verified) return;
    let active = true;
    engineBridge
      .currentTreePilot(installation.path)
      .then((receipt) => {
        if (!active) return;
        setOperationId(receipt?.operationId ?? null);
        setInstalledLanguage(receipt?.language ?? null);
        setInstalledPackageIds(receipt?.packageIds ?? []);
        setVerifiedInstall(Boolean(receipt));
        setPackageVerified(receipt?.packageVerified ?? false);
        setSteamOperationId(receipt?.steamOperationId ?? null);
        if (receipt?.steamProfileToken) setSelectedProfile(receipt.steamProfileToken);
        setSteamRecoveryRequired(receipt?.steamRecoveryRequired ?? false);
        setDotaPatched(receipt?.dotaPatched ?? false);
      })
      .catch((cause) => {
        if (active) setError(codeOf(cause));
      });
    return () => {
      active = false;
    };
  }, [desktop, windows, preview, installation.verified, installation.path]);
  useEffect(() => {
    if (!canManage) return;
    let active = true;
    engineBridge
      .listSteamProfiles()
      .then((found) => {
        if (!active) return;
        setProfiles(found);
        setSelectedProfile((previous) =>
          found.some((profile) => profile.profileToken === previous)
            ? previous
            : (found.find((profile) => profile.status === "ready")?.profileToken ??
              found.find((profile) => profile.status === "already_managed")?.profileToken ??
              ""),
        );
      })
      .catch((cause) => {
        if (active) setError(codeOf(cause));
      });
    return () => {
      active = false;
    };
  }, [canManage]);
  useEffect(() => {
    if (!canManage) return;
    let active = true;
    engineBridge
      .treePilotStressCapabilities()
      .then((capabilities) => {
        if (active) setStressEnabled(capabilities.enabled);
      })
      .catch(() => {
        if (active) setStressEnabled(false);
      });
    engineBridge
      .collectTreePilotEvidence(installation.path)
      .then((report) => {
        if (active) setEvidence(report);
      })
      .catch(() => undefined);
    return () => {
      active = false;
    };
  }, [canManage, installation.path]);

  async function prepare() {
    if (!desktop || phase !== "idle") return;
    setError("");
    setPhase("download");
    try {
      applyDownloadStatus(
        await engineBridge.beginTreePilotDownload(
          ids,
          installation.verified ? installation.path : undefined,
        ),
      );
    } catch (cause) {
      setError(codeOf(cause));
      setPhase("idle");
    }
  }

  async function cancelPrepare() {
    try {
      await engineBridge.cancelTreePilotDownload();
    } catch (cause) {
      setError(codeOf(cause));
    }
  }

  async function install() {
    if (!plan || !canInstall || !selectedLanguage || phase !== "idle") return;
    setError("");
    setPhase("install");
    let installed = false;
    try {
      await engineBridge.previewTreeLanguage(installation.path, selectedLanguage);
      if (selectedProfile) {
        const profile = profiles.find((item) => item.profileToken === selectedProfile);
        if (!profile || !["ready", "already_managed"].includes(profile.status))
          throw new Error("steam_profile_conflict");
        await engineBridge.previewSteamLaunchOptions(selectedProfile, selectedLanguage);
      }
      const runtime = await engineBridge.prepareRuntimeForPatch();
      if (!runtime.patchReady) throw new Error("runtime_busy");
      const receipt = await engineBridge.installTreePilot(
        installation.path,
        ids,
        plan.planId,
        selectedLanguage,
      );
      if (!receipt.committed || !receipt.backupVerified) throw new Error("deployment_unverified");
      installed = true;
      setDotaPatched(false);
      setOperationId(receipt.operationId);
      setInstalledLanguage(receipt.language);
      setInstalledPackageIds(receipt.packageIds);
      setVerifiedInstall(true);
      setPackageVerified(true);
      setPhase("steam");
      if (selectedProfile) {
        const profile = profiles.find((item) => item.profileToken === selectedProfile);
        if (!profile || !["ready", "already_managed"].includes(profile.status))
          throw new Error("steam_profile_conflict");
        const steamPlan = await engineBridge.previewSteamLaunchOptions(
          selectedProfile,
          receipt.language,
        );
        const steamReceipt = await engineBridge.applyTreeSteamLaunchOptions({
          gamePath: installation.path,
          deploymentOperationId: receipt.operationId,
          profileToken: selectedProfile,
          confirmationToken: steamPlan.confirmationToken,
          language: receipt.language,
        });
        if (!steamReceipt.committed || (steamReceipt.changed && !steamReceipt.backupVerified))
          throw new Error("steam_activation_not_ready");
        if (steamReceipt.operationId) {
          setSteamOperationId(steamReceipt.operationId);
        }
        await engineBridge.startSteamAfterTreePilot({
          gamePath: installation.path,
          operationId: receipt.operationId,
          profileToken: selectedProfile,
          steamOperationId: steamReceipt.operationId,
        });
        setSteamStarted(true);
        setSteamRestarted(true);
      } else {
        await engineBridge.startSteamAfterTreePilot({
          gamePath: installation.path,
          operationId: receipt.operationId,
        });
        setSteamRestarted(true);
      }
    } catch (cause) {
      setError(codeOf(cause));
    } finally {
      setEngineActive(false);
      setPhase("idle");
      if (installed) saveEvidenceReport();
    }
  }

  async function restore() {
    if (!operationId || !canManage || phase !== "idle") return;
    setError("");
    setPhase("restore");
    try {
      if (steamRecoveryRequired) throw new Error("steam_recovery_required");
      const runtime = await engineBridge.prepareRuntimeForPatch();
      if (!runtime.patchReady) throw new Error("runtime_busy");
      if (steamOperationId) {
        const steamReceipt = await engineBridge.rollbackSteamLaunchOptions(steamOperationId);
        if (!steamReceipt.rolledBack) throw new Error("steam_rollback_failed");
        setSteamOperationId(null);
      }
      const receipt = await engineBridge.rollbackGameDeployment(installation.path, operationId);
      if (!receipt.rolledBack) throw new Error("rollback_failed");
      setOperationId(null);
      setInstalledLanguage(null);
      setInstalledPackageIds([]);
      setVerifiedInstall(false);
      setPackageVerified(false);
      setPlan(null);
      setSelectedLanguage("");
      setSteamStarted(false);
      try {
        const restarted = await engineBridge.startSteamAfterRestore();
        setSteamRestarted(restarted.steamRunning);
        setRecoveryMessage(
          isRu
            ? "Сборка и параметры Steam восстановлены. Steam снова запущен."
            : "The build and Steam launch options were restored. Steam is running again.",
        );
      } catch {
        setSteamRestarted(false);
        setRecoveryMessage(
          isRu
            ? "Сборка восстановлена, но Steam не удалось запустить автоматически. Запусти его вручную."
            : "The build was restored, but Steam could not be started automatically. Start it manually.",
        );
      }
      setSteamRecoveryRequired(false);
      saveEvidenceReport();
    } catch (cause) {
      setError(codeOf(cause));
    } finally {
      setEngineActive(false);
      setPhase("idle");
    }
  }

  async function activateSteam() {
    if (
      !operationId ||
      !installedLanguage ||
      !canManage ||
      !selectedProfile ||
      !packageVerified ||
      steamRecoveryRequired ||
      phase !== "idle"
    )
      return;
    setError("");
    setPhase("steam");
    try {
      const profile = profiles.find((item) => item.profileToken === selectedProfile);
      if (!profile || !["ready", "already_managed"].includes(profile.status))
        throw new Error("steam_profile_conflict");
      const runtime = await engineBridge.prepareRuntimeForPatch();
      if (!runtime.patchReady) throw new Error("runtime_busy");
      if (steamOperationId) {
        await engineBridge.startSteamAfterTreePilot({
          gamePath: installation.path,
          operationId,
          profileToken: selectedProfile,
          steamOperationId,
        });
        setSteamStarted(true);
        setSteamRestarted(true);
        return;
      }
      const preview = await engineBridge.previewSteamLaunchOptions(
        selectedProfile,
        installedLanguage,
      );
      const receipt = await engineBridge.applyTreeSteamLaunchOptions({
        gamePath: installation.path,
        deploymentOperationId: operationId,
        profileToken: selectedProfile,
        confirmationToken: preview.confirmationToken,
        language: installedLanguage,
      });
      if (!receipt.committed || (receipt.changed && !receipt.backupVerified))
        throw new Error("steam_activation_not_ready");
      if (receipt.operationId) {
        setSteamOperationId(receipt.operationId);
      }
      await engineBridge.startSteamAfterTreePilot({
        gamePath: installation.path,
        operationId,
        profileToken: selectedProfile,
        steamOperationId: receipt.operationId,
      });
      setSteamStarted(true);
      setSteamRestarted(true);
    } catch (cause) {
      setError(codeOf(cause));
    } finally {
      setEngineActive(false);
      setPhase("idle");
    }
  }

  async function restoreSteam() {
    if (!steamOperationId || !canManage || phase !== "idle") return;
    setPhase("restore");
    setError("");
    try {
      const runtime = await engineBridge.prepareRuntimeForPatch();
      if (!runtime.patchReady) throw new Error("runtime_busy");
      const receipt = await engineBridge.rollbackSteamLaunchOptions(steamOperationId);
      if (!receipt.rolledBack) throw new Error("steam_rollback_failed");
      setSteamOperationId(null);
      setSteamStarted(false);
    } catch (cause) {
      setError(codeOf(cause));
    } finally {
      setEngineActive(false);
      setPhase("idle");
    }
  }

  async function copyLaunchOption() {
    try {
      if (!installedLanguage) return;
      await navigator.clipboard.writeText(`-language ${installedLanguage}`);
      setCopied(true);
      window.setTimeout(() => setCopied(false), 2200);
    } catch {
      setError("clipboard_unavailable");
    }
  }

  async function recover() {
    if (!canManage || phase !== "idle" || recovering) return;
    setError("");
    setRecoveryMessage("");
    setRecovering(true);
    try {
      if (steamRecoveryRequired) {
        const runtime = await engineBridge.prepareRuntimeForPatch();
        if (!runtime.patchReady) throw new Error("runtime_busy");
        await engineBridge.recoverSteamLaunchOptions();
      }
      const result = await engineBridge.recoverGameDeployments(installation.path);
      setRecoveryMessage(
        result.inspected === 0
          ? isRu
            ? "Незавершённых операций не найдено."
            : "No interrupted operations found."
          : isRu
            ? `Проверено ${result.inspected}; откат выполнен для ${result.rolledBack}.`
            : `Inspected ${result.inspected}; restored ${result.rolledBack}.`,
      );
      const current = await engineBridge.currentTreePilot(installation.path);
      setOperationId(current?.operationId ?? null);
      setInstalledLanguage(current?.language ?? null);
      setInstalledPackageIds(current?.packageIds ?? []);
      setVerifiedInstall(Boolean(current));
      setPackageVerified(current?.packageVerified ?? false);
      setSteamOperationId(current?.steamOperationId ?? null);
      if (current?.steamProfileToken) setSelectedProfile(current.steamProfileToken);
      setSteamRecoveryRequired(current?.steamRecoveryRequired ?? false);
      setDotaPatched(current?.dotaPatched ?? false);
      saveEvidenceReport();
    } catch (cause) {
      setError(codeOf(cause));
    } finally {
      setEngineActive(false);
      setRecovering(false);
    }
  }

  async function refreshEvidence(copy = false) {
    if (!canManage) return;
    try {
      const report = await engineBridge.collectTreePilotEvidence(installation.path);
      setEvidence(report);
      if (copy) {
        await navigator.clipboard.writeText(JSON.stringify(report, null, 2));
        setReportCopied(true);
        window.setTimeout(() => setReportCopied(false), 2200);
      }
    } catch (cause) {
      setError(codeOf(cause));
    }
  }

  // Keeps a retained copy of the report on disk after state-changing
  // operations. Fire-and-forget: a failure is shown quietly in the report
  // section and never interrupts the operation that triggered it.
  function saveEvidenceReport() {
    if (!canManage) return;
    engineBridge
      .saveTreePilotEvidence(installation.path)
      .then((saved) => {
        setSavedReport(saved.fileName);
        setReportError("");
      })
      .catch((cause) => setReportError(codeOf(cause)));
  }

  async function openReportsFolder() {
    try {
      await engineBridge.openReportsFolder();
    } catch (cause) {
      setReportError(codeOf(cause));
    }
  }

  async function runStressTest(failurePoint: DeploymentStressFailurePoint) {
    if (
      !stressEnabled ||
      !plan ||
      !selectedLanguage ||
      !canInstall ||
      operationId ||
      phase !== "idle" ||
      stressPhase
    )
      return;
    setError("");
    setStressMessage("");
    setStressPhase(failurePoint);
    setEngineActive(true);
    try {
      const runtime = await engineBridge.prepareRuntimeForPatch();
      if (!runtime.patchReady) throw new Error("runtime_busy");
      try {
        await engineBridge.installTreePilotStress(
          installation.path,
          ids,
          plan.planId,
          selectedLanguage,
          failurePoint,
        );
        throw new Error("stress_injection_did_not_fire");
      } catch (cause) {
        if (codeOf(cause) !== "injected_failure") throw cause;
      }
      const recovery = await engineBridge.recoverGameDeployments(installation.path);
      const passed =
        failurePoint === "after_prepared"
          ? recovery.markedFailed > 0 && recovery.rolledBack === 0
          : recovery.rolledBack > 0;
      if (!passed) throw new Error("stress_recovery_unverified");
      await refreshEvidence(false);
      setStressMessage(
        failurePoint === "after_prepared"
          ? isRu
            ? "PASS · Обрыв до публикации не изменил файл игры."
            : "PASS · Interruption before publish left the game file unchanged."
          : isRu
            ? "PASS · Обрыв после публикации восстановлен из проверенного состояния."
            : "PASS · Interruption after publish was restored from verified state.",
      );
      saveEvidenceReport();
    } catch (cause) {
      setError(codeOf(cause));
    } finally {
      setEngineActive(false);
      setStressPhase(null);
    }
  }

  async function runSteamStressTest(failurePoint: DeploymentStressFailurePoint) {
    if (
      !stressEnabled ||
      !operationId ||
      !installedLanguage ||
      !selectedProfile ||
      !packageVerified ||
      steamOperationId ||
      !canInstall ||
      phase !== "idle" ||
      stressPhase
    )
      return;
    setError("");
    setStressMessage("");
    setStressPhase(failurePoint);
    setEngineActive(true);
    try {
      const profile = profiles.find((item) => item.profileToken === selectedProfile);
      if (!profile || !["ready", "already_managed"].includes(profile.status))
        throw new Error("steam_profile_conflict");
      const runtime = await engineBridge.prepareRuntimeForPatch();
      if (!runtime.patchReady) throw new Error("runtime_busy");
      const previewPlan = await engineBridge.previewSteamLaunchOptions(
        selectedProfile,
        installedLanguage,
      );
      if (!previewPlan.changed) throw new Error("stress_requires_unmanaged_profile");
      try {
        await engineBridge.applyTreeSteamLaunchOptionsStress(
          {
            gamePath: installation.path,
            deploymentOperationId: operationId,
            profileToken: selectedProfile,
            confirmationToken: previewPlan.confirmationToken,
            language: installedLanguage,
          },
          failurePoint,
        );
        throw new Error("stress_injection_did_not_fire");
      } catch (cause) {
        if (codeOf(cause) !== "injected_failure") throw cause;
      }
      const receipts = await engineBridge.recoverSteamLaunchOptions();
      if (
        !receipts.some(
          (receipt) => receipt.rolledBack && receipt.beforeSha256 === previewPlan.beforeSha256,
        )
      ) {
        throw new Error("stress_recovery_unverified");
      }
      await refreshEvidence(false);
      setStressMessage(
        failurePoint === "after_prepared"
          ? isRu
            ? "PASS · Обрыв подготовки Steam не изменил параметры запуска."
            : "PASS · Interrupted Steam preparation left launch options unchanged."
          : isRu
            ? "PASS · Изменение Steam после записи восстановлено byte-for-byte."
            : "PASS · Steam change after publish was restored byte-for-byte.",
      );
      saveEvidenceReport();
    } catch (cause) {
      setError(codeOf(cause));
    } finally {
      setEngineActive(false);
      setStressPhase(null);
    }
  }

  // Mounted even without pilot mods selected so an existing installation can
  // still be found and restored; renders nothing when there is none.
  if (!ids.length && !operationId && !steamOperationId && !steamRecoveryRequired) return null;
  return (
    <section className="s-tree-pilot" aria-labelledby="tree-pilot-title">
      <div className="s-tree-pilot-head">
        <span className="s-tree-pilot-symbol">
          <ShieldCheck />
        </span>
        <div>
          <small>
            {isRu ? "ПРОВЕРЯЕМАЯ СБОРКА" : "VERIFIABLE BUILD"} · {deliveryLabel("pilot", language)}
          </small>
          <h2 id="tree-pilot-title">{bundleName}</h2>
        </div>
        <span className="s-tree-pilot-mark">
          {String(ids.length).padStart(2, "0")} /{" "}
          {String(installablePackages.length).padStart(2, "0")}
        </span>
      </div>
      <p>
        {isRu
          ? "BetterFy скачивает только проверенные файлы модов, сверяет каждый и собирает из них один файл в выбранном тобой порядке. Если два мода меняют одно и то же, побеждает верхний."
          : "BetterFy downloads only verified mod files, checks each one and combines them into a single file in your chosen order. If two mods change the same thing, the higher one wins."}
      </p>
      <div className="s-tree-pilot-steps">
        <span className={plan ? "done" : ""}>
          01 <b>{isRu ? "Скачать и сверить ресурсы" : "Download and verify resources"}</b>
        </span>
        <span className={operationId ? "done" : ""}>
          02 <b>{isRu ? "Собрать и записать с откатом" : "Build and install with rollback"}</b>
        </span>
        <span className={steamRestarted ? "done" : ""}>
          03 <b>{isRu ? "Открыть Steam" : "Open Steam"}</b>
        </span>
        <span>
          04 <b>{isRu ? "Проверить в Dota 2" : "Check in Dota 2"}</b>
        </span>
      </div>
      {(!canManage || (ids.length > 0 && !supportedBundle)) && (
        <div className="s-inline-note warning">
          <TriangleAlert />
          <p>
            {!desktop
              ? isRu
                ? "Реальная подготовка доступна только в приложении BetterFy."
                : "Real preparation is only available in the BetterFy desktop app."
              : preview
                ? isRu
                  ? "В гостевом просмотре запись в игру недоступна."
                  : "Game installation is unavailable in guest preview."
                : !windows
                  ? isRu
                    ? "Установка доступна только на Windows. На Mac можно проверить интерфейс и подготовить ресурсы."
                    : "Installation is Windows-only. On Mac you can inspect the UI and prepare resources."
                  : !installation.verified
                    ? isRu
                      ? "Сначала подключи настоящую установку Dota 2 в настройках."
                      : "Connect a real Dota 2 installation in Settings first."
                    : relation
                      ? relation.kind === "conflict"
                        ? isRu
                          ? `«${packageLabel(relation.packageId)}» и «${packageLabel(relation.otherId)}» нельзя ставить вместе. Оставь в сборке один из них.`
                          : `${packageLabel(relation.packageId)} and ${packageLabel(relation.otherId)} cannot be installed together. Keep one of them.`
                        : isRu
                          ? `Для «${packageLabel(relation.packageId)}» нужен мод «${packageLabel(relation.otherId)}». Добавь его в сборку.`
                          : `${packageLabel(relation.packageId)} needs ${packageLabel(relation.otherId)}. Add it to the build.`
                      : isRu
                        ? "Устанавливаются только моды с отметкой «Windows-пилот». Убери остальные игровые моды из сборки."
                        : "Only mods marked Windows pilot install. Remove other game mods from the build."}
          </p>
        </div>
      )}
      {unverified.length > 0 && !operationId && (
        <div className="s-inline-note warning">
          <TriangleAlert />
          <p>
            {isRu
              ? `Ещё не проверено в игре: ${unverified.map(packageLabel).join(", ")}. Поставь, проверь в Dota и при проблеме откати.`
              : `Not verified in game yet: ${unverified.map(packageLabel).join(", ")}. Install, check in Dota and restore if something is wrong.`}
          </p>
        </div>
      )}
      {plan && !operationId && (
        <div className="s-tree-pilot-language">
          <div>
            <strong>
              {isRu ? "Выбери язык Dota для сборки" : "Choose Dota language for the build"}
            </strong>
            <p>
              {isRu
                ? "Моды подключаются через языковую папку Dota: игра будет запускаться с выбранным языком интерфейса. Нидерландский проверен на Windows для модов с отметкой о проверке; остальные языки ещё не проверены."
                : "Mods are loaded through a Dota language folder: the game will start with the chosen interface language. Dutch was verified on Windows for the mods marked as verified; the other languages have not been tested yet."}
            </p>
          </div>
          <select
            aria-label={isRu ? "Язык Dota для установки" : "Dota language for installation"}
            value={selectedLanguage}
            onChange={(event) => setSelectedLanguage(event.target.value as GameLanguage | "")}
          >
            <option value="">{isRu ? "Выбери язык" : "Choose language"}</option>
            <option value="russian">
              {isRu ? "Русский · проверка нужна" : "Russian · needs testing"}
            </option>
            <option value="koreana">
              {isRu ? "Корейский · проверка нужна" : "Korean · needs testing"}
            </option>
            <option value="schinese">
              {isRu ? "Китайский · проверка нужна" : "Chinese · needs testing"}
            </option>
            <option value="dutch">{isRu ? "Нидерландский · проверен" : "Dutch · verified"}</option>
          </select>
        </div>
      )}
      {plan && !operationId && (
        <>
          <div
            className="s-tree-pilot-order"
            aria-label={isRu ? "Порядок приоритета модов" : "Mod priority order"}
          >
            <div>
              <strong>{isRu ? "Порядок модов" : "Mod order"}</strong>
              <p>
                {isRu
                  ? "Если два мода меняют один ресурс, верхний остаётся в итоговой сборке."
                  : "If two mods change the same resource, the higher one remains in the final build."}
              </p>
            </div>
            <ol>
              {plan.contributions.map((item, index) => (
                <li
                  key={item.packageId}
                  title={
                    isRu
                      ? `${item.effectiveResources} из ${item.inputResources} файлов мода войдут в сборку`
                      : `${item.effectiveResources} of ${item.inputResources} mod files remain in the build`
                  }
                >
                  <span>{String(index + 1).padStart(2, "0")}</span>
                  <b>{packageLabel(item.packageId)}</b>
                  <em className={item.shadowedResources > 0 ? "has-overrides" : ""}>
                    {item.effectiveResources}/{item.inputResources}
                  </em>
                </li>
              ))}
            </ol>
          </div>
          <div className="s-tree-pilot-plan">
            <span>
              <b>{isRu ? "Куда будет записан файл" : "File destination"}</b>
              <code>
                {selectedLanguage ? `game/dota_${selectedLanguage}/${plan.targetFile}` : "—"}
              </code>
            </span>
            <span>
              <b>{isRu ? "Состав" : "Bundle"}</b>
              {plan.packageCount} · {plan.resourceCount} {isRu ? "ресурсов" : "resources"}
            </span>
            <span>
              <b>{isRu ? "Совпадения" : "Overlaps"}</b>
              {plan.duplicateResources + plan.overriddenResources || (isRu ? "Нет" : "None")}
            </span>
            <span>
              <b>{isRu ? "Размер файла" : "File size"}</b>
              {(plan.vpkBytes / 1024).toFixed(1)} KB
            </span>
            <span>
              <b>SHA-256</b>
              <code>{plan.vpkSha256.slice(0, 12)}…</code>
            </span>
          </div>
          {(plan.duplicates.length > 0 || plan.overrides.length > 0) && (
            <details className="s-tree-pilot-conflicts">
              <summary>{isRu ? "Показать решение совпадений" : "Show overlap resolution"}</summary>
              {plan.overrides.map((item) => (
                <p key={`override-${item.path}`}>
                  <code>{item.path}</code>
                  <span>
                    {packageLabel(item.winnerPackageId)} →{" "}
                    {isRu ? "оставлен; ниже пропущены" : "kept; lower skipped"}:{" "}
                    {item.shadowedPackageIds.map(packageLabel).join(", ")}
                  </span>
                </p>
              ))}
              {plan.duplicates.map((item) => (
                <p key={`duplicate-${item.path}`}>
                  <code>{item.path}</code>
                  <span>
                    {isRu
                      ? "Одинаковый ресурс сохранён один раз"
                      : "Identical resource stored once"}
                    : {packageLabel(item.keptPackageId)}
                  </span>
                </p>
              ))}
            </details>
          )}
        </>
      )}
      {plan && !operationId && canInstall && (
        <div className="s-tree-pilot-steam">
          <div>
            <strong>
              {isRu ? "Steam-профиль для установки" : "Steam profile for installation"}
            </strong>
            <p>
              {isRu
                ? "BetterFy мягко закроет Dota 2 и Steam, запишет файл, добавит выбранный -language в этот профиль и снова откроет Steam. Если профиль не выбран, параметр нужно будет вставить вручную."
                : "BetterFy will gracefully close Dota 2 and Steam, write the file, add the selected -language option to this profile, and reopen Steam. Without a profile, paste the option manually."}
            </p>
          </div>
          <select
            aria-label={isRu ? "Steam-профиль" : "Steam profile"}
            value={selectedProfile}
            onChange={(event) => setSelectedProfile(event.target.value)}
          >
            <option value="">{isRu ? "Вручную" : "Manual setup"}</option>
            {profiles.map((profile) => (
              <option
                key={profile.profileToken}
                value={profile.profileToken}
                disabled={profile.status !== "ready" && profile.status !== "already_managed"}
              >
                {isRu ? "Профиль" : "Profile"} {profile.profileIndex} ·{" "}
                {profile.status === "ready"
                  ? isRu
                    ? "готов"
                    : "ready"
                  : profile.status === "already_managed"
                    ? isRu
                      ? "язык задан"
                      : "language set"
                    : isRu
                      ? "конфликт"
                      : "conflict"}
              </option>
            ))}
          </select>
        </div>
      )}
      {operationId && dotaPatched && (
        <div className="s-inline-note warning" role="alert">
          <TriangleAlert />
          <p>
            {isRu
              ? "Dota 2 обновилась после установки этой сборки. Проверь игру: если интерфейс выглядит сломанным, откати сборку — BetterFy вернёт исходные файлы."
              : "Dota 2 was updated after this build was installed. Check the game: if the interface looks broken, restore the build and BetterFy will put the original files back."}
          </p>
        </div>
      )}
      {operationId ? (
        <div className="s-tree-pilot-result">
          {verifiedInstall && packageVerified ? <Check /> : <TriangleAlert />}
          <span>
            {verifiedInstall
              ? packageVerified
                ? isRu
                  ? `BetterFy записал сборку из ${installedPackageIds.length || 1} модов. Проверь игру; при проблеме верни исходное состояние.`
                  : `BetterFy installed a ${installedPackageIds.length || 1}-mod build. Check the game; restore the previous state if needed.`
                : isRu
                  ? "Файл и журнал BetterFy совпадают, но исходные ресурсы недоступны для повторной проверки. Откат остаётся доступен; для новой установки подготовь ресурсы заново."
                  : "The installed file matches BetterFy's journal, but source resources are unavailable for another check. Restore remains available; prepare resources again before reinstalling."
              : isRu
                ? "Найдена сохранённая операция, но состояние файла ещё не подтверждено. Проверь установку или выполни откат."
                : "A saved operation was found, but the file state is not verified yet. Check or restore the installation."}
          </span>
          <button
            className="s-btn"
            disabled={phase !== "idle" || !canManage || steamRecoveryRequired}
            onClick={() => void restore()}
          >
            {phase === "restore" ? <LoaderCircle className="s-spin" /> : <RotateCcw />}
            {isRu ? "Откатить" : "Restore"}
          </button>
        </div>
      ) : (
        <div className="s-tree-pilot-actions">
          {!plan ? (
            <>
              <button
                className="s-btn s-btn-primary"
                disabled={phase !== "idle" || !desktop || !supportedBundle}
                onClick={() => void prepare()}
              >
                {phase === "download" ? <LoaderCircle className="s-spin" /> : <Download />}
                {phase === "download"
                  ? isRu
                    ? "Проверяем ресурсы…"
                    : "Verifying resources…"
                  : isRu
                    ? "Подготовить сборку"
                    : "Prepare build"}
              </button>
              {phase === "download" && (
                <button className="s-btn" onClick={() => void cancelPrepare()}>
                  {isRu ? "Отменить" : "Cancel"}
                </button>
              )}
            </>
          ) : (
            <>
              <span>
                <Check />
                {isRu ? "Файлы модов и сборка проверены" : "Mod files and build verified"}
              </span>
              <button
                className="s-btn s-btn-primary"
                disabled={
                  !canInstall || !selectedLanguage || phase !== "idle" || !plan.deployEnabled
                }
                onClick={() => void install()}
              >
                {phase === "install" ? <LoaderCircle className="s-spin" /> : <ShieldCheck />}
                {phase === "install"
                  ? isRu
                    ? "Устанавливаем…"
                    : "Installing…"
                  : isRu
                    ? "Установить сборку"
                    : "Install build"}
                <ArrowRight />
              </button>
            </>
          )}
        </div>
      )}
      {phase === "download" && (
        <div
          className="s-tree-pilot-progress"
          role="progressbar"
          aria-valuenow={verifiedResources}
          aria-valuemin={0}
          aria-valuemax={totalResources || 1}
          aria-label={isRu ? "Проверенные ресурсы" : "Verified resources"}
        >
          <span>
            {isRu ? "Проверено ресурсов" : "Resources verified"} · {verifiedResources}/
            {totalResources || "—"}
          </span>
          <div>
            <i
              style={{
                width: `${totalResources ? (verifiedResources / totalResources) * 100 : 0}%`,
              }}
            />
          </div>
        </div>
      )}
      {operationId && installedLanguage && (
        <div className="s-tree-pilot-steam">
          <div>
            <strong>{isRu ? "Где находится сборка" : "Where the build is installed"}</strong>
            <p>
              <code>game/dota_{installedLanguage}/pak66_dir.vpk</code>
            </p>
            {installedPackageIds.length > 0 && (
              <p>{installedPackageIds.map(packageLabel).join(" · ")}</p>
            )}
            <p>
              {!packageVerified
                ? isRu
                  ? "Исходные ресурсы сейчас не подтверждены. Настройка Steam заблокирована; можно откатить установленный файл."
                  : "Source resources are not currently verified. Steam setup is blocked; the installed file can be restored."
                : steamStarted
                  ? isRu
                    ? "Выбранный Steam-профиль использует нужный язык; Steam открыт. Запусти Dota 2 и проверь каждый мод сборки."
                    : "The selected Steam profile uses the required language; Steam is open. Launch Dota 2 and check every mod in the build."
                  : steamRestarted
                    ? isRu
                      ? "Steam открыт. Добавь параметр ниже в свойствах Dota 2 → параметры запуска, затем запусти игру."
                      : "Steam is open. Add the option below in Dota 2 properties → launch options, then launch the game."
                    : isRu
                      ? "Файл записан, но запуск Steam или его настройка не завершились. Проверь профиль и повтори настройку."
                      : "The file was written, but Steam startup or setup did not finish. Check the profile and retry setup."}
            </p>
            <p>
              {isRu
                ? "Чтобы выбрать другой язык, сначала откати эту установку."
                : "Restore this installation before choosing another language."}
            </p>
          </div>
          <div className="s-tree-pilot-command">
            <code>-language {installedLanguage}</code>
            <button className="s-btn" type="button" onClick={() => void copyLaunchOption()}>
              <Copy />
              {copied ? (isRu ? "Скопировано" : "Copied") : isRu ? "Копировать" : "Copy"}
            </button>
          </div>
          {canManage && !steamStarted && packageVerified && (
            <>
              <select
                aria-label={isRu ? "Steam-профиль" : "Steam profile"}
                value={selectedProfile}
                onChange={(event) => setSelectedProfile(event.target.value)}
              >
                <option value="">{isRu ? "Вручную" : "Manual setup"}</option>
                {profiles.map((profile) => (
                  <option
                    key={profile.profileToken}
                    value={profile.profileToken}
                    disabled={profile.status !== "ready" && profile.status !== "already_managed"}
                  >
                    {isRu ? "Профиль" : "Profile"} {profile.profileIndex} ·{" "}
                    {profile.status === "ready"
                      ? isRu
                        ? "готов"
                        : "ready"
                      : profile.status === "already_managed"
                        ? isRu
                          ? "язык задан"
                          : "language set"
                        : isRu
                          ? "конфликт"
                          : "conflict"}
                  </option>
                ))}
              </select>
              <button
                className="s-btn"
                disabled={phase !== "idle" || !selectedProfile}
                onClick={() => void activateSteam()}
              >
                {phase === "steam" ? <LoaderCircle className="s-spin" /> : <ArrowRight />}
                {isRu ? "Настроить Steam" : "Set up Steam"}
              </button>
            </>
          )}
        </div>
      )}
      {!operationId && steamOperationId && canManage && (
        <button className="s-btn" disabled={phase !== "idle"} onClick={() => void restoreSteam()}>
          <RotateCcw />
          {isRu ? "Восстановить параметры Steam" : "Restore Steam settings"}
        </button>
      )}
      {canManage && (
        <div className="s-tree-pilot-recovery">
          {steamRecoveryRequired && (
            <span role="alert">
              {isRu
                ? "Изменение Steam было прервано. Сначала восстанови Steam, затем откатывай файл игры."
                : "A Steam change was interrupted. Recover Steam before restoring the game file."}
            </span>
          )}
          <button
            className="s-text-button"
            disabled={phase !== "idle" || recovering}
            onClick={() => void recover()}
          >
            {recovering ? <LoaderCircle className="s-spin" /> : <RotateCcw />}
            {steamRecoveryRequired
              ? isRu
                ? "Восстановить Steam и проверить установку"
                : "Recover Steam and check installation"
              : isRu
                ? "Проверить прерванную установку"
                : "Check interrupted installation"}
          </button>
          {recoveryMessage && <span role="status">{recoveryMessage}</span>}
        </div>
      )}
      {canManage && (
        <details className="s-tree-pilot-evidence">
          <summary>
            <FileCheck2 />
            {isRu ? "Отчёт Windows-теста" : "Windows test report"}
            <span>{(evidence?.entries.length ?? 0) + (evidence?.steamEntries.length ?? 0)}</span>
          </summary>
          <div>
            <p>
              {isRu
                ? "Отчёт содержит только версии, этапы, языки, пакеты, SHA-256 и результат точного отката Steam. Путей, Steam ID и данных аккаунта в нём нет."
                : "The report contains only versions, phases, languages, packages, SHA-256 values, and exact Steam rollback results. It contains no paths, Steam IDs, or account data."}
            </p>
            <button className="s-btn" type="button" onClick={() => void refreshEvidence(true)}>
              <Copy />
              {reportCopied
                ? isRu
                  ? "Скопировано"
                  : "Copied"
                : isRu
                  ? "Скопировать отчёт"
                  : "Copy report"}
            </button>
          </div>
          <div>
            <p role="status">
              {savedReport
                ? isRu
                  ? `Отчёт сохранён: ${savedReport}`
                  : `Report saved: ${savedReport}`
                : isRu
                  ? "Отчёт сохраняется автоматически после установки, отката, проверки и стресс-теста."
                  : "The report is saved automatically after install, restore, recovery, and stress tests."}
              {reportError &&
                (isRu
                  ? ` Не удалось сохранить или открыть отчёт (${reportError}).`
                  : ` Could not save or open the report (${reportError}).`)}
            </p>
            <button className="s-btn" type="button" onClick={() => void openReportsFolder()}>
              {isRu ? "Открыть папку отчётов" : "Open reports folder"}
            </button>
          </div>
          {stressEnabled && plan && !operationId && (
            <section>
              <strong>
                <FlaskConical />
                {isRu ? "Контролируемое восстановление" : "Controlled recovery"}
              </strong>
              <p>
                {isRu
                  ? "Только для внутренней тестовой сборки. BetterFy намеренно останавливает транзакцию в безопасной точке и сразу запускает восстановление."
                  : "Internal test build only. BetterFy intentionally stops the transaction at a safe boundary and immediately runs recovery."}
              </p>
              <div>
                <button
                  className="s-btn"
                  disabled={!selectedLanguage || Boolean(stressPhase)}
                  onClick={() => void runStressTest("after_prepared")}
                >
                  {stressPhase === "after_prepared" && <LoaderCircle className="s-spin" />}
                  {isRu ? "Обрыв до записи" : "Interrupt before publish"}
                </button>
                <button
                  className="s-btn"
                  disabled={!selectedLanguage || Boolean(stressPhase)}
                  onClick={() => void runStressTest("after_replace")}
                >
                  {stressPhase === "after_replace" && <LoaderCircle className="s-spin" />}
                  {isRu ? "Обрыв после записи" : "Interrupt after publish"}
                </button>
              </div>
            </section>
          )}
          {stressEnabled &&
            operationId &&
            installedLanguage &&
            selectedProfile &&
            packageVerified &&
            !steamOperationId && (
              <section>
                <strong>
                  <FlaskConical />
                  {isRu ? "Восстановление Steam" : "Steam recovery"}
                </strong>
                <p>
                  {isRu
                    ? "Проверяет обе стороны атомарной записи localconfig.vdf. Текущий профиль должен ещё не содержать выбранный параметр языка."
                    : "Tests both sides of the atomic localconfig.vdf write. The current profile must not already contain the selected language option."}
                </p>
                <div>
                  <button
                    className="s-btn"
                    disabled={Boolean(stressPhase)}
                    onClick={() => void runSteamStressTest("after_prepared")}
                  >
                    {stressPhase === "after_prepared" && <LoaderCircle className="s-spin" />}
                    {isRu ? "Steam · до записи" : "Steam · before publish"}
                  </button>
                  <button
                    className="s-btn"
                    disabled={Boolean(stressPhase)}
                    onClick={() => void runSteamStressTest("after_replace")}
                  >
                    {stressPhase === "after_replace" && <LoaderCircle className="s-spin" />}
                    {isRu ? "Steam · после записи" : "Steam · after publish"}
                  </button>
                </div>
              </section>
            )}
          {stressMessage && (
            <p className="s-tree-pilot-stress-pass" role="status">
              {stressMessage}
            </p>
          )}
        </details>
      )}
      {error && (
        <p className="s-tree-pilot-error" role="alert">
          {explainError(error, isRu)}
        </p>
      )}
      <small className="s-tree-pilot-foot">
        {isRu
          ? "Источник: Egezenn/dota2-minify. Tree Mod, Show NetWorth и Unit Query HUD проверены в игре на одном Windows-компьютере с нидерландским языком; другие языки и компьютеры не проверены. Параметр -language меняет язык текста и может повлиять на озвучку. BetterFy мягко закрывает Dota 2 и Steam и после установки или отката снова запускает Steam."
          : "Source: Egezenn/dota2-minify. Tree Mod, Show NetWorth and Unit Query HUD were verified in game on one Windows computer with Dutch; other languages and computers are untested. The -language option changes text language and may affect audio. BetterFy closes Dota 2 and Steam gracefully and starts Steam again after install or restore."}
      </small>
    </section>
  );
}
