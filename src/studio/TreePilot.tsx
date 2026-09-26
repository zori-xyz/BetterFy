import { useEffect, useState } from "react";
import { ArrowRight, Check, Copy, Download, LoaderCircle, RotateCcw, ShieldCheck, TriangleAlert } from "lucide-react";
import { invoke } from "@tauri-apps/api/core";
import { engineBridge, type GameDeploymentReceipt, type GameInstallation, type GameLanguage, type SteamConfigReceipt, type SteamProfileSummary } from "../engine";
import { useLocale } from "../i18n";

type TreePlan = {
  planId: string;
  packageId: string;
  packageIds: string[];
  sourceCommit: string;
  targetFile: string;
  resourceCount: number;
  resourceBytes: number;
  vpkBytes: number;
  vpkSha256: string;
  bundlePlanId: string;
  packageCount: number;
  duplicateResources: number;
  overriddenResources: number;
  duplicates: Array<{ path: string; keptPackageId: string; duplicatePackageIds: string[] }>;
  overrides: Array<{ path: string; winnerPackageId: string; shadowedPackageIds: string[] }>;
  contributions: Array<{ packageId: string; inputResources: number; effectiveResources: number; duplicateResources: number; shadowedResources: number }>;
  compatibility: "unknown";
  distribution: "internal_pilot";
  deployEnabled: boolean;
};
type TreeDownloadStatus = { phase: "downloading" | "verifying" | "ready" | "failed" | "cancelled"; verifiedResources: number; totalResources: number; errorCode: string | null; plan: TreePlan | null };
type TreeCurrentState = GameDeploymentReceipt & { packageVerified: boolean; steamOperationId: string | null; steamProfileToken: string | null; steamRecoveryRequired: boolean };

const codeOf = (error: unknown) => error instanceof Error ? error.message : String(error);
const explainError = (code: string, isRu: boolean) => {
  const messages: Record<string, [string, string]> = {
    runtime_busy: ["Закрой Dota 2 и Steam, затем повтори.", "Close Dota 2 and Steam, then retry."],
    dota_close_unavailable: ["Не удалось мягко закрыть Dota 2. Закрой игру вручную и повтори.", "Dota 2 could not be closed gracefully. Close it manually and retry."],
    shutdown_timeout: ["Dota 2 или Steam не завершились за 30 секунд. Закрой их вручную и повтори.", "Dota 2 or Steam did not close within 30 seconds. Close them manually and retry."],
    steam_start_failed: ["Файл установлен, но Steam не запустился. Запусти Steam вручную и проверь параметры запуска.", "The file was installed, but Steam did not start. Open Steam manually and check launch options."],
    steam_start_timeout: ["Файл установлен, но BetterFy не дождался запуска Steam. Проверь Steam вручную.", "The file was installed, but BetterFy did not detect Steam starting. Check Steam manually."],
    platform_not_supported: ["Установка доступна только на Windows.", "Installation is Windows-only."],
    download_dns_failed: ["Нет доступа к источнику. Проверь подключение к сети.", "The source is unreachable. Check your connection."],
    download_transport_failed: ["Не удалось скачать ресурс. Повтори подготовку — проверенные файлы сохранятся.", "A resource could not be downloaded. Retry; verified files are kept."],
    build_plan_stale: ["План устарел. Подготовь Tree Mod заново.", "The plan is stale. Prepare Tree Mod again."],
    deployment_target_foreign: ["Целевой файл занят другой модификацией. BetterFy не будет его заменять.", "Another modification owns the target. BetterFy will not overwrite it."],
    deployment_language_change_requires_restore: ["Сначала откати установленный Tree Mod, затем выбери другой язык.", "Restore the installed Tree Mod before choosing another language."],
    language_folder_unavailable: ["В установке Dota нет папки выбранного языка с gameinfo.gi. Выбери другой язык или восстанови файлы игры через Steam.", "This Dota installation lacks the selected language folder with gameinfo.gi. Choose another language or verify game files in Steam."],
    launch_option_conflict: ["В Steam уже указан другой язык. Восстанови прежние параметры BetterFy или измени их вручную перед установкой.", "Steam already specifies a different language. Restore BetterFy's previous settings or change them manually before installing."],
    deployment_conflict: ["Файл игры изменился после установки. Автоматический откат остановлен.", "The game file changed after installation. Automatic restore was stopped."],
    backup_failed: ["Резервная копия BetterFy недоступна. Автоматический откат заблокирован; не заменяй файл вручную, пока не проверишь состояние установки.", "BetterFy's backup is unavailable. Automatic restore is blocked; inspect the installation before changing the file manually."],
    backup_verification_failed: ["Резервная копия не прошла проверку. BetterFy не будет восстанавливать её поверх файла игры.", "The backup failed verification. BetterFy will not restore it over the game file."],
    rollback_conflict: ["Файл игры изменился после установки. Автоматический откат остановлен.", "The game file changed after installation. Automatic restore was stopped."],
    steam_profile_conflict: ["Этот Steam-профиль нельзя менять автоматически. Выбери другой или проверь параметры запуска вручную.", "This Steam profile cannot be changed automatically. Choose another or inspect its launch options."],
    steam_activation_not_ready: ["Параметры Steam не подтверждены. Не считай мод активным в игре.", "Steam settings were not verified. Do not assume the mod is active in game."],
    steam_recovery_required: ["Найдено прерванное изменение Steam. Сначала восстанови его в диагностике.", "An interrupted Steam change was found. Recover it in diagnostics first."],
    clipboard_unavailable: ["Не удалось скопировать параметр. Выдели и скопируй его вручную.", "Could not copy the option. Select and copy it manually."],
  };
  return messages[code]?.[isRu ? 0 : 1] ?? (isRu ? `Операция остановлена (${code}). Проверь состояние установки перед запуском игры.` : `Operation stopped (${code}). Check installation state before launching the game.`);
};

export default function TreePilot({ ids, installation, preview }: { ids: string[]; installation: GameInstallation; preview: boolean }) {
  const { isRu } = useLocale();
  const [plan, setPlan] = useState<TreePlan | null>(null);
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
  const [error, setError] = useState("");
  const supportedIds = ["minify-tree-mod", "minify-show-networth", "minify-repopulate-unit-query-hud"];
  const packageLabel = (id: string) => id.includes("tree-mod") ? "Tree Mod" : id.includes("show-networth") ? "Show Net Worth" : id.includes("unit-query-hud") ? "Unit Query HUD" : id;
  const supportedBundle = ids.length > 0 && ids.length <= 3 && ids.every(id => supportedIds.includes(id)) && new Set(ids).size === ids.length;
  const bundleName = ids.length > 1 ? (isRu ? `Сборка · ${ids.length} мода` : `Build · ${ids.length} mods`) : ids[0] === "minify-show-networth" ? "Show Net Worth" : ids[0] === "minify-repopulate-unit-query-hud" ? "Unit Query HUD" : "Tree Mod";
  const windows = /Windows/i.test(navigator.userAgent);
  const desktop = "__TAURI_INTERNALS__" in window;
  const canInstall = desktop && windows && !preview && installation.verified && supportedBundle;
  function applyDownloadStatus(status: TreeDownloadStatus | null) {
    if (!status) return;
    setVerifiedResources(status.verifiedResources);
    setTotalResources(status.totalResources);
    if (status.phase === "ready") {
      const expectedIds = ids.map(id => id === "minify-tree-mod" ? "minify.tree-mod" : id === "minify-show-networth" ? "minify.show-networth" : id === "minify-repopulate-unit-query-hud" ? "minify.repopulate-unit-query-hud" : id);
      if (status.plan?.planId.startsWith("sha256:") && status.plan.packageIds.join("|") === expectedIds.join("|")) {
        setPlan(status.plan);
        setError("");
      } else {
        setPlan(null);
        setError("");
      }
      setPhase("idle");
    } else if (status.phase === "failed" || status.phase === "cancelled") {
      setError(status.phase === "cancelled" ? "" : status.errorCode ?? "download_failed");
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
    invoke<TreeDownloadStatus | null>("tree_pilot_download_status").then(status => {
      if (active) applyDownloadStatus(status);
    }).catch(() => undefined);
    return () => { active = false; };
  }, [desktop, ids.join("|"), operationId]);
  useEffect(() => {
    if (phase !== "download") return;
    let active = true;
    const timer = window.setInterval(() => {
      invoke<TreeDownloadStatus | null>("tree_pilot_download_status").then(status => {
        if (active) applyDownloadStatus(status);
      }).catch(cause => { if (active) { setError(codeOf(cause)); setPhase("idle"); } });
    }, 450);
    return () => { active = false; window.clearInterval(timer); };
  }, [phase]);
  useEffect(() => {
    if (!desktop || !windows || preview || !installation.verified) return;
    let active = true;
    invoke<TreeCurrentState | null>("current_tree_pilot", { gamePath: installation.path })
      .then(receipt => {
        if (!active) return;
        setOperationId(receipt?.operationId ?? null);
        setInstalledLanguage(receipt?.language ?? null);
        setInstalledPackageIds(receipt?.packageIds ?? []);
        setVerifiedInstall(Boolean(receipt));
        setPackageVerified(receipt?.packageVerified ?? false);
        setSteamOperationId(receipt?.steamOperationId ?? null);
        if (receipt?.steamProfileToken) setSelectedProfile(receipt.steamProfileToken);
        setSteamRecoveryRequired(receipt?.steamRecoveryRequired ?? false);
      })
      .catch(cause => { if (active) setError(codeOf(cause)); });
    return () => { active = false; };
  }, [desktop, windows, preview, installation.verified, installation.path]);
  useEffect(() => {
    if (!canInstall) return;
    let active = true;
    engineBridge.listSteamProfiles().then(found => {
      if (!active) return;
      setProfiles(found);
      setSelectedProfile(previous => found.some(profile => profile.profileToken === previous)
        ? previous
        : found.find(profile => profile.status === "ready")?.profileToken
          ?? found.find(profile => profile.status === "already_managed")?.profileToken
          ?? "");
    }).catch(cause => { if (active) setError(codeOf(cause)); });
    return () => { active = false; };
  }, [canInstall]);

  async function prepare() {
    if (!desktop || phase !== "idle") return;
    setError("");
    setPhase("download");
    try {
      applyDownloadStatus(await invoke<TreeDownloadStatus>("begin_tree_pilot_download", { packageIds: ids }));
    } catch (cause) { setError(codeOf(cause)); setPhase("idle"); }
  }

  async function cancelPrepare() {
    try { await invoke<TreeDownloadStatus>("cancel_tree_pilot_download"); }
    catch (cause) { setError(codeOf(cause)); }
  }

  async function install() {
    if (!plan || !canInstall || !selectedLanguage || phase !== "idle") return;
    setError("");
    setPhase("install");
    try {
      await invoke("preview_tree_language", { gamePath: installation.path, language: selectedLanguage });
      if (selectedProfile) {
        const profile = profiles.find(item => item.profileToken === selectedProfile);
        if (!profile || !["ready", "already_managed"].includes(profile.status)) throw new Error("steam_profile_conflict");
        await engineBridge.previewSteamLaunchOptions(selectedProfile, selectedLanguage);
      }
      const runtime = await engineBridge.prepareRuntimeForPatch();
      if (!runtime.patchReady) throw new Error("runtime_busy");
      const receipt = await invoke<GameDeploymentReceipt>("install_tree_pilot", {
        request: { gamePath: installation.path, packageIds: ids, expectedPlanId: plan.planId, language: selectedLanguage, confirmed: true },
      });
      if (!receipt.committed || !receipt.backupVerified) throw new Error("deployment_unverified");
      setOperationId(receipt.operationId);
      setInstalledLanguage(receipt.language);
      setInstalledPackageIds(receipt.packageIds);
      setVerifiedInstall(true);
      setPackageVerified(true);
      setPhase("steam");
      if (selectedProfile) {
        const profile = profiles.find(item => item.profileToken === selectedProfile);
        if (!profile || !["ready", "already_managed"].includes(profile.status)) throw new Error("steam_profile_conflict");
        const steamPlan = await engineBridge.previewSteamLaunchOptions(selectedProfile, receipt.language);
        const steamReceipt = await invoke<SteamConfigReceipt>("apply_tree_steam_launch_options", { request: {
          gamePath: installation.path,
          deploymentOperationId: receipt.operationId,
          profileToken: selectedProfile,
          confirmationToken: steamPlan.confirmationToken,
          language: receipt.language,
          confirmed: true,
        } });
        if (!steamReceipt.committed || (steamReceipt.changed && !steamReceipt.backupVerified)) throw new Error("steam_activation_not_ready");
        if (steamReceipt.operationId) {
          setSteamOperationId(steamReceipt.operationId);
        }
        await invoke("start_steam_after_tree_pilot", { request: {
          gamePath: installation.path,
          operationId: receipt.operationId,
          profileToken: selectedProfile,
          steamOperationId: steamReceipt.operationId,
          confirmed: true,
        } });
        setSteamStarted(true);
        setSteamRestarted(true);
      } else {
        await invoke("start_steam_after_tree_pilot", { request: {
          gamePath: installation.path,
          operationId: receipt.operationId,
          confirmed: true,
        } });
        setSteamRestarted(true);
      }
    } catch (cause) { setError(codeOf(cause)); }
    finally { setPhase("idle"); }
  }

  async function restore() {
    if (!operationId || !canInstall || phase !== "idle") return;
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
      setSteamRestarted(false);
      setSteamRecoveryRequired(false);
      if (!steamOperationId) {
        setRecoveryMessage(isRu
          ? "Файл восстановлен. Если язык задавался в Steam раньше или вручную, проверь параметры запуска Dota 2 отдельно."
          : "The game file was restored. If a language option was set earlier or manually, check Dota 2 launch options separately.");
      }
    } catch (cause) { setError(codeOf(cause)); }
    finally { setPhase("idle"); }
  }

  async function activateSteam() {
    if (!operationId || !installedLanguage || !canInstall || !selectedProfile || !packageVerified || steamRecoveryRequired || phase !== "idle") return;
    setError("");
    setPhase("steam");
    try {
      const profile = profiles.find(item => item.profileToken === selectedProfile);
      if (!profile || !["ready", "already_managed"].includes(profile.status)) throw new Error("steam_profile_conflict");
      const runtime = await engineBridge.prepareRuntimeForPatch();
      if (!runtime.patchReady) throw new Error("runtime_busy");
      if (steamOperationId) {
        await invoke("start_steam_after_tree_pilot", { request: {
          gamePath: installation.path,
          operationId,
          profileToken: selectedProfile,
          steamOperationId,
          confirmed: true,
        } });
        setSteamStarted(true);
        setSteamRestarted(true);
        return;
      }
      const preview = await engineBridge.previewSteamLaunchOptions(selectedProfile, installedLanguage);
      const receipt = await invoke<SteamConfigReceipt>("apply_tree_steam_launch_options", { request: {
        gamePath: installation.path,
        deploymentOperationId: operationId,
        profileToken: selectedProfile,
        confirmationToken: preview.confirmationToken,
        language: installedLanguage,
        confirmed: true,
      } });
      if (!receipt.committed || (receipt.changed && !receipt.backupVerified)) throw new Error("steam_activation_not_ready");
      if (receipt.operationId) {
        setSteamOperationId(receipt.operationId);
      }
      await invoke("start_steam_after_tree_pilot", { request: {
        gamePath: installation.path,
        operationId,
        profileToken: selectedProfile,
        steamOperationId: receipt.operationId,
        confirmed: true,
      } });
      setSteamStarted(true);
      setSteamRestarted(true);
    } catch (cause) { setError(codeOf(cause)); }
    finally { setPhase("idle"); }
  }

  async function restoreSteam() {
    if (!steamOperationId || !canInstall || phase !== "idle") return;
    setPhase("restore");
    setError("");
    try {
      const runtime = await engineBridge.prepareRuntimeForPatch();
      if (!runtime.patchReady) throw new Error("runtime_busy");
      const receipt = await engineBridge.rollbackSteamLaunchOptions(steamOperationId);
      if (!receipt.rolledBack) throw new Error("steam_rollback_failed");
      setSteamOperationId(null);
      setSteamStarted(false);
    } catch (cause) { setError(codeOf(cause)); }
    finally { setPhase("idle"); }
  }

  async function copyLaunchOption() {
    try {
      if (!installedLanguage) return;
      await navigator.clipboard.writeText(`-language ${installedLanguage}`);
      setCopied(true);
      window.setTimeout(() => setCopied(false), 2200);
    } catch { setError("clipboard_unavailable"); }
  }

  async function recover() {
    if (!canInstall || phase !== "idle" || recovering) return;
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
      setRecoveryMessage(result.inspected === 0
        ? (isRu ? "Незавершённых операций не найдено." : "No interrupted operations found.")
        : (isRu ? `Проверено ${result.inspected}; откат выполнен для ${result.rolledBack}.` : `Inspected ${result.inspected}; restored ${result.rolledBack}.`));
      const current = await invoke<TreeCurrentState | null>("current_tree_pilot", { gamePath: installation.path });
      setOperationId(current?.operationId ?? null);
      setInstalledLanguage(current?.language ?? null);
      setInstalledPackageIds(current?.packageIds ?? []);
      setVerifiedInstall(Boolean(current));
      setPackageVerified(current?.packageVerified ?? false);
      setSteamOperationId(current?.steamOperationId ?? null);
      if (current?.steamProfileToken) setSelectedProfile(current.steamProfileToken);
      setSteamRecoveryRequired(current?.steamRecoveryRequired ?? false);
    } catch (cause) { setError(codeOf(cause)); }
    finally { setRecovering(false); }
  }

  return <section className="s-tree-pilot" aria-labelledby="tree-pilot-title">
    <div className="s-tree-pilot-head"><span className="s-tree-pilot-symbol"><ShieldCheck /></span><div><small>{isRu ? "ПРОВЕРЯЕМАЯ СБОРКА · WINDOWS-ПИЛОТ" : "VERIFIABLE BUILD · WINDOWS PILOT"}</small><h2 id="tree-pilot-title">{bundleName}</h2></div><span className="s-tree-pilot-mark">{String(ids.length).padStart(2, "0")} / 03</span></div>
    <p>{isRu ? "BetterFy скачивает только закреплённые ресурсы, сверяет каждый файл и собирает один VPK в выбранном тобой порядке. Первый мод имеет приоритет при совпадении ресурсов." : "BetterFy downloads only pinned resources, verifies every file, and assembles one VPK in your chosen order. The first mod wins when resources overlap."}</p>
    <div className="s-tree-pilot-steps"><span className={plan ? "done" : ""}>01 <b>{isRu ? "Скачать и сверить ресурсы" : "Download and verify resources"}</b></span><span className={operationId ? "done" : ""}>02 <b>{isRu ? "Собрать и записать с откатом" : "Build and install with rollback"}</b></span><span className={steamRestarted ? "done" : ""}>03 <b>{isRu ? "Открыть Steam" : "Open Steam"}</b></span><span>04 <b>{isRu ? "Проверить в Dota 2" : "Check in Dota 2"}</b></span></div>
    {!canInstall && <div className="s-inline-note warning"><TriangleAlert /><p>{!desktop ? (isRu ? "Реальная подготовка доступна только в приложении BetterFy." : "Real preparation is only available in the BetterFy desktop app.") : preview ? (isRu ? "В гостевом просмотре запись в игру недоступна." : "Game installation is unavailable in guest preview.") : !windows ? (isRu ? "Установка доступна только на Windows. На Mac можно проверить интерфейс и подготовить ресурсы." : "Installation is Windows-only. On Mac you can inspect the UI and prepare resources.") : !installation.verified ? (isRu ? "Сначала подключи настоящую установку Dota 2 в настройках." : "Connect a real Dota 2 installation in Settings first.") : (isRu ? "В живой пилот входят Tree Mod, Show Net Worth и Unit Query HUD. Убери остальные игровые моды из сборки." : "The live pilot supports Tree Mod, Show Net Worth, and Unit Query HUD. Remove other game mods from the build.")}</p></div>}
    {plan && !operationId && <div className="s-tree-pilot-language"><div><strong>{isRu ? "Выбери язык Dota для сборки" : "Choose Dota language for the build"}</strong><p>{isRu ? "Сборка будет записана в папку выбранного языка. Dutch проверен с Tree Mod на Windows; Show Net Worth и остальные языки требуют проверки." : "The build goes into the selected language folder. Dutch was verified with Tree Mod on Windows; Show Net Worth and the other languages still need testing."}</p></div><select aria-label={isRu ? "Язык Dota для установки" : "Dota language for installation"} value={selectedLanguage} onChange={event => setSelectedLanguage(event.target.value as GameLanguage | "")}><option value="">{isRu ? "Выбери язык" : "Choose language"}</option><option value="russian">{isRu ? "Русский · проверка нужна" : "Russian · needs testing"}</option><option value="koreana">{isRu ? "Корейский · проверка нужна" : "Korean · needs testing"}</option><option value="schinese">{isRu ? "Китайский · проверка нужна" : "Chinese · needs testing"}</option><option value="dutch">{isRu ? "Нидерландский · Tree Mod проверен" : "Dutch · Tree Mod verified"}</option></select></div>}
    {plan && !operationId && <>
      <div className="s-tree-pilot-order" aria-label={isRu ? "Порядок приоритета модов" : "Mod priority order"}>
        <div><strong>{isRu ? "Порядок внутри VPK" : "Order inside the VPK"}</strong><p>{isRu ? "Если два мода меняют один ресурс, верхний остаётся в итоговой сборке." : "If two mods change the same resource, the higher one remains in the final build."}</p></div>
        <ol>{plan.contributions.map((item, index) => <li key={item.packageId} title={isRu ? `${item.effectiveResources} из ${item.inputResources} ресурсов войдут в VPK` : `${item.effectiveResources} of ${item.inputResources} resources remain in the VPK`}><span>{String(index + 1).padStart(2, "0")}</span><b>{packageLabel(item.packageId)}</b><em className={item.shadowedResources > 0 ? "has-overrides" : ""}>{item.effectiveResources}/{item.inputResources}</em></li>)}</ol>
      </div>
      <div className="s-tree-pilot-plan"><span><b>{isRu ? "Куда будет записан файл" : "File destination"}</b><code>{selectedLanguage ? `game/dota_${selectedLanguage}/${plan.targetFile}` : "—"}</code></span><span><b>{isRu ? "Состав" : "Bundle"}</b>{plan.packageCount} · {plan.resourceCount} {isRu ? "ресурсов" : "resources"}</span><span><b>{isRu ? "Совпадения" : "Overlaps"}</b>{plan.duplicateResources + plan.overriddenResources || (isRu ? "Нет" : "None")}</span><span><b>{isRu ? "Размер VPK" : "VPK size"}</b>{(plan.vpkBytes / 1024).toFixed(1)} KB</span><span><b>SHA-256</b><code>{plan.vpkSha256.slice(0, 12)}…</code></span></div>
      {(plan.duplicates.length > 0 || plan.overrides.length > 0) && <details className="s-tree-pilot-conflicts"><summary>{isRu ? "Показать решение совпадений" : "Show overlap resolution"}</summary>{plan.overrides.map(item => <p key={`override-${item.path}`}><code>{item.path}</code><span>{packageLabel(item.winnerPackageId)} → {isRu ? "оставлен; ниже пропущены" : "kept; lower skipped"}: {item.shadowedPackageIds.map(packageLabel).join(", ")}</span></p>)}{plan.duplicates.map(item => <p key={`duplicate-${item.path}`}><code>{item.path}</code><span>{isRu ? "Одинаковый ресурс сохранён один раз" : "Identical resource stored once"}: {packageLabel(item.keptPackageId)}</span></p>)}</details>}
    </>}
    {plan && !operationId && canInstall && <div className="s-tree-pilot-steam"><div><strong>{isRu ? "Steam-профиль для установки" : "Steam profile for installation"}</strong><p>{isRu ? "BetterFy мягко закроет Dota 2 и Steam, запишет файл, добавит выбранный -language в этот профиль и снова откроет Steam. Если профиль не выбран, параметр нужно будет вставить вручную." : "BetterFy will gracefully close Dota 2 and Steam, write the file, add the selected -language option to this profile, and reopen Steam. Without a profile, paste the option manually."}</p></div><select aria-label={isRu ? "Steam-профиль" : "Steam profile"} value={selectedProfile} onChange={event => setSelectedProfile(event.target.value)}><option value="">{isRu ? "Вручную" : "Manual setup"}</option>{profiles.map(profile => <option key={profile.profileToken} value={profile.profileToken} disabled={profile.status !== "ready" && profile.status !== "already_managed"}>{isRu ? "Профиль" : "Profile"} {profile.profileIndex} · {profile.status === "ready" ? (isRu ? "готов" : "ready") : profile.status === "already_managed" ? (isRu ? "язык задан" : "language set") : (isRu ? "конфликт" : "conflict")}</option>)}</select></div>}
    {operationId ? <div className="s-tree-pilot-result">
      {verifiedInstall && packageVerified ? <Check /> : <TriangleAlert />}
      <span>{verifiedInstall
        ? packageVerified
          ? (isRu ? `BetterFy записал сборку из ${installedPackageIds.length || 1} модов. Проверь игру; при проблеме верни исходное состояние.` : `BetterFy installed a ${installedPackageIds.length || 1}-mod build. Check the game; restore the previous state if needed.`)
          : (isRu ? "Файл и журнал BetterFy совпадают, но исходные ресурсы недоступны для повторной проверки. Откат остаётся доступен; для новой установки подготовь ресурсы заново." : "The installed file matches BetterFy's journal, but source resources are unavailable for another check. Restore remains available; prepare resources again before reinstalling.")
        : (isRu ? "Найдена сохранённая операция, но состояние файла ещё не подтверждено. Проверь установку или выполни откат." : "A saved operation was found, but the file state is not verified yet. Check or restore the installation.")}</span>
      <button className="s-btn" disabled={phase !== "idle" || !canInstall || steamRecoveryRequired} onClick={() => void restore()}>{phase === "restore" ? <LoaderCircle className="s-spin" /> : <RotateCcw />}{isRu ? "Откатить" : "Restore"}</button>
    </div> : <div className="s-tree-pilot-actions">{!plan ? <><button className="s-btn s-btn-primary" disabled={phase !== "idle" || !desktop || !supportedBundle} onClick={() => void prepare()}>{phase === "download" ? <LoaderCircle className="s-spin" /> : <Download />}{phase === "download" ? (isRu ? "Проверяем ресурсы…" : "Verifying resources…") : (isRu ? "Подготовить сборку" : "Prepare build")}</button>{phase === "download" && <button className="s-btn" onClick={() => void cancelPrepare()}>{isRu ? "Отменить" : "Cancel"}</button>}</> : <><span><Check />{isRu ? "Ресурсы и VPK проверены" : "Resources and VPK verified"}</span><button className="s-btn s-btn-primary" disabled={!canInstall || !selectedLanguage || phase !== "idle" || !plan.deployEnabled} onClick={() => void install()}>{phase === "install" ? <LoaderCircle className="s-spin" /> : <ShieldCheck />}{phase === "install" ? (isRu ? "Устанавливаем…" : "Installing…") : (isRu ? "Установить сборку" : "Install build")}<ArrowRight /></button></>}</div>}
    {phase === "download" && <div className="s-tree-pilot-progress" role="progressbar" aria-valuenow={verifiedResources} aria-valuemin={0} aria-valuemax={totalResources || 1} aria-label={isRu ? "Проверенные ресурсы" : "Verified resources"}><span>{isRu ? "Проверено ресурсов" : "Resources verified"} · {verifiedResources}/{totalResources || "—"}</span><div><i style={{ width: `${totalResources ? verifiedResources / totalResources * 100 : 0}%` }} /></div></div>}
    {operationId && installedLanguage && <div className="s-tree-pilot-steam"><div><strong>{isRu ? "Где находится сборка" : "Where the build is installed"}</strong><p><code>game/dota_{installedLanguage}/pak66_dir.vpk</code></p>{installedPackageIds.length > 0 && <p>{installedPackageIds.map(packageLabel).join(" · ")}</p>}<p>{!packageVerified ? (isRu ? "Исходные ресурсы сейчас не подтверждены. Настройка Steam заблокирована; можно откатить установленный файл." : "Source resources are not currently verified. Steam setup is blocked; the installed file can be restored.") : steamStarted ? (isRu ? "Выбранный Steam-профиль использует нужный язык; Steam открыт. Запусти Dota 2 и проверь каждый мод сборки." : "The selected Steam profile uses the required language; Steam is open. Launch Dota 2 and check every mod in the build.") : steamRestarted ? (isRu ? "Steam открыт. Добавь параметр ниже в свойствах Dota 2 → параметры запуска, затем запусти игру." : "Steam is open. Add the option below in Dota 2 properties → launch options, then launch the game.") : (isRu ? "Файл записан, но запуск Steam или его настройка не завершились. Проверь профиль и повтори настройку." : "The file was written, but Steam startup or setup did not finish. Check the profile and retry setup.")}</p><p>{isRu ? "Чтобы выбрать другой язык, сначала откати эту установку." : "Restore this installation before choosing another language."}</p></div><div className="s-tree-pilot-command"><code>-language {installedLanguage}</code><button className="s-btn" type="button" onClick={() => void copyLaunchOption()}><Copy />{copied ? (isRu ? "Скопировано" : "Copied") : (isRu ? "Копировать" : "Copy")}</button></div>{canInstall && !steamStarted && packageVerified && <><select aria-label={isRu ? "Steam-профиль" : "Steam profile"} value={selectedProfile} onChange={event => setSelectedProfile(event.target.value)}><option value="">{isRu ? "Вручную" : "Manual setup"}</option>{profiles.map(profile => <option key={profile.profileToken} value={profile.profileToken} disabled={profile.status !== "ready" && profile.status !== "already_managed"}>{isRu ? "Профиль" : "Profile"} {profile.profileIndex} · {profile.status === "ready" ? (isRu ? "готов" : "ready") : profile.status === "already_managed" ? (isRu ? "язык задан" : "language set") : (isRu ? "конфликт" : "conflict")}</option>)}</select><button className="s-btn" disabled={phase !== "idle" || !selectedProfile} onClick={() => void activateSteam()}>{phase === "steam" ? <LoaderCircle className="s-spin" /> : <ArrowRight />}{isRu ? "Настроить Steam" : "Set up Steam"}</button></>}</div>}
    {!operationId && steamOperationId && canInstall && <button className="s-btn" disabled={phase !== "idle"} onClick={() => void restoreSteam()}><RotateCcw />{isRu ? "Восстановить параметры Steam" : "Restore Steam settings"}</button>}
    {canInstall && <div className="s-tree-pilot-recovery">{steamRecoveryRequired && <span role="alert">{isRu ? "Изменение Steam было прервано. Сначала восстанови Steam, затем откатывай файл игры." : "A Steam change was interrupted. Recover Steam before restoring the game file."}</span>}<button className="s-text-button" disabled={phase !== "idle" || recovering} onClick={() => void recover()}>{recovering ? <LoaderCircle className="s-spin" /> : <RotateCcw />}{steamRecoveryRequired ? (isRu ? "Восстановить Steam и проверить установку" : "Recover Steam and check installation") : (isRu ? "Проверить прерванную установку" : "Check interrupted installation")}</button>{recoveryMessage && <span role="status">{recoveryMessage}</span>}</div>}
    {error && <p className="s-tree-pilot-error" role="alert">{explainError(error, isRu)}</p>}
    <small className="s-tree-pilot-foot">{isRu ? "Источник: Egezenn/dota2-minify · Tree Mod: robbyz512. Tree Mod проверен в игре только с Dutch; Show Net Worth и Unit Query HUD требуют Windows-проверки. Параметр -language меняет язык текста и может повлиять на озвучку. BetterFy мягко закрывает Dota 2 и Steam; после отката Steam останется закрытым." : "Source: Egezenn/dota2-minify · Tree Mod: robbyz512. Tree Mod was verified in game only with Dutch; Show Net Worth and Unit Query HUD still require Windows verification. The -language option changes text language and may affect audio. BetterFy closes Dota 2 and Steam gracefully; Steam stays closed after restore."}</small>
  </section>;
}
