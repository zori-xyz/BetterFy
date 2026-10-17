import { useEffect, useRef, useState } from "react";
import {
  ArrowRight,
  Check,
  CircleDashed,
  Copy,
  Download,
  FileCheck2,
  FlaskConical,
  Gamepad2,
  LoaderCircle,
  Plus,
  RotateCcw,
  ShieldCheck,
  TriangleAlert,
} from "lucide-react";
import {
  EngineFault,
  engineBridge,
  type DeploymentEvidenceReport,
  type DeploymentStressFailurePoint,
  type DetectedLanguage,
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
import { journal } from "./builder/journal";
import { noticeFor } from "./builder/notices";
import { demoActive, demoBridge, isDesktopRuntime, useDemoSettings } from "./builder/demo";
import { usePhrase } from "./builder/phrases";
import { NoticeCard } from "./builder/NoticeCard";
import { DemoPanel } from "./builder/DemoPanel";
import { buildLanguages, languageLabel as languageName } from "./builder/languages";
import { Choice, ChoiceCards, ChoiceMenu, type ChoiceOption } from "./builder/Choice";
import { getStorageItem, setStorageItem } from "../storage";
import { WardrobeReview, formatBytes } from "./WardrobeReview";

const LANGUAGE_KEY = "betterfy:build-language";
// "auto" follows the language Steam starts Dota in, "manual" keeps the
// player's own pick. Nothing stored means auto.
const LANGUAGE_MODE_KEY = "betterfy:build-language-mode";
// English first, then the two slots checked on Windows; the rest sit in a menu.
const PINNED_LANGUAGES: GameLanguage[] = ["betterfy", "russian", "dutch"];
const isBuildLanguage = (value: string | null): value is GameLanguage =>
  buildLanguages.some((option) => option.value === value);

type LanguagePick = GameLanguage | "auto";
type Detection =
  | { status: "idle" | "loading" }
  | { status: "done"; result: DetectedLanguage | null }
  | { status: "failed"; code: string };
// Detection runs on every visit to the build screen; the journal only hears
// about it when the answer changes.
let loggedDetection = "";

/** A one-click fix for a mod relation, in catalog IDs. */
export type RelationFix =
  | { kind: "add"; add: string; requiredBy: string }
  | { kind: "keep"; keep: string; drop: string };

/** What the builder screen around the pilot card needs to know. */
export type PilotState = {
  plan: TreePilotPlan | null;
  installed: boolean;
  installedPackageIds: string[];
  installedLanguage: GameLanguage | null;
  justInstalled: boolean;
  busy: boolean;
  steamReady: boolean;
  /** The signed-in account may run stress tests. */
  developer: boolean;
};

const codeOf = (error: unknown) =>
  error instanceof EngineFault
    ? error.code
    : error instanceof Error
      ? error.message
      : String(error);
export default function TreePilot({
  ids,
  installation,
  preview,
  onState,
  onFixRelation,
  onConnectDota,
}: {
  ids: string[];
  installation: GameInstallation;
  preview: boolean;
  onState?: (state: PilotState) => void;
  /** Adds a required mod or drops a conflicting one, next to the message. */
  onFixRelation?: (fix: RelationFix) => void;
  /** Opens the Dota 2 connection screen. */
  onConnectDota?: () => void;
}) {
  const { isRu, language } = useLocale();
  const demo = demoActive();
  const bridge = demo ? demoBridge : engineBridge;
  const demoDeveloper = useDemoSettings().developer;
  const say = (tone: Parameters<typeof journal.log>[0], ru: string, en: string, detail?: string) =>
    journal.log(tone, ru, en, detail);
  const languageLabel = (value: GameLanguage) => languageName(value, isRu);
  const [justInstalled, setJustInstalled] = useState(false);
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
  // By default the build follows the language Steam starts Dota in. The
  // language last picked or installed, or Russian for a Russian interface,
  // is the fallback when Steam cannot tell.
  const [followDota, setFollowDota] = useState(
    () => getStorageItem(LANGUAGE_MODE_KEY) !== "manual",
  );
  const [chosenLanguage, setChosenLanguage] = useState<GameLanguage | "">(() => {
    const stored = getStorageItem(LANGUAGE_KEY);
    if (isBuildLanguage(stored)) return stored;
    return isRu ? "russian" : "";
  });
  const [detection, setDetection] = useState<Detection>({ status: "idle" });
  const detected = detection.status === "done" ? (detection.result?.language ?? null) : null;
  // Steam's own name for it, e.g. "english" for the BetterFy slot.
  const steamName = detection.status === "done" ? (detection.result?.steamLanguage ?? null) : null;
  // Empty while Steam is still being asked, so Install never runs on a guess.
  const selectedLanguage: GameLanguage | "" = followDota
    ? detection.status === "loading"
      ? ""
      : (detected ?? chosenLanguage)
    : chosenLanguage;
  const chooseLanguage = (value: LanguagePick) => {
    if (value === "auto" || value === detected) {
      setFollowDota(true);
      setStorageItem(LANGUAGE_MODE_KEY, "auto");
      return;
    }
    setFollowDota(false);
    setChosenLanguage(value);
    setStorageItem(LANGUAGE_MODE_KEY, "manual");
    setStorageItem(LANGUAGE_KEY, value);
  };
  const languageFieldRef = useRef<HTMLDivElement>(null);
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
  // The plan whose wardrobe path conflicts the person confirmed.
  const [acknowledgedPlan, setAcknowledgedPlan] = useState<string | null>(null);
  const phaseRef = useRef(phase);
  phaseRef.current = phase;
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
  // The browser preview runs the same screens against the demo bridge.
  const windows = demo || /Windows/i.test(navigator.userAgent);
  const desktop = demo || isDesktopRuntime();
  // Managing an existing installation (restore, Steam, recovery, evidence)
  // must not depend on what is currently selected: changing the selection
  // after installing would otherwise hide the only way back.
  const canManage = desktop && windows && (demo || (!preview && installation.verified));
  const canInstall = canManage && supportedBundle;
  // A skin that shares paths with another item installs only after the
  // person has seen the list and said so (the engine checks it again).
  const conflictsPending =
    (plan?.wardrobeConflicts.length ?? 0) > 0 && acknowledgedPlan !== plan?.planId;
  // Packages whose manifest records no in-game verification yet.
  const unverified = ids.filter((id) => !findPackage(id)?.verifiedLanguages?.length);
  function applyDownloadStatus(status: TreePilotDownloadStatus | null) {
    if (!status) return;
    setVerifiedResources(status.verifiedResources);
    setTotalResources(status.totalResources);
    if (
      phaseRef.current === "download" &&
      status.phase !== "downloading" &&
      status.phase !== "verifying"
    ) {
      if (status.phase === "ready")
        say(
          "ok",
          `Файлы проверены: ${status.verifiedResources} из ${status.totalResources}. Сборка собрана в план.`,
          `Files verified: ${status.verifiedResources} of ${status.totalResources}. The build plan is ready.`,
          status.plan?.planId.slice(0, 19),
        );
      else if (status.phase === "cancelled")
        say(
          "warn",
          "Подготовку остановили. Проверенные файлы сохранены.",
          "Preparation stopped. Verified files are kept.",
        );
      else
        say(
          "error",
          "Подготовка не удалась.",
          "Preparation failed.",
          status.errorCode ?? "download_failed",
        );
    }
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
    bridge
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
      bridge
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
    if (!canManage) return;
    let active = true;
    bridge
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
  }, [canManage, installation.path]);
  useEffect(() => {
    if (!canManage) return;
    let active = true;
    bridge
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
  // Reads Steam's own manifest for Dota (read-only) so the build can keep the
  // language the player already runs the game in.
  const wantsLanguage = canManage && ids.length > 0;
  useEffect(() => {
    if (!wantsLanguage) return;
    let active = true;
    setDetection({ status: "loading" });
    const note = (key: string, ...entry: Parameters<typeof journal.log>) => {
      if (key === loggedDetection) return;
      loggedDetection = key;
      journal.log(...entry);
    };
    bridge
      .detectDotaLanguage(installation.path)
      .then((result) => {
        if (!active) return;
        setDetection({ status: "done", result });
        const key = `${installation.path}|${result?.steamLanguage ?? ""}`;
        if (result?.language)
          note(
            key,
            "ok",
            `Steam запускает Dota на языке: ${languageName(result.language, true)}.`,
            `Steam starts Dota in ${languageName(result.language, false)}.`,
            `steam: ${result.steamLanguage}`,
          );
        else if (result)
          note(
            key,
            "warn",
            `Steam запускает Dota на языке «${result.steamLanguage}», а папки для него у BetterFy нет.`,
            `Steam starts Dota in "${result.steamLanguage}", and BetterFy has no folder for it.`,
          );
        else note(key, "warn", "Steam не указал язык Dota.", "Steam lists no language for Dota.");
      })
      .catch((cause) => {
        if (!active) return;
        const code = codeOf(cause);
        setDetection({ status: "failed", code });
        note(
          `${installation.path}|${code}`,
          "warn",
          "Не удалось узнать язык Dota из Steam.",
          "Could not read Dota's language from Steam.",
          code,
        );
      });
    return () => {
      active = false;
    };
  }, [wantsLanguage, installation.path]);
  useEffect(() => {
    if (!canManage) return;
    let active = true;
    bridge
      .treePilotStressCapabilities()
      .then((capabilities) => {
        if (active) setStressEnabled(capabilities.enabled);
      })
      .catch(() => {
        if (active) setStressEnabled(false);
      });
    bridge
      .collectTreePilotEvidence(installation.path)
      .then((report) => {
        if (active) setEvidence(report);
      })
      .catch(() => undefined);
    return () => {
      active = false;
    };
  }, [canManage, installation.path, demoDeveloper]);

  async function prepare() {
    if (!desktop || phase !== "idle") return;
    setError("");
    setPhase("download");
    phaseRef.current = "download";
    say(
      "step",
      `Скачиваем и сверяем файлы: ${ids.length} ${ids.length === 1 ? "мод" : ids.length < 5 ? "мода" : "модов"}.`,
      `Downloading and checking files for ${ids.length} ${ids.length === 1 ? "mod" : "mods"}.`,
    );
    try {
      applyDownloadStatus(
        await bridge.beginTreePilotDownload(
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
      await bridge.cancelTreePilotDownload();
    } catch (cause) {
      setError(codeOf(cause));
    }
  }

  async function install() {
    if (!plan || !canInstall || !selectedLanguage || phase !== "idle" || conflictsPending) return;
    setError("");
    setPhase("install");
    setJustInstalled(false);
    let installed = false;
    say(
      "step",
      `Проверяем папку языка: ${languageLabel(selectedLanguage)}.`,
      `Checking the ${languageLabel(selectedLanguage)} language folder.`,
      `game/dota_${selectedLanguage}`,
    );
    try {
      await bridge.previewTreeLanguage(installation.path, selectedLanguage);
      if (selectedProfile) {
        const profile = profiles.find((item) => item.profileToken === selectedProfile);
        if (!profile || !["ready", "already_managed"].includes(profile.status))
          throw new Error("steam_profile_conflict");
        await bridge.previewSteamLaunchOptions(selectedProfile, selectedLanguage);
      }
      say("step", "Мягко закрываем Dota 2 и Steam.", "Closing Dota 2 and Steam gracefully.");
      const runtime = await bridge.prepareRuntimeForPatch();
      if (!runtime.patchReady) throw new Error("runtime_busy");
      say(
        "step",
        "Сохраняем исходный файл и записываем сборку.",
        "Backing up the original file and writing the build.",
        plan.targetFile,
      );
      const receipt = await bridge.installTreePilot(
        installation.path,
        ids,
        plan.planId,
        selectedLanguage,
        plan.wardrobeConflicts.length > 0,
      );
      if (!receipt.committed || !receipt.backupVerified) throw new Error("deployment_unverified");
      installed = true;
      say(
        "ok",
        "Сборка записана, бэкап проверен.",
        "Build written, backup verified.",
        `dota_${receipt.language}/${plan.targetFile}`,
      );
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
        say(
          "step",
          `Добавляем -language ${receipt.language} в профиль Steam.`,
          `Adding -language ${receipt.language} to the Steam profile.`,
        );
        const steamPlan = await bridge.previewSteamLaunchOptions(selectedProfile, receipt.language);
        const steamReceipt = await bridge.applyTreeSteamLaunchOptions({
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
        say(
          "ok",
          "Параметр запуска добавлен, копия настроек Steam сохранена.",
          "Launch option added; Steam settings backed up.",
        );
        say("step", "Запускаем Steam.", "Starting Steam.");
        await bridge.startSteamAfterTreePilot({
          gamePath: installation.path,
          operationId: receipt.operationId,
          profileToken: selectedProfile,
          steamOperationId: steamReceipt.operationId,
        });
        setSteamStarted(true);
        setSteamRestarted(true);
      } else {
        say("step", "Запускаем Steam.", "Starting Steam.");
        await bridge.startSteamAfterTreePilot({
          gamePath: installation.path,
          operationId: receipt.operationId,
        });
        setSteamRestarted(true);
      }
      say(
        "ok",
        selectedProfile
          ? "Steam запущен с нужным языком."
          : "Steam запущен. Осталось вставить параметр запуска.",
        selectedProfile
          ? "Steam is running with the right language."
          : "Steam is running. Paste the launch option to finish.",
      );
      setJustInstalled(true);
    } catch (cause) {
      const code = codeOf(cause);
      setError(code);
      say("error", "Установка остановлена.", "Installation stopped.", code);
    } finally {
      setEngineActive(false);
      setPhase("idle");
      if (installed) saveEvidenceReport();
    }
  }

  async function restore() {
    if (!operationId || !canManage || phase !== "idle") return;
    setError("");
    setRecoveryMessage("");
    setPhase("restore");
    // Once BetterFy has closed Steam it always tries to start it again and
    // keeps a report, whether or not every step of the restore succeeded.
    let steamClosed = false;
    let buildRestored = false;
    let steamProblem = "";
    let keptUserChange = false;
    setJustInstalled(false);
    say(
      "step",
      "Откат: мягко закрываем Dota 2 и Steam.",
      "Restore: closing Dota 2 and Steam gracefully.",
    );
    try {
      if (steamRecoveryRequired) throw new Error("steam_recovery_required");
      const runtime = await bridge.prepareRuntimeForPatch();
      if (!runtime.patchReady) throw new Error("runtime_busy");
      steamClosed = true;
      if (steamOperationId) {
        say("step", "Возвращаем параметры запуска Steam.", "Restoring Steam launch options.");
        // The game file does not depend on Steam's launch options, so a
        // Steam problem is reported but does not keep the build installed.
        try {
          const steamReceipt = await bridge.rollbackSteamLaunchOptions(steamOperationId);
          if (!steamReceipt.rolledBack) throw new Error("steam_rollback_failed");
          keptUserChange = Boolean(steamReceipt.keptUserChange);
          setSteamOperationId(null);
        } catch (cause) {
          steamProblem = codeOf(cause);
        }
      }
      if (steamProblem)
        say(
          "warn",
          "Параметры Steam вернуть не удалось.",
          "Steam launch options were not restored.",
          steamProblem,
        );
      say(
        "step",
        "Возвращаем исходный файл из бэкапа.",
        "Restoring the original file from the backup.",
      );
      const receipt = await bridge.rollbackGameDeployment(installation.path, operationId);
      if (!receipt.rolledBack) throw new Error("rollback_failed");
      buildRestored = true;
      say("ok", "Исходные файлы Dota 2 на месте.", "Original Dota 2 files are back.");
      setOperationId(null);
      setInstalledLanguage(null);
      setInstalledPackageIds([]);
      setVerifiedInstall(false);
      setPackageVerified(false);
      setPlan(null);
      setSteamStarted(false);
      setSteamRecoveryRequired(false);
      if (steamProblem) setError(steamProblem);
    } catch (cause) {
      const code = codeOf(cause);
      setError(code);
      say("error", "Откат остановлен.", "Restore stopped.", code);
    } finally {
      if (steamClosed) {
        let steamRunning = false;
        try {
          steamRunning = (await bridge.startSteamAfterRestore()).steamRunning;
        } catch {
          steamRunning = false;
        }
        setSteamRestarted(steamRunning);
        if (steamRunning) say("ok", "Steam снова запущен.", "Steam is running again.");
        else
          say(
            "warn",
            "Steam не запустился сам — запусти его вручную.",
            "Steam did not start; start it manually.",
          );
        const parts: string[] = [];
        if (buildRestored) {
          parts.push(isRu ? "Сборка удалена из игры." : "The build was removed from the game.");
          if (steamProblem)
            parts.push(
              isRu
                ? "Параметры запуска Steam вернуть не удалось — нажми «Восстановить параметры Steam» или проверь их вручную."
                : "Steam launch options could not be restored. Use Restore Steam settings or check them manually.",
            );
          else if (keptUserChange)
            parts.push(
              isRu
                ? "BetterFy убрал свой параметр -language, а параметры запуска, изменённые после установки, оставил как есть."
                : "BetterFy removed its -language option and kept launch options that were changed after installation.",
            );
          else if (steamOperationId)
            parts.push(
              isRu
                ? "Параметры запуска Steam восстановлены."
                : "Steam launch options were restored.",
            );
        }
        parts.push(
          steamRunning
            ? isRu
              ? "Steam снова запущен."
              : "Steam is running again."
            : isRu
              ? "Steam не удалось запустить автоматически — запусти его вручную."
              : "Steam could not be started automatically. Start it manually.",
        );
        setRecoveryMessage(parts.join(" "));
        saveEvidenceReport();
      }
      setEngineActive(false);
      setPhase("idle");
    }
  }

  // The game replaced or removed BetterFy's file (usually a Dota update).
  // Nothing in the game folder is touched: the old record is only forgotten,
  // so a new install can start from what the game has now.
  async function releaseStale() {
    if (!canManage || phase !== "idle") return;
    setPhase("restore");
    setEngineActive(true);
    try {
      const released = await bridge.releaseStaleDeployment(installation.path);
      say(
        "ok",
        released.reason === "missing"
          ? "Файл BetterFy в игре уже удалён. Старая установка забыта."
          : "Файл в игре заменён не BetterFy. Старая установка забыта, файл не тронут.",
        released.reason === "missing"
          ? "BetterFy's file is already gone from the game. The old install is forgotten."
          : "The file in the game was replaced by something else. The old install is forgotten and the file was left alone.",
      );
      setError("");
      setOperationId(null);
      setInstalledLanguage(null);
      setInstalledPackageIds([]);
      setVerifiedInstall(false);
      setPackageVerified(false);
      setPlan(null);
      setSteamStarted(false);
      setRecoveryMessage(
        isRu
          ? steamOperationId
            ? "Старая установка забыта. Параметр -language в Steam можно вернуть кнопкой «Восстановить параметры Steam». Потом подготовь сборку заново."
            : "Старая установка забыта. Подготовь сборку заново; если в слоте остался чужой файл, выбери другой язык."
          : steamOperationId
            ? "The old install is forgotten. Use Restore Steam settings to remove the -language option, then prepare the build again."
            : "The old install is forgotten. Prepare the build again; if another file sits in this slot, pick a different language.",
      );
    } catch (cause) {
      const code = codeOf(cause);
      setError(code);
      say("error", "Не получилось отпустить установку.", "Could not let go of the install.", code);
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
    say(
      "step",
      "Настраиваем Steam для установленной сборки.",
      "Setting up Steam for the installed build.",
    );
    try {
      const profile = profiles.find((item) => item.profileToken === selectedProfile);
      if (!profile || !["ready", "already_managed"].includes(profile.status))
        throw new Error("steam_profile_conflict");
      const runtime = await bridge.prepareRuntimeForPatch();
      if (!runtime.patchReady) throw new Error("runtime_busy");
      if (steamOperationId) {
        await bridge.startSteamAfterTreePilot({
          gamePath: installation.path,
          operationId,
          profileToken: selectedProfile,
          steamOperationId,
        });
        setSteamStarted(true);
        setSteamRestarted(true);
        return;
      }
      const preview = await bridge.previewSteamLaunchOptions(selectedProfile, installedLanguage);
      const receipt = await bridge.applyTreeSteamLaunchOptions({
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
      await bridge.startSteamAfterTreePilot({
        gamePath: installation.path,
        operationId,
        profileToken: selectedProfile,
        steamOperationId: receipt.operationId,
      });
      setSteamStarted(true);
      setSteamRestarted(true);
      say("ok", "Steam настроен и запущен.", "Steam is set up and running.");
    } catch (cause) {
      const code = codeOf(cause);
      setError(code);
      say("error", "Настройка Steam остановлена.", "Steam setup stopped.", code);
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
      const runtime = await bridge.prepareRuntimeForPatch();
      if (!runtime.patchReady) throw new Error("runtime_busy");
      const receipt = await bridge.rollbackSteamLaunchOptions(steamOperationId);
      if (!receipt.rolledBack) throw new Error("steam_rollback_failed");
      setSteamOperationId(null);
      setSteamStarted(false);
      say("ok", "Параметры запуска Steam восстановлены.", "Steam launch options restored.");
    } catch (cause) {
      const code = codeOf(cause);
      setError(code);
      say("error", "Параметры Steam не восстановлены.", "Steam settings were not restored.", code);
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
    say("step", "Ищем прерванные операции.", "Looking for interrupted operations.");
    try {
      if (steamRecoveryRequired) {
        const runtime = await bridge.prepareRuntimeForPatch();
        if (!runtime.patchReady) throw new Error("runtime_busy");
        await bridge.recoverSteamLaunchOptions();
      }
      const result = await bridge.recoverGameDeployments(installation.path);
      setRecoveryMessage(
        result.inspected === 0
          ? isRu
            ? "Незавершённых операций не найдено."
            : "No interrupted operations found."
          : isRu
            ? `Проверено ${result.inspected}; откат выполнен для ${result.rolledBack}.`
            : `Inspected ${result.inspected}; restored ${result.rolledBack}.`,
      );
      say(
        "ok",
        result.inspected === 0
          ? "Прерванных операций нет."
          : `Проверено ${result.inspected}, откачено ${result.rolledBack}.`,
        result.inspected === 0
          ? "No interrupted operations."
          : `Inspected ${result.inspected}, restored ${result.rolledBack}.`,
      );
      const current = await bridge.currentTreePilot(installation.path);
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
      const report = await bridge.collectTreePilotEvidence(installation.path);
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
    bridge
      .saveTreePilotEvidence(installation.path)
      .then((saved) => {
        setSavedReport(saved.fileName);
        setReportError("");
        say("ok", "Отчёт сохранён.", "Report saved.", saved.fileName);
      })
      .catch((cause) => setReportError(codeOf(cause)));
  }

  async function openReportsFolder() {
    try {
      await bridge.openReportsFolder();
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
    say(
      "dev",
      `Стресс-тест: обрыв ${failurePoint === "after_prepared" ? "до" : "после"} записи.`,
      `Stress test: interrupt ${failurePoint === "after_prepared" ? "before" : "after"} publish.`,
      failurePoint,
    );
    try {
      const runtime = await bridge.prepareRuntimeForPatch();
      if (!runtime.patchReady) throw new Error("runtime_busy");
      try {
        await bridge.installTreePilotStress(
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
      const recovery = await bridge.recoverGameDeployments(installation.path);
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
      say("dev", "PASS · восстановление подтверждено.", "PASS · recovery verified.");
      saveEvidenceReport();
    } catch (cause) {
      const code = codeOf(cause);
      setError(code);
      say("error", "Стресс-тест не прошёл.", "Stress test failed.", code);
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
    say(
      "dev",
      `Стресс-тест Steam: обрыв ${failurePoint === "after_prepared" ? "до" : "после"} записи.`,
      `Steam stress test: interrupt ${failurePoint === "after_prepared" ? "before" : "after"} publish.`,
      failurePoint,
    );
    try {
      const profile = profiles.find((item) => item.profileToken === selectedProfile);
      if (!profile || !["ready", "already_managed"].includes(profile.status))
        throw new Error("steam_profile_conflict");
      const runtime = await bridge.prepareRuntimeForPatch();
      if (!runtime.patchReady) throw new Error("runtime_busy");
      const previewPlan = await bridge.previewSteamLaunchOptions(
        selectedProfile,
        installedLanguage,
      );
      if (!previewPlan.changed) throw new Error("stress_requires_unmanaged_profile");
      try {
        await bridge.applyTreeSteamLaunchOptionsStress(
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
      const receipts = await bridge.recoverSteamLaunchOptions();
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
      say("dev", "PASS · восстановление подтверждено.", "PASS · recovery verified.");
      saveEvidenceReport();
    } catch (cause) {
      const code = codeOf(cause);
      setError(code);
      say("error", "Стресс-тест не прошёл.", "Stress test failed.", code);
    } finally {
      setEngineActive(false);
      setStressPhase(null);
    }
  }

  // Mounted even without pilot mods selected so an existing installation can
  // still be found and restored; renders nothing when there is none.
  const busy = phase !== "idle" || recovering || Boolean(stressPhase);
  useEffect(() => {
    onState?.({
      plan,
      installed: Boolean(operationId),
      installedPackageIds,
      installedLanguage,
      justInstalled,
      busy,
      steamReady: steamStarted,
      developer: stressEnabled,
    });
  }, [
    plan,
    operationId,
    installedPackageIds,
    installedLanguage,
    justInstalled,
    busy,
    steamStarted,
    stressEnabled,
  ]);
  const activePhrase = usePhrase(phase === "idle" ? (recovering ? "restore" : null) : phase, isRu);
  if (!ids.length && !operationId && !steamOperationId && !steamRecoveryRequired) return null;
  const notice = error ? noticeFor(error, isRu) : null;
  // Relation fixes act on catalog IDs; a package the catalog does not list
  // (a superseded one) gets the message without a button.
  const catalogIdOf = (packageId: string) => {
    const catalogId = findPackage(packageId)?.catalogId;
    return catalogId && modById.has(catalogId) ? catalogId : null;
  };
  const relationFixes: Array<{ fix: RelationFix; label: string }> = [];
  if (relation && onFixRelation) {
    const first = catalogIdOf(relation.packageId);
    const other = catalogIdOf(relation.otherId);
    const firstName = packageLabel(relation.packageId);
    const otherName = packageLabel(relation.otherId);
    if (first && other && relation.kind === "missing")
      relationFixes.push({
        fix: { kind: "add", add: other, requiredBy: first },
        label: isRu ? `Добавить «${otherName}»` : `Add ${otherName}`,
      });
    else if (first && other)
      relationFixes.push(
        {
          fix: { kind: "keep", keep: first, drop: other },
          label: isRu ? `Оставить «${firstName}»` : `Keep ${firstName}`,
        },
        {
          fix: { kind: "keep", keep: other, drop: first },
          label: isRu ? `Оставить «${otherName}»` : `Keep ${otherName}`,
        },
      );
  }
  const blockerKind =
    !canManage || (ids.length > 0 && !supportedBundle)
      ? !desktop
        ? "desktop"
        : preview && !demo
          ? "preview"
          : !windows
            ? "windows"
            : !installation.verified && !demo
              ? "connect"
              : relation
                ? "relation"
                : "pilot"
      : null;
  const canConnect = blockerKind === "connect" && Boolean(onConnectDota);
  const canFixRelation = blockerKind === "relation" && relationFixes.length > 0;
  const blocker =
    blockerKind === "desktop"
      ? isRu
        ? "Реальная подготовка доступна только в приложении BetterFy."
        : "Real preparation is only available in the BetterFy desktop app."
      : blockerKind === "preview"
        ? isRu
          ? "В гостевом просмотре запись в игру недоступна."
          : "Game installation is unavailable in guest preview."
        : blockerKind === "windows"
          ? isRu
            ? "Установка доступна только на Windows. На Mac можно посмотреть интерфейс и подготовить ресурсы."
            : "Installation is Windows-only. On Mac you can inspect the UI and prepare resources."
          : blockerKind === "connect"
            ? canConnect
              ? isRu
                ? "Сначала подключи настоящую установку Dota 2: без неё сборку некуда ставить."
                : "Connect a real Dota 2 installation first: without it there is nowhere to install the build."
              : isRu
                ? "Сначала подключи настоящую установку Dota 2 в настройках."
                : "Connect a real Dota 2 installation in Settings first."
            : blockerKind === "relation" && relation
              ? relation.kind === "conflict"
                ? isRu
                  ? `«${packageLabel(relation.packageId)}» и «${packageLabel(relation.otherId)}» нельзя ставить вместе.${canFixRelation ? "" : " Оставь в сборке один из них."}`
                  : `${packageLabel(relation.packageId)} and ${packageLabel(relation.otherId)} cannot be installed together.${canFixRelation ? "" : " Keep one of them."}`
                : isRu
                  ? `Для «${packageLabel(relation.packageId)}» нужен мод «${packageLabel(relation.otherId)}»${canFixRelation ? ": без него рецепт не соберётся." : ". Добавь его в сборку."}`
                  : `${packageLabel(relation.packageId)} needs ${packageLabel(relation.otherId)}${canFixRelation ? ": the recipe does not build without it." : ". Add it to the build."}`
              : blockerKind === "pilot"
                ? isRu
                  ? "Ставятся только моды с отметкой «Windows-пилот». Убери остальные игровые моды из сборки."
                  : "Only mods marked Windows pilot install. Remove other game mods from the build."
                : "";
  const steps = [
    {
      done: Boolean(plan) || Boolean(operationId),
      busy: phase === "download",
      label: isRu ? "Скачать и сверить файлы" : "Download and check files",
      hint: plan
        ? isRu
          ? `${plan.resourceCount} файлов · ${formatBytes(plan.vpkBytes, true)}`
          : `${plan.resourceCount} files · ${formatBytes(plan.vpkBytes, false)}`
        : isRu
          ? "Только проверенные файлы из каталога"
          : "Only verified catalog files",
    },
    {
      done: Boolean(operationId),
      busy: phase === "install",
      label: isRu ? "Записать с бэкапом" : "Write with a backup",
      hint:
        operationId && installedLanguage
          ? `dota_${installedLanguage}/pak66_dir.vpk`
          : isRu
            ? "Исходный файл можно вернуть"
            : "The original can be restored",
    },
    {
      done: steamRestarted || steamStarted,
      busy: phase === "steam",
      label: isRu ? "Настроить Steam" : "Set up Steam",
      hint: steamStarted
        ? isRu
          ? "Язык задан в профиле"
          : "Language set in the profile"
        : (installedLanguage ?? selectedLanguage)
          ? `-language ${installedLanguage ?? selectedLanguage}`
          : isRu
            ? "Параметр -language"
            : "The -language option",
    },
    {
      done: false,
      busy: false,
      label: isRu ? "Играть" : "Play",
      hint: isRu ? "Проверь моды в Dota 2" : "Check the mods in Dota 2",
    },
  ];
  const current = steps.findIndex((step) => !step.done);
  const progress = totalResources ? verifiedResources / totalResources : 0;
  const title = operationId
    ? isRu
      ? "Сборка стоит в игре"
      : "The build is in the game"
    : phase === "download"
      ? isRu
        ? "Проверяем файлы"
        : "Checking files"
      : phase === "install"
        ? isRu
          ? "Ставим сборку"
          : "Installing the build"
        : plan
          ? isRu
            ? "Всё сверено. Выбери язык"
            : "All checked. Pick a language"
          : isRu
            ? "Подготовь сборку"
            : "Prepare the build";
  const runNoticeAction = () => {
    if (!notice) return;
    if (notice.action === "retry") void (plan ? install() : prepare());
    else if (notice.action === "prepare") {
      setPlan(null);
      void prepare();
    } else if (notice.action === "report") void refreshEvidence(true);
    else if (notice.action === "release") void releaseStale();
    else if (notice.action === "steam" && installedLanguage) void copyLaunchOption();
    else if (notice.action === "settings") {
      const field = languageFieldRef.current;
      if (notice.code === "language_folder_unavailable")
        (
          field?.querySelector<HTMLElement>('[role="radio"][tabindex="0"]') ??
          field?.querySelector<HTMLElement>("button")
        )?.focus();
      else if (notice.code === "game_path_required") onConnectDota?.();
    }
  };
  // "settings" notices get a button only where this screen can act itself:
  // the language choice is right here, and Dota is connected in one click.
  const settingsActionLabel =
    notice?.code === "language_folder_unavailable" && plan && !operationId
      ? isRu
        ? "Выбрать другой язык"
        : "Choose another language"
      : notice?.code === "game_path_required" && onConnectDota
        ? isRu
          ? "Подключить Dota 2"
          : "Connect Dota 2"
        : null;
  const noticeActionLabel =
    notice?.action === "retry"
      ? isRu
        ? "Повторить"
        : "Retry"
      : notice?.action === "prepare"
        ? isRu
          ? "Подготовить заново"
          : "Prepare again"
        : notice?.action === "release"
          ? isRu
            ? "Отпустить старую установку"
            : "Let go of the old install"
          : notice?.action === "report"
          ? reportCopied
            ? isRu
              ? "Отчёт скопирован"
              : "Report copied"
            : isRu
              ? "Скопировать отчёт"
              : "Copy report"
          : notice?.action === "steam" && installedLanguage
            ? copied
              ? isRu
                ? "Скопировано"
                : "Copied"
              : isRu
                ? "Скопировать -language"
                : "Copy -language"
            : notice?.action === "settings"
              ? settingsActionLabel
              : null;
  // Profiles are numbered, never named: the report and the screen carry no
  // Steam IDs or account names.
  const profileOptions: ChoiceOption<string>[] = [
    {
      value: "manual",
      label: isRu ? "Вручную" : "Manual",
      hint: isRu ? "-language вставишь сам" : "you paste -language",
    },
    ...profiles.map((profile): ChoiceOption<string> => {
      const usable = profile.status === "ready" || profile.status === "already_managed";
      return {
        value: profile.profileToken,
        label: `${isRu ? "Профиль" : "Profile"} ${profile.profileIndex}`,
        hint:
          profile.status === "ready"
            ? isRu
              ? "BetterFy добавит сам"
              : "BetterFy adds it"
            : profile.status === "already_managed"
              ? isRu
                ? "язык уже задан"
                : "language already set"
              : isRu
                ? "конфликт, не трогаем"
                : "conflict, left alone",
        tone: profile.status === "ready" ? "ready" : usable ? undefined : "warn",
        hintIcon: usable ? undefined : <TriangleAlert />,
        disabled: !usable,
      };
    }),
  ];
  const profileChoice = (
    <Choice
      label={isRu ? "Steam-профиль" : "Steam profile"}
      value={selectedProfile || "manual"}
      options={profileOptions}
      onChange={(value) => setSelectedProfile(value === "manual" ? "" : value)}
      placeholder={isRu ? "Выбери профиль" : "Choose a profile"}
      disabled={phase !== "idle"}
    />
  );
  const languageOption = (value: GameLanguage): ChoiceOption<LanguagePick> => {
    const option = buildLanguages.find((item) => item.value === value);
    const experimental = value === "betterfy";
    return {
      value,
      label: languageLabel(value),
      hint: experimental
        ? isRu
          ? "эксперимент"
          : "experimental"
        : option?.verified
          ? isRu
            ? "проверен в игре"
            : "checked in game"
          : isRu
            ? "не проверен"
            : "not checked",
      hintIcon: experimental ? (
        <FlaskConical />
      ) : option?.verified ? (
        <ShieldCheck />
      ) : (
        <CircleDashed />
      ),
      tone: experimental ? "warn" : option?.verified ? "ready" : "quiet",
    };
  };
  const autoOption: ChoiceOption<LanguagePick> | null =
    detection.status === "idle"
      ? null
      : detection.status === "loading"
        ? {
            value: "auto",
            label: isRu ? "Как в Dota сейчас" : "Same as Dota now",
            hint: isRu ? "смотрим, что стоит в Steam…" : "checking Steam…",
            hintIcon: <LoaderCircle className="s-spin" />,
            disabled: true,
            wide: true,
          }
        : detected
          ? {
              ...languageOption(detected),
              value: "auto",
              label: isRu
                ? `Как в Dota сейчас · ${languageLabel(detected)}`
                : `Same as Dota now · ${languageLabel(detected)}`,
              // English is the BetterFy slot and not proven since the patch.
              badge: detected === "betterfy" ? undefined : isRu ? "рекомендуем" : "recommended",
              meta: steamName ? `steam: ${steamName}` : undefined,
              wide: true,
            }
          : {
              value: "auto",
              label: isRu ? "Как в Dota сейчас" : "Same as Dota now",
              hint:
                detection.status === "failed"
                  ? isRu
                    ? "не удалось прочитать Steam"
                    : "could not read Steam"
                  : steamName
                    ? isRu
                      ? "для этого языка у BetterFy нет папки"
                      : "BetterFy has no folder for this language"
                    : isRu
                      ? "Steam не указал язык"
                      : "Steam lists no language",
              hintIcon: <CircleDashed />,
              tone: "quiet",
              meta: steamName ? `steam: ${steamName}` : undefined,
              disabled: true,
              wide: true,
            };
  const languageCards = [
    ...(autoOption ? [autoOption] : []),
    ...PINNED_LANGUAGES.filter((value) => value !== detected).map(languageOption),
  ];
  const otherLanguages = buildLanguages
    .filter((option) => !PINNED_LANGUAGES.includes(option.value))
    .map(
      (option): ChoiceOption<LanguagePick> => ({
        value: option.value,
        label: languageLabel(option.value),
        hint: option.native,
        badge: option.value === detected ? (isRu ? "в Dota сейчас" : "in Dota now") : undefined,
        keywords: `${option.ru} ${option.en} ${option.value}`,
      }),
    );
  // The detected language is shown on the "as in Dota" card wherever it was
  // picked from.
  const shownLanguage: LanguagePick | "" =
    selectedLanguage && selectedLanguage === detected ? "auto" : selectedLanguage;
  const followFallback =
    followDota && (detection.status === "failed" || (detection.status === "done" && !detected));
  const languageNote = (() => {
    if (!selectedLanguage)
      return followDota && detection.status === "loading"
        ? isRu
          ? "Смотрим, на каком языке Steam запускает Dota."
          : "Checking which language Steam starts Dota in."
        : isRu
          ? "Моды подключаются через папку языка: Dota запустится с этим языком интерфейса."
          : "Mods load through a language folder: Dota starts with this interface language.";
    const fallback = followFallback
      ? isRu
        ? `Steam не подсказал язык — выбран: ${languageLabel(selectedLanguage)}. `
        : `Steam did not say, so ${languageLabel(selectedLanguage)} is selected. `
      : "";
    if (selectedLanguage === "betterfy")
      return (
        fallback +
        (isRu
          ? "Текст Dota останется английским. С патча 23.07.2026 Dota может не принять язык, которого нет у Valve, — поэтому Minify ушёл от своей папки. Если язык окажется не тот, откат в один клик."
          : "Dota text stays English. Since the 23 July 2026 patch Dota may refuse a language Valve does not ship, which is why Minify dropped its own folder. If the language is wrong, restore in one click.")
      );
    const option = buildLanguages.find((item) => item.value === selectedLanguage);
    return fallback + (option ? (isRu ? option.noteRu : option.noteEn) : "");
  })();
  return (
    <section
      className={`b-pilot ${busy ? "is-busy" : ""} ${operationId ? "is-installed" : ""}`}
      aria-labelledby="tree-pilot-title"
    >
      <header className="b-pilot-head">
        <small>
          {demo && <b className="b-demo-chip">{isRu ? "ДЕМО" : "DEMO"}</b>}
          {isRu ? "УСТАНОВКА В DOTA 2" : "INSTALL TO DOTA 2"} · {deliveryLabel("pilot", language)}
          {stressEnabled && (
            <b className="b-dev-chip" title={isRu ? "Аккаунт разработчика" : "Developer account"}>
              DEV
            </b>
          )}
        </small>
        <h2 id="tree-pilot-title">{title}</h2>
        <span className="b-pilot-count" title={isRu ? "Моды для установки" : "Mods to install"}>
          {String(operationId ? installedPackageIds.length || ids.length : ids.length).padStart(
            2,
            "0",
          )}
          <i>/{String(installablePackages.length).padStart(2, "0")}</i>
        </span>
      </header>
      {demo && <DemoPanel />}
      <ol className="b-steps">
        {steps.map((step, index) => (
          <li
            key={index}
            className={[
              step.done ? "is-done" : "",
              index === current ? "is-current" : "",
              step.busy ? "is-busy" : "",
            ].join(" ")}
          >
            <i aria-hidden="true">
              {step.done ? <Check /> : step.busy ? <LoaderCircle className="s-spin" /> : index + 1}
            </i>
            <div>
              <b>{step.label}</b>
              {step.busy && activePhrase ? (
                <span className="b-phrase" key={activePhrase.key} aria-live="polite">
                  {activePhrase.text}
                </span>
              ) : (
                <span>{step.hint}</span>
              )}
              {step.busy && phase === "download" && (
                <div
                  className="b-progress"
                  role="progressbar"
                  aria-valuenow={verifiedResources}
                  aria-valuemin={0}
                  aria-valuemax={totalResources || 1}
                  aria-label={isRu ? "Проверенные файлы" : "Verified files"}
                >
                  <div>
                    <i style={{ transform: `scaleX(${progress})` }} />
                  </div>
                  <small>
                    {verifiedResources}/{totalResources || "—"}
                  </small>
                </div>
              )}
            </div>
          </li>
        ))}
      </ol>
      {notice && (
        <NoticeCard
          notice={notice}
          actionLabel={noticeActionLabel}
          onAction={noticeActionLabel ? runNoticeAction : undefined}
          onDismiss={() => setError("")}
          developer={stressEnabled}
        />
      )}
      {blocker && (
        <div className="b-pilot-blocker">
          <TriangleAlert />
          <div>
            <p>{blocker}</p>
            {canFixRelation && (
              <div className="b-fix">
                {relationFixes.map(({ fix, label }) => (
                  <button
                    key={label}
                    className="s-btn"
                    type="button"
                    disabled={busy}
                    onClick={() => onFixRelation?.(fix)}
                  >
                    {fix.kind === "add" ? <Plus /> : <Check />}
                    {label}
                  </button>
                ))}
              </div>
            )}
            {canConnect && (
              <div className="b-fix">
                <button className="s-btn" type="button" disabled={busy} onClick={onConnectDota}>
                  <Gamepad2 />
                  {isRu ? "Подключить Dota 2" : "Connect Dota 2"}
                </button>
              </div>
            )}
          </div>
        </div>
      )}
      {operationId && dotaPatched && (
        <p className="b-pilot-blocker is-warning" role="alert">
          <TriangleAlert />
          {isRu
            ? "Dota 2 обновилась после установки. Если интерфейс сломан, откати сборку — BetterFy вернёт исходные файлы."
            : "Dota 2 updated after this install. If the interface looks broken, restore the build and BetterFy puts the original files back."}
        </p>
      )}
      {unverified.length > 0 && !operationId && !blocker && (
        <p className="b-pilot-hint">
          {isRu
            ? `Ещё не проверено в игре: ${unverified.map(packageLabel).join(", ")}. Если что-то не так — откат в один клик.`
            : `Not checked in game yet: ${unverified.map(packageLabel).join(", ")}. If something is off, restore in one click.`}
        </p>
      )}
      {plan && !operationId && (
        <WardrobeReview
          plan={plan}
          packageLabel={packageLabel}
          acknowledged={!conflictsPending}
          disabled={phase !== "idle"}
          onAcknowledge={(value) => setAcknowledgedPlan(value ? plan.planId : null)}
        />
      )}
      {plan && !operationId && (
        <div className="b-fields">
          <div className="b-field" ref={languageFieldRef}>
            <div className="b-field-head">
              <span id="tree-pilot-language">{isRu ? "Язык Dota" : "Dota language"}</span>
              {selectedLanguage && (
                <code title={`game/dota_${selectedLanguage}/${plan.targetFile}`}>
                  -language {selectedLanguage}
                </code>
              )}
            </div>
            <ChoiceCards
              label={isRu ? "Язык Dota" : "Dota language"}
              labelledBy="tree-pilot-language"
              value={
                languageCards.some((option) => option.value === shownLanguage) ? shownLanguage : ""
              }
              options={languageCards}
              onChange={chooseLanguage}
              disabled={phase !== "idle"}
            />
            <ChoiceMenu
              label={isRu ? "Другие языки Dota" : "Other Dota languages"}
              value={
                otherLanguages.some((option) => option.value === shownLanguage) ? shownLanguage : ""
              }
              options={otherLanguages}
              onChange={chooseLanguage}
              placeholder={isRu ? "Другие языки" : "Other languages"}
              placeholderHint={
                isRu
                  ? `остальные языки Dota из Steam · ${otherLanguages.length}`
                  : `the rest of Dota's Steam languages · ${otherLanguages.length}`
              }
              caption={
                isRu ? "Ещё не проверены в игре на Windows" : "Not checked in game on Windows yet"
              }
              searchPlaceholder={
                isRu ? "Найти: français, Deutsch, 日本語…" : "Find: français, Deutsch, 日本語…"
              }
              emptyText={isRu ? "Такого языка в Dota нет." : "Dota has no such language."}
              disabled={phase !== "idle"}
            />
            <small aria-live="polite">{languageNote}</small>
          </div>
          {canInstall && (
            <div className="b-field">
              <div className="b-field-head">
                <span>{isRu ? "Профиль Steam" : "Steam profile"}</span>
              </div>
              {profileChoice}
              <small>
                {selectedProfile
                  ? isRu
                    ? "BetterFy сам добавит -language и вернёт как было при откате."
                    : "BetterFy adds -language itself and puts it back on restore."
                  : isRu
                    ? "Параметр запуска нужно будет вставить вручную."
                    : "You will paste the launch option yourself."}
              </small>
            </div>
          )}
        </div>
      )}
      {operationId ? (
        <div className="b-installed">
          {installedLanguage && (
            <div className="b-command">
              <span>
                {isRu ? "Параметр запуска" : "Launch option"} · {languageLabel(installedLanguage)}
              </span>
              <code>-language {installedLanguage}</code>
              <button className="s-btn" type="button" onClick={() => void copyLaunchOption()}>
                {copied ? <Check /> : <Copy />}
                {copied ? (isRu ? "Скопировано" : "Copied") : isRu ? "Копировать" : "Copy"}
              </button>
            </div>
          )}
          <p className="b-pilot-hint">
            {!verifiedInstall
              ? isRu
                ? "Найдена сохранённая операция, но файл ещё не подтверждён. Проверь установку или откати."
                : "A saved operation was found, but the file is not verified yet. Check or restore it."
              : !packageVerified
                ? isRu
                  ? "Файл совпадает с журналом BetterFy, но исходные ресурсы недоступны для повторной проверки. Откат доступен."
                  : "The file matches BetterFy's journal, but source resources cannot be re-checked. Restore is available."
                : steamStarted
                  ? isRu
                    ? "Steam открыт и знает нужный язык. Запускай Dota 2 и проверь каждый мод."
                    : "Steam is open and knows the language. Launch Dota 2 and check every mod."
                  : steamRestarted
                    ? isRu
                      ? "Steam открыт. Вставь параметр в свойства Dota 2 → Параметры запуска."
                      : "Steam is open. Paste the option into Dota 2 properties → Launch options."
                    : isRu
                      ? "Файл записан, но Steam не настроен до конца. Выбери профиль и повтори."
                      : "The file is written, but Steam setup did not finish. Pick a profile and retry."}
          </p>
          {installedLanguage === "betterfy" && (
            <p className="b-pilot-blocker is-warning">
              <FlaskConical />
              {isRu
                ? "Английский — эксперимент. Если Dota запустилась не на английском, откати сборку: BetterFy вернёт файл и параметры Steam."
                : "English is experimental. If Dota starts in another language, restore the build: BetterFy puts back the file and the Steam options."}
            </p>
          )}
          {canManage && !steamStarted && packageVerified && (
            <div className="b-steam-retry">
              {profileChoice}
              <button
                className="s-btn"
                disabled={phase !== "idle" || !selectedProfile}
                onClick={() => void activateSteam()}
              >
                {phase === "steam" ? <LoaderCircle className="s-spin" /> : <ArrowRight />}
                {isRu ? "Настроить Steam" : "Set up Steam"}
              </button>
            </div>
          )}
          <button
            className="s-btn b-restore"
            disabled={phase !== "idle" || !canManage || steamRecoveryRequired}
            onClick={() => void restore()}
          >
            <RotateCcw className={`b-rewind ${phase === "restore" ? "is-spinning" : ""}`} />
            {phase === "restore"
              ? isRu
                ? "Откатываем…"
                : "Restoring…"
              : isRu
                ? "Откатить сборку"
                : "Restore the game"}
          </button>
        </div>
      ) : (
        <div className="b-actions">
          {!plan ? (
            <>
              <button
                className="s-btn s-btn-primary b-primary"
                disabled={
                  phase !== "idle" || !desktop || !supportedBundle || Boolean(blocker && !demo)
                }
                onClick={() => void prepare()}
              >
                {phase === "download" ? <LoaderCircle className="s-spin" /> : <Download />}
                {phase === "download"
                  ? isRu
                    ? "Проверяем…"
                    : "Checking…"
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
            <button
              className="s-btn s-btn-primary b-primary"
              disabled={
                !canInstall ||
                !selectedLanguage ||
                phase !== "idle" ||
                !plan.deployEnabled ||
                conflictsPending
              }
              onClick={() => {
                // The language that went in becomes the fallback next time.
                if (selectedLanguage) setStorageItem(LANGUAGE_KEY, selectedLanguage);
                void install();
              }}
            >
              {phase === "install" || phase === "steam" ? (
                <LoaderCircle className="s-spin" />
              ) : (
                <ShieldCheck />
              )}
              {phase === "install" || phase === "steam"
                ? isRu
                  ? "Ставим…"
                  : "Installing…"
                : !selectedLanguage
                  ? followDota && detection.status === "loading"
                    ? isRu
                      ? "Смотрим язык Dota…"
                      : "Checking Dota's language…"
                    : isRu
                      ? "Сначала выбери язык"
                      : "Pick a language first"
                  : isRu
                    ? "Поставить в Dota 2"
                    : "Install to Dota 2"}
              <ArrowRight className="s-arrow-forward" />
            </button>
          )}
        </div>
      )}
      {!operationId && steamOperationId && canManage && (
        <button className="s-btn" disabled={phase !== "idle"} onClick={() => void restoreSteam()}>
          <RotateCcw className="b-rewind" />
          {isRu ? "Восстановить параметры Steam" : "Restore Steam settings"}
        </button>
      )}
      {steamRecoveryRequired && (
        <p className="b-pilot-blocker" role="alert">
          <TriangleAlert />
          {isRu
            ? "Изменение Steam было прервано. Сначала восстанови Steam, затем откатывай файл игры."
            : "A Steam change was interrupted. Recover Steam before restoring the game file."}
        </p>
      )}
      {recoveryMessage && (
        <p className="b-pilot-hint" role="status">
          {recoveryMessage}
        </p>
      )}
      <details className="b-more">
        <summary>{isRu ? "Подробности" : "Details"}</summary>
        {plan && !operationId && (
          <>
            <div className="b-more-block">
              <strong>{isRu ? "Порядок модов" : "Mod order"}</strong>
              <p>
                {isRu
                  ? "Если два мода меняют один файл, в сборке остаётся верхний."
                  : "If two mods change the same file, the higher one stays in the build."}
              </p>
              <ol className="b-order">
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
            <dl className="b-facts">
              <div>
                <dt>{isRu ? "Файл" : "File"}</dt>
                <dd>
                  <code>
                    {selectedLanguage
                      ? `game/dota_${selectedLanguage}/${plan.targetFile}`
                      : plan.targetFile}
                  </code>
                </dd>
              </div>
              <div>
                <dt>{isRu ? "Состав" : "Bundle"}</dt>
                <dd>
                  {plan.packageCount} · {plan.resourceCount} {isRu ? "файлов" : "files"}
                </dd>
              </div>
              <div>
                <dt>{isRu ? "Совпадения" : "Overlaps"}</dt>
                <dd>
                  {plan.duplicateResources + plan.overriddenResources || (isRu ? "нет" : "none")}
                </dd>
              </div>
              <div>
                <dt>{isRu ? "Размер" : "Size"}</dt>
                <dd>{formatBytes(plan.vpkBytes, isRu)}</dd>
              </div>
              <div>
                <dt>SHA-256</dt>
                <dd>
                  <code>{plan.vpkSha256.slice(0, 16)}…</code>
                </dd>
              </div>
            </dl>
            {(plan.duplicates.length > 0 || plan.overrides.length > 0) && (
              <div className="b-more-block b-overlaps">
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
                      {isRu ? "Одинаковый файл сохранён один раз" : "Identical file stored once"}:{" "}
                      {packageLabel(item.keptPackageId)}
                    </span>
                  </p>
                ))}
              </div>
            )}
          </>
        )}
        {operationId && installedPackageIds.length > 0 && (
          <div className="b-more-block">
            <strong>{isRu ? "Что стоит в игре" : "What is installed"}</strong>
            <p>{installedPackageIds.map(packageLabel).join(" · ")}</p>
          </div>
        )}
        {canManage && (
          <div className="b-more-block b-more-row">
            <div>
              <strong>{isRu ? "Прерванная установка" : "Interrupted install"}</strong>
              <p>
                {isRu
                  ? "Если BetterFy закрыли посреди записи, здесь можно довести откат до конца."
                  : "If BetterFy was closed mid-write, finish the restore from here."}
              </p>
            </div>
            <button
              className="s-btn"
              disabled={phase !== "idle" || recovering}
              onClick={() => void recover()}
            >
              <RotateCcw className={`b-rewind ${recovering ? "is-spinning" : ""}`} />
              {steamRecoveryRequired
                ? isRu
                  ? "Восстановить Steam и проверить"
                  : "Recover Steam and check"
                : isRu
                  ? "Проверить"
                  : "Check"}
            </button>
          </div>
        )}
        {canManage && (
          <div className="b-more-block b-more-row">
            <div>
              <strong>
                <FileCheck2 />
                {isRu ? "Отчёт Windows-теста" : "Windows test report"}{" "}
                <span className="b-count">
                  {(evidence?.entries.length ?? 0) + (evidence?.steamEntries.length ?? 0)}
                </span>
              </strong>
              <p>
                {savedReport
                  ? isRu
                    ? `Сохранён: ${savedReport}`
                    : `Saved: ${savedReport}`
                  : isRu
                    ? "Только версии, этапы, языки, пакеты и SHA-256. Без путей, Steam ID и данных аккаунта."
                    : "Only versions, phases, languages, packages and SHA-256. No paths, Steam IDs or account data."}
                {reportError &&
                  (isRu
                    ? ` Не удалось сохранить (${reportError}).`
                    : ` Could not save (${reportError}).`)}
              </p>
            </div>
            <span className="b-more-buttons">
              <button className="s-btn" type="button" onClick={() => void refreshEvidence(true)}>
                {reportCopied ? <Check /> : <Copy />}
                {reportCopied ? (isRu ? "Скопировано" : "Copied") : isRu ? "Копировать" : "Copy"}
              </button>
              <button className="s-btn" type="button" onClick={() => void openReportsFolder()}>
                {isRu ? "Папка" : "Folder"}
              </button>
            </span>
          </div>
        )}
        {stressEnabled && (
          <div className="b-more-block b-dev">
            <strong>
              <FlaskConical />
              {isRu ? "Режим разработчика · стресс-тесты" : "Developer mode · stress tests"}
            </strong>
            <p>
              {isRu
                ? "BetterFy намеренно обрывает запись в безопасной точке и сразу проверяет восстановление."
                : "BetterFy deliberately interrupts the write at a safe point and verifies recovery right away."}
            </p>
            {plan && !operationId ? (
              <div className="b-more-buttons">
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
            ) : operationId &&
              installedLanguage &&
              selectedProfile &&
              packageVerified &&
              !steamOperationId ? (
              <div className="b-more-buttons">
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
            ) : (
              <small>
                {isRu
                  ? "Тесты записи доступны после подготовки и выбора языка, тесты Steam — после установки без настроенного Steam."
                  : "Write tests unlock after preparing and picking a language; Steam tests after an install without Steam set up."}
              </small>
            )}
            {stressMessage && (
              <p className="b-pass" role="status">
                {stressMessage}
              </p>
            )}
          </div>
        )}
        <small className="b-source">
          {isRu
            ? "Источник: Egezenn/dota2-minify. Tree Mod, Show NetWorth, Unit Query HUD и Remove River проверены в игре на одном Windows-компьютере с нидерландским и русским языком; другие языки и компьютеры не проверены. Параметр -language меняет язык текста и может повлиять на озвучку."
            : "Source: Egezenn/dota2-minify. Tree Mod, Show NetWorth, Unit Query HUD and Remove River were verified in game on one Windows computer with Dutch and Russian; other languages and computers are untested. The -language option changes text language and may affect audio."}
        </small>
      </details>
    </section>
  );
}
