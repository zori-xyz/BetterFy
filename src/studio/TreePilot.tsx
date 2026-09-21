import { useEffect, useState } from "react";
import { ArrowRight, Check, Download, LoaderCircle, RotateCcw, ShieldCheck, TriangleAlert } from "lucide-react";
import { invoke } from "@tauri-apps/api/core";
import { engineBridge, type GameDeploymentReceipt, type GameInstallation, type SteamProfileSummary } from "../engine";
import { useLocale } from "../i18n";
import { getStorageItem, setStorageItem } from "../storage";

type TreePlan = {
  planId: string;
  packageId: string;
  sourceCommit: string;
  targetFile: string;
  resourceCount: number;
  resourceBytes: number;
  vpkBytes: number;
  vpkSha256: string;
  compatibility: "unknown";
  distribution: "internal_pilot";
  deployEnabled: boolean;
};
type TreeDownloadStatus = { phase: "downloading" | "verifying" | "ready" | "failed" | "cancelled"; verifiedResources: number; totalResources: number; errorCode: string | null; plan: TreePlan | null };

const operationKey = "betterfy:tree-pilot-operation";
const steamOperationKey = "betterfy:tree-pilot-steam-operation";
const codeOf = (error: unknown) => error instanceof Error ? error.message : String(error);
const explainError = (code: string, isRu: boolean) => {
  const messages: Record<string, [string, string]> = {
    runtime_busy: ["Закрой Dota 2 и Steam, затем повтори.", "Close Dota 2 and Steam, then retry."],
    platform_not_supported: ["Установка доступна только на Windows.", "Installation is Windows-only."],
    download_dns_failed: ["Нет доступа к источнику. Проверь подключение к сети.", "The source is unreachable. Check your connection."],
    download_transport_failed: ["Не удалось скачать ресурс. Повтори подготовку — проверенные файлы сохранятся.", "A resource could not be downloaded. Retry; verified files are kept."],
    build_plan_stale: ["План устарел. Подготовь Tree Mod заново.", "The plan is stale. Prepare Tree Mod again."],
    deployment_target_foreign: ["Целевой файл занят другой модификацией. BetterFy не будет его заменять.", "Another modification owns the target. BetterFy will not overwrite it."],
    deployment_conflict: ["Файл игры изменился после установки. Автоматический откат остановлен.", "The game file changed after installation. Automatic restore was stopped."],
    rollback_conflict: ["Файл игры изменился после установки. Автоматический откат остановлен.", "The game file changed after installation. Automatic restore was stopped."],
    operation_not_saved: ["Установка завершена, но браузерное хранилище недоступно. Не закрывай приложение до отката или проверки.", "Installation finished, but local storage is unavailable. Keep the app open until restoration or verification."],
    steam_profile_conflict: ["Этот Steam-профиль нельзя менять автоматически. Выбери другой или проверь параметры запуска вручную.", "This Steam profile cannot be changed automatically. Choose another or inspect its launch options."],
    steam_activation_not_ready: ["Параметры Steam не подтверждены. Не считай мод активным в игре.", "Steam settings were not verified. Do not assume the mod is active in game."],
    steam_recovery_required: ["Найдено прерванное изменение Steam. Сначала восстанови его в диагностике.", "An interrupted Steam change was found. Recover it in diagnostics first."],
  };
  return messages[code]?.[isRu ? 0 : 1] ?? (isRu ? `Операция остановлена (${code}). Проверь состояние установки перед запуском игры.` : `Operation stopped (${code}). Check installation state before launching the game.`);
};

export default function TreePilot({ ids, installation, preview }: { ids: string[]; installation: GameInstallation; preview: boolean }) {
  const { isRu } = useLocale();
  const [plan, setPlan] = useState<TreePlan | null>(null);
  const storageKey = `${operationKey}:${installation.path}`;
  const steamStorageKey = `${steamOperationKey}:${installation.path}`;
  const [operationId, setOperationId] = useState(() => getStorageItem(storageKey));
  const [verifiedInstall, setVerifiedInstall] = useState(false);
  const [steamOperationId, setSteamOperationId] = useState(() => getStorageItem(steamStorageKey));
  const [phase, setPhase] = useState<"idle" | "download" | "install" | "steam" | "restore">("idle");
  const [verifiedResources, setVerifiedResources] = useState(0);
  const [recovering, setRecovering] = useState(false);
  const [recoveryMessage, setRecoveryMessage] = useState("");
  const [profiles, setProfiles] = useState<SteamProfileSummary[]>([]);
  const [selectedProfile, setSelectedProfile] = useState("");
  const [steamStarted, setSteamStarted] = useState(false);
  const [error, setError] = useState("");
  const onlyTree = ids.length === 1 && ids[0] === "minify-tree-mod";
  const windows = /Windows/i.test(navigator.userAgent);
  const desktop = "__TAURI_INTERNALS__" in window;
  const canInstall = desktop && windows && !preview && installation.verified && onlyTree;
  function applyDownloadStatus(status: TreeDownloadStatus | null) {
    if (!status) return;
    setVerifiedResources(status.verifiedResources);
    if (status.phase === "ready") {
      if (status.plan?.packageId === "minify.tree-mod" && status.plan.resourceCount === 21 && status.plan.planId.startsWith("sha256:")) {
        setPlan(status.plan);
        setError("");
      } else setError("tree_plan_invalid");
      setPhase("idle");
    } else if (status.phase === "failed" || status.phase === "cancelled") {
      setError(status.phase === "cancelled" ? "" : status.errorCode ?? "download_failed");
      setPhase("idle");
    } else setPhase("download");
  }
  useEffect(() => {
    if (!desktop) return;
    let active = true;
    invoke<TreeDownloadStatus | null>("tree_pilot_download_status").then(status => {
      if (active) applyDownloadStatus(status);
    }).catch(() => undefined);
    return () => { active = false; };
  }, [desktop]);
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
    invoke<GameDeploymentReceipt | null>("current_tree_pilot", { gamePath: installation.path })
      .then(receipt => {
        if (!active) return;
        setOperationId(receipt?.operationId ?? null);
        setVerifiedInstall(Boolean(receipt));
        setStorageItem(storageKey, receipt?.operationId ?? "");
      })
      .catch(cause => {
        const code = codeOf(cause);
        if (active && code !== "content_store_invalid") setError(code);
      });
    return () => { active = false; };
  }, [desktop, windows, preview, installation.verified, installation.path, storageKey]);
  useEffect(() => {
    if (!operationId || !canInstall) return;
    let active = true;
    engineBridge.listSteamProfiles().then(found => {
      if (!active) return;
      setProfiles(found);
      setSelectedProfile(previous => found.some(profile => profile.profileToken === previous) ? previous : found.find(profile => profile.status === "ready" || profile.status === "already_managed")?.profileToken ?? "");
    }).catch(cause => { if (active) setError(codeOf(cause)); });
    return () => { active = false; };
  }, [operationId, canInstall]);

  async function prepare() {
    if (!desktop || phase !== "idle") return;
    setError("");
    setPhase("download");
    try {
      applyDownloadStatus(await invoke<TreeDownloadStatus>("begin_tree_pilot_download"));
    } catch (cause) { setError(codeOf(cause)); setPhase("idle"); }
  }

  async function cancelPrepare() {
    try { await invoke<TreeDownloadStatus>("cancel_tree_pilot_download"); }
    catch (cause) { setError(codeOf(cause)); }
  }

  async function install() {
    if (!plan || !canInstall || phase !== "idle") return;
    setError("");
    setPhase("install");
    try {
      const runtime = await engineBridge.inspectRuntime();
      if (!runtime.patchReady) throw new Error("runtime_busy");
      const receipt = await invoke<GameDeploymentReceipt>("install_tree_pilot", {
        request: { gamePath: installation.path, expectedPlanId: plan.planId, confirmed: true },
      });
      if (!receipt.committed || !receipt.backupVerified) throw new Error("deployment_unverified");
      setOperationId(receipt.operationId);
      setVerifiedInstall(true);
      if (!setStorageItem(storageKey, receipt.operationId)) setError("operation_not_saved");
    } catch (cause) { setError(codeOf(cause)); }
    finally { setPhase("idle"); }
  }

  async function restore() {
    if (!operationId || !canInstall || phase !== "idle") return;
    setError("");
    setPhase("restore");
    try {
      const receipt = await engineBridge.rollbackGameDeployment(installation.path, operationId);
      if (!receipt.rolledBack) throw new Error("rollback_failed");
      if (steamOperationId) {
        const steamReceipt = await engineBridge.rollbackSteamLaunchOptions(steamOperationId);
        if (!steamReceipt.rolledBack) throw new Error("steam_rollback_failed");
        setSteamOperationId(null);
        setStorageItem(steamStorageKey, "");
      }
      setOperationId(null);
      setVerifiedInstall(false);
      setStorageItem(storageKey, "");
      setPlan(null);
      setSteamStarted(false);
    } catch (cause) { setError(codeOf(cause)); }
    finally { setPhase("idle"); }
  }

  async function activateSteam() {
    if (!operationId || !canInstall || !selectedProfile || phase !== "idle") return;
    setError("");
    setPhase("steam");
    try {
      const profile = profiles.find(item => item.profileToken === selectedProfile);
      if (!profile || !["ready", "already_managed"].includes(profile.status)) throw new Error("steam_profile_conflict");
      const runtime = await engineBridge.inspectRuntime();
      if (!runtime.patchReady) throw new Error("runtime_busy");
      const preview = await engineBridge.previewSteamLaunchOptions(selectedProfile);
      const receipt = await engineBridge.applySteamLaunchOptions(preview);
      if (!receipt.committed || (receipt.changed && !receipt.backupVerified)) throw new Error("steam_activation_not_ready");
      if (receipt.operationId) {
        setSteamOperationId(receipt.operationId);
        if (!setStorageItem(steamStorageKey, receipt.operationId)) throw new Error("operation_not_saved");
      }
      await engineBridge.startSteamAfterProfile(selectedProfile, receipt.operationId);
      setSteamStarted(true);
    } catch (cause) { setError(codeOf(cause)); }
    finally { setPhase("idle"); }
  }

  async function restoreSteam() {
    if (!steamOperationId || !canInstall || phase !== "idle") return;
    setPhase("restore");
    setError("");
    try {
      const receipt = await engineBridge.rollbackSteamLaunchOptions(steamOperationId);
      if (!receipt.rolledBack) throw new Error("steam_rollback_failed");
      setSteamOperationId(null);
      setStorageItem(steamStorageKey, "");
      setSteamStarted(false);
    } catch (cause) { setError(codeOf(cause)); }
    finally { setPhase("idle"); }
  }

  async function recover() {
    if (!canInstall || phase !== "idle" || recovering) return;
    setError("");
    setRecoveryMessage("");
    setRecovering(true);
    try {
      const result = await engineBridge.recoverGameDeployments(installation.path);
      setRecoveryMessage(result.inspected === 0
        ? (isRu ? "Незавершённых операций не найдено." : "No interrupted operations found.")
        : (isRu ? `Проверено ${result.inspected}; откат выполнен для ${result.rolledBack}.` : `Inspected ${result.inspected}; restored ${result.rolledBack}.`));
      const current = await invoke<GameDeploymentReceipt | null>("current_tree_pilot", { gamePath: installation.path });
      setOperationId(current?.operationId ?? null);
      setVerifiedInstall(Boolean(current));
      setStorageItem(storageKey, current?.operationId ?? "");
    } catch (cause) { setError(codeOf(cause)); }
    finally { setRecovering(false); }
  }

  return <section className="s-tree-pilot" aria-labelledby="tree-pilot-title">
    <div className="s-tree-pilot-head"><span className="s-tree-pilot-symbol"><ShieldCheck /></span><div><small>{isRu ? "ПЕРВЫЙ РЕАЛЬНЫЙ МОД · WINDOWS-ПИЛОТ" : "FIRST REAL MOD · WINDOWS PILOT"}</small><h2 id="tree-pilot-title">Tree Mod</h2></div><span className="s-tree-pilot-mark">01 / 01</span></div>
    <p>{isRu ? "Заменяет деревья небольшими круглыми кустами. Нужен стандартный ландшафт. Сейчас устанавливается только этот мод — другие элементы сборки остаются предварительным выбором." : "Replaces trees with small round bushes. Requires the default terrain. Only this mod is installed; other build items remain a preview selection."}</p>
    <div className="s-tree-pilot-steps"><span className={plan ? "done" : ""}>01 <b>{isRu ? "Скачать и сверить 21 файл" : "Download and verify 21 files"}</b></span><span className={operationId ? "done" : ""}>02 <b>{isRu ? "Записать с возможностью отката" : "Install with rollback"}</b></span><span className={steamStarted ? "done" : ""}>03 <b>{isRu ? "Настроить и открыть Steam" : "Set up and open Steam"}</b></span><span>04 <b>{isRu ? "Проверить в Dota 2" : "Check in Dota 2"}</b></span></div>
    {!canInstall && <div className="s-inline-note warning"><TriangleAlert /><p>{!desktop ? (isRu ? "Реальная подготовка доступна только в приложении BetterFy." : "Real preparation is only available in the BetterFy desktop app.") : preview ? (isRu ? "В гостевом просмотре запись в игру недоступна." : "Game installation is unavailable in guest preview.") : !windows ? (isRu ? "Установка доступна только на Windows. На Mac можно проверить интерфейс и подготовить ресурсы." : "Installation is Windows-only. On Mac you can inspect the UI and prepare resources.") : !installation.verified ? (isRu ? "Сначала подключи настоящую установку Dota 2 в настройках." : "Connect a real Dota 2 installation in Settings first.") : (isRu ? "Для пилота оставь в сборке только Tree Mod." : "Keep only Tree Mod in the build for this pilot.")}</p></div>}
    {plan && !operationId && <div className="s-tree-pilot-plan"><span><b>{isRu ? "Цель" : "Target"}</b>game/dota_dutch/{plan.targetFile}</span><span><b>{isRu ? "Размер VPK" : "VPK size"}</b>{(plan.vpkBytes / 1024).toFixed(1)} KB</span><span><b>SHA-256</b><code>{plan.vpkSha256.slice(0, 12)}…</code></span></div>}
    {operationId ? <div className="s-tree-pilot-result">{verifiedInstall ? <Check /> : <TriangleAlert />}<span>{verifiedInstall ? (isRu ? "BetterFy записал Tree Mod. Проверь игру; при проблеме верни исходное состояние." : "BetterFy installed Tree Mod. Check the game; restore the previous state if needed.") : (isRu ? "Найдена сохранённая операция, но состояние файла ещё не подтверждено. Проверь установку или выполни откат." : "A saved operation was found, but the file state is not verified yet. Check or restore the installation.")}</span><button className="s-btn" disabled={phase !== "idle" || !canInstall} onClick={() => void restore()}>{phase === "restore" ? <LoaderCircle className="s-spin" /> : <RotateCcw />}{isRu ? "Откатить" : "Restore"}</button></div> : <div className="s-tree-pilot-actions">{!plan ? <><button className="s-btn s-btn-primary" disabled={phase !== "idle" || !desktop} onClick={() => void prepare()}>{phase === "download" ? <LoaderCircle className="s-spin" /> : <Download />}{phase === "download" ? (isRu ? "Проверяем ресурсы…" : "Verifying resources…") : (isRu ? "Подготовить Tree Mod" : "Prepare Tree Mod")}</button>{phase === "download" && <button className="s-btn" onClick={() => void cancelPrepare()}>{isRu ? "Отменить" : "Cancel"}</button>}</> : <><span><Check />{isRu ? "Ресурсы и VPK проверены" : "Resources and VPK verified"}</span><button className="s-btn s-btn-primary" disabled={!canInstall || phase !== "idle" || !plan.deployEnabled} onClick={() => void install()}>{phase === "install" ? <LoaderCircle className="s-spin" /> : <ShieldCheck />}{phase === "install" ? (isRu ? "Устанавливаем…" : "Installing…") : (isRu ? "Установить Tree Mod" : "Install Tree Mod")}<ArrowRight /></button></>}</div>}
    {phase === "download" && <div className="s-tree-pilot-progress" role="progressbar" aria-valuenow={verifiedResources} aria-valuemin={0} aria-valuemax={21} aria-label={isRu ? "Проверенные ресурсы" : "Verified resources"}><span>{isRu ? "Проверено ресурсов" : "Resources verified"} · {verifiedResources}/21</span><div><i style={{ width: `${verifiedResources / 21 * 100}%` }} /></div></div>}
    {operationId && canInstall && <div className="s-tree-pilot-steam"><div><strong>{isRu ? "Следующий шаг — Steam" : "Next: Steam"}</strong><p>{isRu ? "BetterFy добавит аргумент запуска только в выбранный профиль и откроет Steam. Dota 2 не запустится автоматически." : "BetterFy will add the launch argument only to the selected profile and open Steam. Dota 2 will not start automatically."}</p>{profiles.length === 0 && <p>{isRu ? "Steam-профиль пока не найден. Открой Steam хотя бы один раз и повторно открой эту вкладку." : "No Steam profile found yet. Open Steam once, then reopen this tab."}</p>}</div>{profiles.length > 0 && <select aria-label={isRu ? "Steam-профиль" : "Steam profile"} value={selectedProfile} onChange={event => setSelectedProfile(event.target.value)}><option value="">{isRu ? "Выбери профиль" : "Select profile"}</option>{profiles.map(profile => <option key={profile.profileToken} value={profile.profileToken} disabled={profile.status !== "ready" && profile.status !== "already_managed"}>{isRu ? "Профиль" : "Profile"} {profile.profileIndex} · {profile.status === "ready" ? (isRu ? "готов" : "ready") : profile.status === "already_managed" ? "BetterFy" : (isRu ? "конфликт" : "conflict")}</option>)}</select>}<button className="s-btn" disabled={phase !== "idle" || !selectedProfile || steamStarted} onClick={() => void activateSteam()}>{phase === "steam" ? <LoaderCircle className="s-spin" /> : <ArrowRight />}{steamStarted ? (isRu ? "Steam открыт" : "Steam opened") : (isRu ? "Настроить и открыть Steam" : "Set up and open Steam")}</button></div>}
    {!operationId && steamOperationId && canInstall && <button className="s-btn" disabled={phase !== "idle"} onClick={() => void restoreSteam()}><RotateCcw />{isRu ? "Восстановить параметры Steam" : "Restore Steam settings"}</button>}
    {canInstall && <div className="s-tree-pilot-recovery"><button className="s-text-button" disabled={phase !== "idle" || recovering} onClick={() => void recover()}>{recovering ? <LoaderCircle className="s-spin" /> : <RotateCcw />}{isRu ? "Проверить прерванную установку" : "Check interrupted installation"}</button>{recoveryMessage && <span role="status">{recoveryMessage}</span>}</div>}
    {error && <p className="s-tree-pilot-error" role="alert">{explainError(error, isRu)}</p>}
    <small className="s-tree-pilot-foot">{isRu ? "Источник: Egezenn/dota2-minify · автор мода: robbyz512 · совместимость пока не подтверждена Windows-тестом. Закрой Dota 2 и Steam перед установкой и откатом." : "Source: Egezenn/dota2-minify · mod author: robbyz512 · compatibility awaits Windows testing. Close Dota 2 and Steam before installation or rollback."}</small>
  </section>;
}
