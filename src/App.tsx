import { lazy, Suspense, useEffect, useLayoutEffect, useRef, useState } from "react";
import { flushSync } from "react-dom";
import {
  ArchiveRestore,
  ArrowRight,
  Bell,
  BookOpen,
  Boxes,
  Check,
  CircleCheckBig,
  CircleUserRound,
  ExternalLink,
  FolderOpen,
  Gamepad2,
  Heart,
  Home,
  Languages,
  Layers3,
  Library,
  LoaderCircle,
  LogOut,
  MessageCircle,
  Monitor,
  PackageOpen,
  Play,
  Power,
  RotateCcw,
  Save,
  Search,
  Settings,
  ShieldCheck,
  SlidersHorizontal,
  Sparkles,
  Stethoscope,
  Sun,
  Moon,
  TriangleAlert,
  Users,
  X,
} from "lucide-react";
import AuthFlow from "./AuthFlow";
import AppUpdater from "./AppUpdater";
import AccentTitle from "./AccentTitle";
import BetterFyMark from "./BetterFyMark";
import BetterFyWordmark from "./BetterFyWordmark";
import baneHome from "./assets/bane-home.png";
import courierSuccess from "./assets/courier-success.png";
import enigmaProgress from "./assets/enigma-progress.webp";
import pudgeRecovery from "./assets/pudge-recovery.png";
import wukongBuild from "./assets/wukong-build.png";
import type { AuthSession } from "./auth";
import {
  EngineFault,
  engineBridge,
  type BuildPlan,
  type BuildReceipt,
  type GameInstallation,
  type SteamConfigReceipt,
  type SteamLaunchOptionPreview,
  type SteamProfileSummary,
} from "./engine";
import { useLocale, type Language } from "./i18n";
import OnboardingFlow from "./OnboardingFlow";
import PresetManager from "./PresetManager";
import { getStorageItem, getStoredStringArray, setStorageItem } from "./storage";

type AppStage = "loading" | "auth" | "setup" | "workspace";
type SidePanel = "settings" | "profile" | null;
type WorkspaceRoute = "home" | "discover" | "build" | "library";
type BuildView = "review" | "conflict" | "ready" | "progress" | "success" | "recovery" | "restored";
type ThemeMode = "dark" | "light";

const installationStorageKey = "betterfy:game-installation";
const ModCatalogRoute = lazy(() => import("./ModCatalogRoute"));

type ViewTransitionDocument = Document & {
  startViewTransition?: (update: () => void) => { finished: Promise<void> };
};

function transitionUI(update: () => void) {
  const reduced = window.matchMedia("(prefers-reduced-motion: reduce)").matches
    || document.documentElement.classList.contains("motion-disabled");
  const startViewTransition = (document as ViewTransitionDocument).startViewTransition;
  if (!startViewTransition || reduced) {
    update();
    return;
  }
  startViewTransition.call(document, () => flushSync(update)).finished.catch(() => undefined);
}

const copy = {
  ru: {
    loading: {
      label: "BETTERFY / DESKTOP",
      title: "Готовим твою Dota",
      subtitle: "Запускаем оболочку. К файлам Dota 2 обратимся только после твоего разрешения.",
      steps: ["Интерфейс", "Локальный профиль", "Каталог"],
      footnote: "Запуск без доступа к файлам Dota 2",
    },
    nav: {
      home: "Главная",
      discover: "Каталог",
      build: "Сборка",
      library: "Библиотека",
      settings: "Настройки",
      profile: "Профиль",
    },
    home: {
      eyebrow: "ПРОСТРАНСТВО ГОТОВО",
      title: "Твоя Dota.\nТолько лучше.",
      subtitle: "Игра подключена. Следующий шаг — собрать первый набор модов и образов.",
      action: "Открыть сборку",
      actionHint: "Текущий профиль сохранён локально",
      artLabel: "BANE / HOME",
      artTitle: "Bane",
      artHint: "Тёмный якорь пространства BetterFy",
      buildEyebrow: "АКТИВНАЯ СБОРКА",
      buildTitle: "Первая сборка ждёт контент",
      buildText: "Выбранные моды появятся здесь, когда мы откроем каталог отдельным этапом.",
      browse: "Открыть каталог",
      empty: "Пока пусто",
      state: "DOTA 2",
      connected: "Подключена",
      demoConnected: "Демо-путь",
      build: "СБОРКА",
      buildValue: "Не создана",
      access: "ДОСТУП",
      accessValue: "Telegram",
      preview: "Prototype shell · игровые файлы не изменяются",
    },
    catalog: {
      prototype: "Каталог · выбор сохраняется локально, загрузка и патчинг пока не выполняются",
    },
    build: {
      eyebrow: "BUILD / 01",
      title: "Собери свою версию Dota",
      text: "Проверяем состав, зависимости и конфликты до любых изменений в игре.",
      emptyTitle: "Тестовая сборка готова к проверке",
      emptyText: "Два локальных fixture-мода позволяют пройти весь сценарий без записи в папку Dota 2.",
      plan: "ПЛАН СБОРКИ",
      items: "Контент",
      dependencies: "Зависимости",
      conflicts: "Конфликты",
      unavailable: "Ожидает каталога",
      back: "Вернуться на главную",
      inspect: "Проверить тестовый план",
      planning: "Строим dry-run план…",
      conflictTitle: "Найден конфликт fixture-модов",
      conflictText: "Два тестовых мода меняют один ресурс. Engine остановил план до выбора победителя — игровые файлы не затронуты.",
      conflict: "ТРЕБУЕТ РЕШЕНИЯ",
      inspectAgain: "Перестроить план",
      keepViolet: "Оставить Violet",
      keepClean: "Оставить Clean",
      variantViolet: "Ambient Violet",
      variantClean: "Ambient Clean",
      replaceHint: "Будет выбран только один вариант ресурса",
      readyLabel: "DRY RUN / READY",
      readyTitle: "Решения приняты",
      readyText: "План согласован. Дальше BetterFy покажет полный цикл подготовки в безопасном preview-режиме.",
      start: "Запустить preview-сборку",
      recoveryPreview: "Проверить сценарий Recovery",
      progressEyebrow: "BUILD / IN PROGRESS",
      progressTitle: "Сборка обретает форму",
      progressText: "Показываем будущий engine-процесс. Сейчас работает только интерфейсная симуляция без записи файлов.",
      phases: ["Проверка плана", "Разрешение ресурсов", "Сборка staging", "Проверка результата"],
      progressNote: "Игровая папка не изменяется",
      interrupt: "Смоделировать ошибку",
      successEyebrow: "BUILD / READY",
      successTitle: "Можно отправляться в игру",
      successText: "Preview-сборка прошла проверку. В production BetterFy запустит только Steam — Dota 2 ты откроешь сам.",
      play: "Открыть Steam",
      playUnavailable: "Steam запустится автоматически после безопасного патчинга",
      activationTitle: "Активировать Steam-профиль",
      activationText: "BetterFy добавит только свой launch option в выбранный локальный профиль, проверит запись и запустит Steam. Dota 2 остаётся закрытой.",
      activationDesktopOnly: "Активация доступна только в Windows desktop-сборке с проверенной Dota 2.",
      activationLoading: "Ищем локальные Steam-профили…",
      activationEmpty: "Локальные Steam-профили не найдены. Один раз войди в Steam на этом компьютере и повтори.",
      activationProfile: "Профиль Steam",
      activationReady: "Готов к активации",
      activationManaged: "Уже настроен BetterFy",
      activationConflict: "Конфликт launch options",
      activationInvalid: "Конфиг повреждён",
      activationConfirm: "Я понимаю: BetterFy закроет Dota 2 и Steam, изменит launch options этого профиля и снова запустит только Steam.",
      activationAction: "Активировать и открыть Steam",
      activationStopping: "Закрываем Dota 2 и Steam…",
      activationApplying: "Создаём backup и применяем настройку…",
      activationStarting: "Проверяем результат и запускаем Steam…",
      activationComplete: "Steam запущен. Теперь открой Dota 2 самостоятельно.",
      activationRetry: "Повторить активацию",
      activationError: "Активация остановлена безопасно. Проверь Steam-профиль и повтори.",
      activationRecovery: "Откатить настройку BetterFy",
      configTitle: "Сохрани эту конфигурацию",
      configText: "Вернись к ней позже или передай пресет другому игроку через менеджер конфигов.",
      openConfigs: "Открыть менеджер конфигов",
      selectedContent: "Выбрано в каталоге",
      resolvedState: "План разрешён",
      finishPreview: "Завершить preview",
      recoveryEyebrow: "RECOVERY / RESTORE",
      recoveryTitle: "Вернём всё в спокойное состояние",
      recoveryText: "Этот экран показывает будущий путь восстановления. В прототипе очищается только временное состояние интерфейса.",
      activationRecoveryText: "BetterFy закроет Steam, сверит журнал и восстановит точные байты launch options из проверенного backup. Steam после отката нужно открыть вручную.",
      restore: "Восстановить staging",
      restoreActivation: "Откатить профиль и staging",
      restoring: "Восстанавливаем…",
      restoredLabel: "RECOVERY / COMPLETE",
      restoredTitle: "Временное состояние очищено",
      restoredText: "Никакие игровые файлы не изменялись. Можно вернуться к плану и повторить проверку.",
      rebuild: "Вернуться к плану",
      recoverySafe: "В production восстановление будет подтверждаться журналом операции",
      prototype: "Build staging работает локально · Steam launch options активируются только после подтверждения · файлы Dota 2 пока не изменяются",
    },
    library: {
      eyebrow: "LIBRARY / LOCAL",
      title: "Твоя библиотека",
      text: "Конфиги, локальные импорты и проверенные подборки живут в одном месте.",
      imports: "Локальные импорты",
      importsText: "Добавление личных файлов появится вместе с безопасной проверкой архивов.",
      saved: "Сохранённые сборки",
      savedText: "После первой сборки здесь появятся версии, которые можно восстановить или повторить.",
      empty: "Пока нет материалов",
      prototype: "Конфиги сохраняются локально · импорт мод-архивов пока отключён",
    },
    community: {
      label: "BETTERFY COMMUNITY",
      title: "Делаем BetterFy вместе",
      text: "Новости раннего доступа, обратная связь и помощь с установкой — напрямую через нашего бота.",
      action: "Открыть @BetterFyBot",
      note: "Без выдуманных рейтингов — только живой канал команды и игроков.",
      libraryTitle: "Community builds",
      libraryText: "Здесь появятся проверенные авторские подборки сообщества. Источник и автор будут указаны у каждой сборки.",
      soon: "ГОТОВИМ",
    },
    panel: {
      close: "Закрыть",
      settings: "Настройки",
      settingsText: "Интерфейс, подключение Dota 2 и локальные параметры приложения.",
      interface: "Интерфейс",
      game: "Подключение игры",
      language: "Язык приложения",
      theme: "Тема интерфейса",
      themeText: "Светлая слоновая кость или глубокий тёмный режим.",
      light: "Светлая",
      dark: "Тёмная",
      motion: "Анимации интерфейса",
      motionText: "Переходы, отклики и атмосферное движение.",
      startup: "Запускать вместе с Windows",
      startupText: "Сохраняем предпочтение. Регистрация в ОС появится в desktop-релизе.",
      gamePath: "Путь к Dota 2",
      verified: "Проверен локально",
      demo: "Демонстрационный путь",
      reconnect: "Проверить другую установку",
      diagnostics: "Диагностика подключения",
      diagnosticsText: "Повторно проверить путь и обязательные файлы.",
      runDiagnostics: "Запустить проверку",
      checking: "Проверяем…",
      diagnosticReady: "Путь подтверждён",
      diagnosticDemo: "Демо-проверка завершена",
      diagnosticError: "Путь требует внимания",
      profile: "Профиль",
      profileText: "Демонстрационный локальный профиль раннего доступа.",
      signedIn: "Вход через Telegram",
      plan: "Founding Tester",
      community: "Комьюнити и поддержка",
      communityText: "Новости, ранние сборки и связь с командой.",
      openBot: "Открыть @BetterFyBot",
      signout: "Выйти из профиля",
    },
  },
  en: {
    loading: {
      label: "BETTERFY / DESKTOP",
      title: "Preparing your Dota",
      subtitle: "Starting the shell. Dota 2 files stay untouched until you allow access.",
      steps: ["Interface", "Local profile", "Catalog"],
      footnote: "Starting without Dota 2 file access",
    },
    nav: {
      home: "Home",
      discover: "Discover",
      build: "Build",
      library: "Library",
      settings: "Settings",
      profile: "Profile",
    },
    home: {
      eyebrow: "SPACE READY",
      title: "Your Dota.\nOnly better.",
      subtitle: "The game is connected. Next, compose your first set of mods and cosmetics.",
      action: "Open build",
      actionHint: "Your current profile is stored locally",
      artLabel: "BANE / HOME",
      artTitle: "Bane",
      artHint: "A dark anchor for the BetterFy space",
      buildEyebrow: "ACTIVE BUILD",
      buildTitle: "Your first build needs content",
      buildText: "Selected mods will appear here when the catalog opens as a separate stage.",
      browse: "Open catalog",
      empty: "Nothing selected",
      state: "DOTA 2",
      connected: "Connected",
      demoConnected: "Demo path",
      build: "BUILD",
      buildValue: "Not created",
      access: "ACCESS",
      accessValue: "Telegram",
      preview: "Prototype shell · no game files are changed",
    },
    catalog: {
      prototype: "Catalog · selection is stored locally; download and patching are not active yet",
    },
    build: {
      eyebrow: "BUILD / 01",
      title: "Compose your own Dota",
      text: "Review content, dependencies, and conflicts before anything reaches the game.",
      emptyTitle: "The fixture build is ready to inspect",
      emptyText: "Two local fixture mods let you test the complete flow without writing to Dota 2.",
      plan: "BUILD PLAN",
      items: "Content",
      dependencies: "Dependencies",
      conflicts: "Conflicts",
      unavailable: "Waiting for catalog",
      back: "Return Home",
      inspect: "Inspect fixture plan",
      planning: "Building dry-run plan…",
      conflictTitle: "Fixture conflict detected",
      conflictText: "Two fixture mods target the same resource. The engine stopped before execution — no game files were touched.",
      conflict: "NEEDS A DECISION",
      inspectAgain: "Rebuild plan",
      keepViolet: "Keep Violet",
      keepClean: "Keep Clean",
      variantViolet: "Ambient Violet",
      variantClean: "Ambient Clean",
      replaceHint: "Only one resource variant will remain",
      readyLabel: "DRY RUN / READY",
      readyTitle: "Every decision is resolved",
      readyText: "The plan is coherent. BetterFy can now demonstrate the complete preparation cycle in safe preview mode.",
      start: "Start preview build",
      recoveryPreview: "Preview Recovery flow",
      progressEyebrow: "BUILD / IN PROGRESS",
      progressTitle: "Your build is taking shape",
      progressText: "This demonstrates the future engine sequence. It is a UI simulation and does not write files.",
      phases: ["Checking plan", "Resolving resources", "Building staging", "Verifying result"],
      progressNote: "The game directory remains untouched",
      interrupt: "Simulate an error",
      successEyebrow: "BUILD / READY",
      successTitle: "Ready for the match",
      successText: "The preview build passed verification. Production will start Steam only — you launch Dota 2 yourself.",
      play: "Open Steam",
      playUnavailable: "Steam will start automatically after a safe patch",
      activationTitle: "Activate a Steam profile",
      activationText: "BetterFy adds only its owned launch option to the selected local profile, verifies the write, and starts Steam. Dota 2 stays closed.",
      activationDesktopOnly: "Activation is available only in the Windows desktop build with a verified Dota 2 installation.",
      activationLoading: "Looking for local Steam profiles…",
      activationEmpty: "No local Steam profiles were found. Sign in to Steam once on this PC and try again.",
      activationProfile: "Steam profile",
      activationReady: "Ready to activate",
      activationManaged: "Already managed by BetterFy",
      activationConflict: "Launch options conflict",
      activationInvalid: "Invalid config",
      activationConfirm: "I understand: BetterFy will close Dota 2 and Steam, change this profile's launch options, and start Steam only.",
      activationAction: "Activate and open Steam",
      activationStopping: "Closing Dota 2 and Steam…",
      activationApplying: "Creating a backup and applying the setting…",
      activationStarting: "Verifying the result and starting Steam…",
      activationComplete: "Steam is running. Launch Dota 2 yourself when ready.",
      activationRetry: "Retry activation",
      activationError: "Activation stopped safely. Check the Steam profile and try again.",
      activationRecovery: "Roll back the BetterFy setting",
      configTitle: "Save this configuration",
      configText: "Return to it later or share the preset through the config manager.",
      openConfigs: "Open config manager",
      selectedContent: "Selected in Discover",
      resolvedState: "Plan resolved",
      finishPreview: "Finish preview",
      recoveryEyebrow: "RECOVERY / RESTORE",
      recoveryTitle: "Return to a calm state",
      recoveryText: "This demonstrates the future recovery path. The prototype only clears temporary interface state.",
      activationRecoveryText: "BetterFy will close Steam, verify the journal, and restore the exact launch-option bytes from the checked backup. Start Steam manually after rollback.",
      restore: "Restore staging",
      restoreActivation: "Roll back profile and staging",
      restoring: "Restoring…",
      restoredLabel: "RECOVERY / COMPLETE",
      restoredTitle: "Temporary state cleared",
      restoredText: "No game files were changed. Return to the plan and run the preview again.",
      rebuild: "Return to plan",
      recoverySafe: "Production recovery will be verified against the operation journal",
      prototype: "Build staging runs locally · Steam launch options activate only after confirmation · Dota 2 files are still untouched",
    },
    library: {
      eyebrow: "LIBRARY / LOCAL",
      title: "Your library",
      text: "Configs, local imports, and curated compositions live in one place.",
      imports: "Local imports",
      importsText: "Personal files arrive with safe archive validation.",
      saved: "Saved builds",
      savedText: "After your first build, repeatable and recoverable versions will appear here.",
      empty: "No materials yet",
      prototype: "Configs are stored locally · mod archive import is still disabled",
    },
    community: {
      label: "BETTERFY COMMUNITY",
      title: "Build BetterFy with us",
      text: "Early Access news, feedback, and setup help — directly through our bot.",
      action: "Open @BetterFyBot",
      note: "No fabricated ratings — just a direct channel between the team and players.",
      libraryTitle: "Community builds",
      libraryText: "Verified community collections will appear here. Every build will retain its source and author.",
      soon: "IN PROGRESS",
    },
    panel: {
      close: "Close",
      settings: "Settings",
      settingsText: "Interface, Dota 2 connection, and local application preferences.",
      interface: "Interface",
      game: "Game connection",
      language: "Application language",
      theme: "Interface theme",
      themeText: "Soft ivory light mode or the deep dark space.",
      light: "Light",
      dark: "Dark",
      motion: "Interface motion",
      motionText: "Transitions, feedback, and atmospheric movement.",
      startup: "Launch with Windows",
      startupText: "The preference is saved. OS registration arrives in the desktop release.",
      gamePath: "Dota 2 path",
      verified: "Verified locally",
      demo: "Demonstration path",
      reconnect: "Check another installation",
      diagnostics: "Connection diagnostics",
      diagnosticsText: "Check the path and required marker files again.",
      runDiagnostics: "Run diagnostics",
      checking: "Checking…",
      diagnosticReady: "Path verified",
      diagnosticDemo: "Demo check completed",
      diagnosticError: "Path needs attention",
      profile: "Profile",
      profileText: "A local Early Access demonstration profile.",
      signedIn: "Signed in with Telegram",
      plan: "Founding Tester",
      community: "Community and support",
      communityText: "News, early builds, and direct contact with the team.",
      openBot: "Open @BetterFyBot",
      signout: "Sign out",
    },
  },
} satisfies Record<Language, any>;

function readStoredInstallation() {
  try {
    const raw = getStorageItem(installationStorageKey);
    return raw ? JSON.parse(raw) as GameInstallation : null;
  } catch {
    return null;
  }
}

export default function App() {
  const [stage, setStage] = useState<AppStage>("loading");
  const [installation, setInstallation] = useState<GameInstallation | null>(null);
  const [session, setSession] = useState<AuthSession | null>(null);
  const [theme, setTheme] = useState<ThemeMode>(() =>
    getStorageItem("betterfy:theme") === "light" ? "light" : "dark");

  useLayoutEffect(() => {
    document.documentElement.dataset.theme = theme;
    document.documentElement.style.colorScheme = theme;
    setStorageItem("betterfy:theme", theme);
  }, [theme]);

  useEffect(() => {
    if (stage !== "loading") return;
    const timer = window.setTimeout(() => transitionUI(() => setStage("auth")), 2350);
    return () => window.clearTimeout(timer);
  }, [stage]);

  const completeAuth = (nextSession: AuthSession) => {
    const stored = readStoredInstallation();
    setSession(nextSession);
    setInstallation(stored);
    transitionUI(() => setStage(stored ? "workspace" : "setup"));
  };

  const completeSetup = (game: GameInstallation) => {
    setInstallation(game);
    setStorageItem(installationStorageKey, JSON.stringify(game));
    transitionUI(() => setStage("workspace"));
  };

  let content;
  if (stage === "loading") content = <LoadingScreen />;
  else if (stage === "auth") content = <AuthFlow onComplete={completeAuth} />;
  else if (stage === "setup" || !installation) content = <OnboardingFlow onComplete={completeSetup} />;
  else {
    content = (
      <Workspace
        installation={installation}
        session={session}
        theme={theme}
        onThemeChange={setTheme}
        onReconnect={() => transitionUI(() => setStage("setup"))}
        onSignOut={() => {
          setSession(null);
          transitionUI(() => setStage("auth"));
        }}
      />
    );
  }
  return <>{content}<AppUpdater /></>;
}

function LoadingScreen() {
  const { language } = useLocale();
  const t = copy[language].loading;

  return (
    <main className="launch-screen" aria-label={t.title}>
      <div className="launch-atmosphere" aria-hidden="true" />
      <div className="launch-topline"><span>{t.label}</span><span>LOCAL / START</span></div>
      <div className="launch-content">
        <BetterFyWordmark animated hero />
        <p>{t.label}</p>
        <div className="launch-sequence" role="status" aria-live="polite">
          <div className="launch-sequence-track" aria-hidden="true">
            <i />
            <b />
            <b />
            <b />
          </div>
          <ol>
            {t.steps.map((step: string, index: number) => (
              <li key={step}><strong>{String(index + 1).padStart(2, "0")}</strong><span>{step}</span></li>
            ))}
          </ol>
        </div>
      </div>
      <p className="launch-footnote">{t.footnote}</p>
    </main>
  );
}

function Workspace({
  installation,
  session,
  theme,
  onThemeChange,
  onReconnect,
  onSignOut,
}: {
  installation: GameInstallation;
  session: AuthSession | null;
  theme: ThemeMode;
  onThemeChange: (theme: ThemeMode) => void;
  onReconnect: () => void;
  onSignOut: () => void;
}) {
  const { language, setLanguage } = useLocale();
  const t = copy[language];
  const [panel, setPanel] = useState<SidePanel>(null);
  const [panelClosing, setPanelClosing] = useState(false);
  const panelCloseTimer = useRef<number | null>(null);
  const [route, setRoute] = useState<WorkspaceRoute>("home");
  const [catalogCompact, setCatalogCompact] = useState(false);
  const [motion, setMotion] = useState(() => getStorageItem("betterfy:motion") !== "off");
  const [startup, setStartup] = useState(() => getStorageItem("betterfy:startup-preference") === "on");
  const [diagnostic, setDiagnostic] = useState<"idle" | "checking" | "ready" | "demo" | "error">("idle");
  const [selectedModIds, setSelectedModIds] = useState<string[]>(() =>
    getStoredStringArray("betterfy:selected-mods"));

  const clearPanelCloseTimer = () => {
    if (panelCloseTimer.current === null) return;
    window.clearTimeout(panelCloseTimer.current);
    panelCloseTimer.current = null;
  };

  const openPanel = (nextPanel: Exclude<SidePanel, null>) => {
    clearPanelCloseTimer();
    setPanelClosing(false);
    setPanel(nextPanel);
  };

  const closePanel = () => {
    if (!panel || panelClosing) return;
    clearPanelCloseTimer();
    setPanelClosing(true);
    panelCloseTimer.current = window.setTimeout(() => {
      setPanel(null);
      setPanelClosing(false);
      panelCloseTimer.current = null;
    }, 180);
  };

  useEffect(() => () => clearPanelCloseTimer(), []);

  useEffect(() => {
    document.documentElement.classList.toggle("motion-disabled", !motion);
    setStorageItem("betterfy:motion", motion ? "on" : "off");
    return () => document.documentElement.classList.remove("motion-disabled");
  }, [motion]);

  useEffect(() => {
    [wukongBuild, enigmaProgress, courierSuccess, pudgeRecovery].forEach((source) => {
      const image = new Image();
      image.src = source;
    });
  }, []);

  useEffect(() => {
    if (!panel) return;
    const closeOnEscape = (event: KeyboardEvent) => {
      if (event.key === "Escape") closePanel();
    };
    window.addEventListener("keydown", closeOnEscape);
    return () => window.removeEventListener("keydown", closeOnEscape);
  }, [panel, panelClosing]);

  useEffect(() => {
    if (route === "discover") return;
    setCatalogCompact(false);
    document.documentElement.style.removeProperty("--catalog-scroll-progress");
  }, [route]);

  const toggleStartup = () => {
    setStartup((value) => {
      setStorageItem("betterfy:startup-preference", value ? "off" : "on");
      return !value;
    });
  };

  const runDiagnostics = async () => {
    setDiagnostic("checking");
    try {
      const result = await engineBridge.validateGamePath(installation.path);
      setDiagnostic(result.verified ? "ready" : "demo");
    } catch {
      setDiagnostic("error");
    }
  };

  const toggleCatalogMod = (id: string) => {
    setSelectedModIds((current) => {
      const next = current.includes(id) ? current.filter((item) => item !== id) : [...current, id];
      setStorageItem("betterfy:selected-mods", JSON.stringify(next));
      return next;
    });
  };

  const applyPreset = (modIds: string[]) => {
    const next = [...new Set(modIds)];
    setSelectedModIds(next);
    setStorageItem("betterfy:selected-mods", JSON.stringify(next));
  };

  const nav = [
    { key: "home", icon: Home, enabled: true },
    { key: "discover", icon: Search, enabled: true },
    { key: "build", icon: Layers3, enabled: true },
    { key: "library", icon: Library, enabled: true },
  ] as const;

  const navigate = (nextRoute: WorkspaceRoute) => {
    if (nextRoute === route) return;
    clearPanelCloseTimer();
    transitionUI(() => {
      setPanel(null);
      setPanelClosing(false);
      setRoute(nextRoute);
    });
  };

  return (
    <main className={`app-frame ${route === "discover" && catalogCompact ? "catalog-focus-mode" : ""}`}>
      <aside className="app-rail" aria-label={language === "ru" ? "Основная навигация" : "Primary navigation"}>
        <nav>
          {nav.map(({ key, icon: Icon, enabled }) => (
            <button
              className={key === route ? "active" : ""}
              disabled={!enabled}
              key={key}
              aria-current={key === route ? "page" : undefined}
              title={!enabled ? `${t.nav[key]} · soon` : t.nav[key]}
              onClick={() => enabled && navigate(key as WorkspaceRoute)}
            >
              <Icon />
              <span>{t.nav[key]}</span>
              {!enabled && <i aria-hidden="true" />}
            </button>
          ))}
        </nav>
        <div className="rail-bottom">
          <button onClick={() => openPanel("settings")}><Settings /><span>{t.nav.settings}</span></button>
          <button onClick={() => openPanel("profile")}><CircleUserRound /><span>{t.nav.profile}</span></button>
        </div>
      </aside>

      <section className="home-surface">
        <header className="home-topbar" data-tauri-drag-region>
          <BetterFyWordmark />
          <div className="topbar-actions">
            <div className="locale-switch" aria-label={language === "ru" ? "Язык" : "Language"}>
              <button className={language === "ru" ? "active" : ""} onClick={() => setLanguage("ru")}>RU</button>
              <button className={language === "en" ? "active" : ""} onClick={() => setLanguage("en")}>EN</button>
            </div>
            <button
              className="icon-button theme-button"
              onClick={() => onThemeChange(theme === "dark" ? "light" : "dark")}
              aria-label={theme === "dark" ? t.panel.light : t.panel.dark}
              title={theme === "dark" ? t.panel.light : t.panel.dark}
            >
              {theme === "dark" ? <Sun /> : <Moon />}
            </button>
            <button className="icon-button" disabled aria-label={language === "ru" ? "Уведомления" : "Notifications"}>
              <Bell />
            </button>
          </div>
        </header>

        <div className="workspace-route" key={route}>
          {route === "home" && (
            <HomeRoute
              t={t}
              installation={installation}
              selectedCount={selectedModIds.length}
              onOpenBuild={() => navigate("build")}
              onOpenCatalog={() => navigate("discover")}
            />
          )}
          {route === "discover" && (
            <Suspense fallback={<div className="catalog-route-loading"><LoaderCircle /></div>}>
              <ModCatalogRoute
                language={language}
                selectedIds={selectedModIds}
                onToggle={toggleCatalogMod}
                onNavigationCompactChange={setCatalogCompact}
              />
            </Suspense>
          )}
          {route === "build" && (
            <BuildRoute
              t={t}
              language={language}
              installation={installation}
              selectedCount={selectedModIds.length}
              onHome={() => navigate("home")}
              onOpenConfigs={() => navigate("library")}
            />
          )}
          {route === "library" && (
            <LibraryRoute
              t={t}
              language={language}
              selectedIds={selectedModIds}
              onApplyPreset={applyPreset}
            />
          )}
        </div>

        <footer className="prototype-note">
          {route === "home"
            ? t.home.preview
            : route === "discover"
              ? t.catalog.prototype
              : route === "build"
                ? t.build.prototype
                : t.library.prototype}
        </footer>
      </section>

      {panel && (
        <div className={`side-panel-layer ${panelClosing ? "is-closing" : ""}`} onMouseDown={closePanel}>
          <aside
            className="side-panel"
            aria-label={panel === "settings" ? t.panel.settings : t.panel.profile}
            aria-modal="true"
            role="dialog"
            onMouseDown={(event) => event.stopPropagation()}
          >
            <header>
              <div>
                <span>BETTERFY / {panel.toUpperCase()}</span>
                <h2>{panel === "settings" ? t.panel.settings : t.panel.profile}</h2>
              </div>
              <button autoFocus onClick={closePanel} aria-label={t.panel.close}><X /></button>
            </header>

            {panel === "settings" ? (
              <div className="panel-content">
                <p>{t.panel.settingsText}</p>
                <section className="panel-group">
                  <span>{t.panel.interface}</span>
                  <div className="panel-row">
                    <i><Languages /></i>
                    <div><strong>{t.panel.language}</strong><small>Русский / English</small></div>
                    <div className="locale-switch">
                      <button className={language === "ru" ? "active" : ""} onClick={() => setLanguage("ru")}>RU</button>
                      <button className={language === "en" ? "active" : ""} onClick={() => setLanguage("en")}>EN</button>
                    </div>
                  </div>
                  <div className="panel-row">
                    <i>{theme === "dark" ? <Moon /> : <Sun />}</i>
                    <div><strong>{t.panel.theme}</strong><small>{t.panel.themeText}</small></div>
                    <div className="theme-segment" aria-label={t.panel.theme}>
                      <button
                        className={theme === "light" ? "active" : ""}
                        onClick={() => onThemeChange("light")}
                      >
                        {t.panel.light}
                      </button>
                      <button
                        className={theme === "dark" ? "active" : ""}
                        onClick={() => onThemeChange("dark")}
                      >
                        {t.panel.dark}
                      </button>
                    </div>
                  </div>
                  <div className="panel-row">
                    <i><SlidersHorizontal /></i>
                    <div><strong>{t.panel.motion}</strong><small>{t.panel.motionText}</small></div>
                    <button
                      className={`motion-switch ${motion ? "active" : ""}`}
                      role="switch"
                      aria-checked={motion}
                      aria-label={t.panel.motion}
                      onClick={() => setMotion((value) => !value)}
                    >
                      <span />
                    </button>
                  </div>
                  <div className="panel-row">
                    <i><Power /></i>
                    <div><strong>{t.panel.startup}</strong><small>{t.panel.startupText}</small></div>
                    <button
                      className={`motion-switch ${startup ? "active" : ""}`}
                      role="switch"
                      aria-checked={startup}
                      aria-label={t.panel.startup}
                      onClick={toggleStartup}
                    >
                      <span />
                    </button>
                  </div>
                </section>
                <section className="panel-group">
                  <span>{t.panel.game}</span>
                  <div className="panel-row game-path-row">
                    <i><Monitor /></i>
                    <div>
                      <strong>{t.panel.gamePath}</strong>
                      <small title={installation.path}>{installation.path}</small>
                    </div>
                    <em className={installation.verified ? "verified" : ""}>
                      {installation.verified ? t.panel.verified : t.panel.demo}
                    </em>
                  </div>
                  <button className="panel-inline-action" onClick={onReconnect}>
                    <FolderOpen />{t.panel.reconnect}<ArrowRight />
                  </button>
                  <div className={`diagnostic-result ${diagnostic}`}>
                    <Stethoscope />
                    <div>
                      <strong>{t.panel.diagnostics}</strong>
                      <small>
                        {diagnostic === "checking"
                          ? t.panel.checking
                          : diagnostic === "ready"
                            ? t.panel.diagnosticReady
                            : diagnostic === "demo"
                              ? t.panel.diagnosticDemo
                              : diagnostic === "error"
                                ? t.panel.diagnosticError
                                : t.panel.diagnosticsText}
                      </small>
                    </div>
                    <button onClick={runDiagnostics} disabled={diagnostic === "checking"}>
                      {t.panel.runDiagnostics}
                    </button>
                  </div>
                </section>
              </div>
            ) : (
              <div className="panel-content">
                <p>{t.panel.profileText}</p>
                <div className="profile-identity">
                  <span className="profile-brand-avatar" aria-label="BetterFy"><BetterFyMark /></span>
                  <div>
                    <h3>{session?.displayName ?? "BetterFy Tester"}</h3>
                    <p>{session?.username ? `@${session.username}` : "@BetterFyBot"}</p>
                  </div>
                  <i><Check /></i>
                </div>
                <dl className="profile-facts">
                  <div><dt>{t.panel.signedIn}</dt><dd>@BetterFyBot</dd></div>
                  <div><dt>Access</dt><dd>{session?.accessTier ?? t.panel.plan}</dd></div>
                </dl>
                <section className="profile-community">
                  <MessageCircle />
                  <div><strong>{t.panel.community}</strong><small>{t.panel.communityText}</small></div>
                  <a href="https://t.me/BetterFyBot" target="_blank" rel="noreferrer" aria-label={t.panel.openBot}>
                    <ExternalLink />
                  </a>
                </section>
                <button className="panel-signout" onClick={onSignOut}><LogOut />{t.panel.signout}</button>
              </div>
            )}
          </aside>
        </div>
      )}
    </main>
  );
}

function HomeRoute({
  t,
  installation,
  selectedCount,
  onOpenBuild,
  onOpenCatalog,
}: {
  t: typeof copy.ru;
  installation: GameInstallation;
  selectedCount: number;
  onOpenBuild: () => void;
  onOpenCatalog: () => void;
}) {
  return (
    <div className="home-content">
      <section className="home-hero">
        <div className="hero-copy">
          <span className="eyebrow"><Sparkles />{t.home.eyebrow}</span>
          <h1>{t.home.title.split("\n").map((line: string) => <span key={line}>{line}</span>)}</h1>
          <p>{t.home.subtitle}</p>
          <button className="primary-action" onClick={onOpenBuild}>
            <span><Layers3 /></span><b>{t.home.action}</b><ArrowRight />
          </button>
          <small>{t.home.actionHint}</small>
        </div>

        <div className="hero-art-slot" aria-label={t.home.artTitle}>
          <div className="art-orbit" aria-hidden="true"><i /><i /><i /></div>
          <span>{t.home.artLabel}</span>
          <img className="home-bane-art" src={baneHome} alt="" />
          <div className="art-caption">
            <strong>{t.home.artTitle}</strong>
            <small>{t.home.artHint}</small>
          </div>
        </div>
      </section>

      <section className="home-lower">
        <article className="build-preview">
          <div><span>{t.home.buildEyebrow}</span><h2>{t.home.buildTitle}</h2><p>{t.home.buildText}</p></div>
          <div className="empty-build"><Boxes /><span>{selectedCount ? `${selectedCount} ${t.home.build}` : t.home.empty}</span></div>
          <button onClick={onOpenCatalog}>{t.home.browse}<ArrowRight /></button>
        </article>
        <aside className="system-strip">
          <SystemState label={t.home.state} value={installation.verified ? t.home.connected : t.home.demoConnected} tone={installation.verified ? "ready" : "warning"} />
          <SystemState label={t.home.build} value={t.home.buildValue} />
          <SystemState label={t.home.access} value={t.home.accessValue} tone="ready" />
        </aside>
      </section>

      <CommunityCard t={t.community} />
    </div>
  );
}

function BuildRoute({
  t,
  language,
  installation,
  selectedCount,
  onHome,
  onOpenConfigs,
}: {
  t: typeof copy.ru;
  language: Language;
  installation: GameInstallation;
  selectedCount: number;
  onHome: () => void;
  onOpenConfigs: () => void;
}) {
  const [plan, setPlan] = useState<BuildPlan | null>(null);
  const [planning, setPlanning] = useState(false);
  const [view, setView] = useState<BuildView>("review");
  const [choice, setChoice] = useState<"violet" | "clean" | null>(null);
  const [progress, setProgress] = useState(0);
  const [targetProgress, setTargetProgress] = useState(0);
  const [receipt, setReceipt] = useState<BuildReceipt | null>(null);
  const [recoveryOperationId, setRecoveryOperationId] = useState<string | null>(null);
  const [restoring, setRestoring] = useState(false);
  const [steamReceipt, setSteamReceipt] = useState<SteamConfigReceipt | null>(null);

  const inspectPlan = async () => {
    setPlanning(true);
    try {
      const nextPlan = await engineBridge.planBuild([
        "fixture.ambient-violet",
        "fixture.ambient-clean",
      ]);
      setPlan(nextPlan);
      setView(nextPlan.conflicts.length ? "conflict" : "ready");
    } finally {
      setPlanning(false);
    }
  };

  useEffect(() => {
    if (view !== "progress" || progress >= targetProgress) return;
    const timer = window.setTimeout(
      () => setProgress((value) => Math.min(targetProgress, value + Math.max(1, Math.ceil((targetProgress - value) / 12)))),
      42,
    );
    return () => window.clearTimeout(timer);
  }, [progress, targetProgress, view]);

  useEffect(() => {
    if (view !== "progress" || progress < 100 || !receipt) return;
    const timer = window.setTimeout(() => setView("success"), 520);
    return () => window.clearTimeout(timer);
  }, [progress, receipt, view]);

  const chooseVariant = (nextChoice: "violet" | "clean") => {
    setChoice(nextChoice);
    setView("ready");
  };

  const startBuild = async () => {
    const modId = choice === "clean" ? "fixture.ambient-clean" : "fixture.ambient-violet";
    setProgress(2);
    setTargetProgress(8);
    setReceipt(null);
    setRecoveryOperationId(null);
    setView("progress");
    const startedAt = Date.now();
    try {
      const nextReceipt = await engineBridge.buildProfile(
        [modId],
        language,
        (snapshot) => setTargetProgress(snapshot.progress),
      );
      setReceipt(nextReceipt);
      setTargetProgress(100);
    } catch {
      try {
        const operations = await engineBridge.listOperations();
        const interrupted = operations.find(
          (operation) => operation.createdAtMs >= startedAt - 1000
            && operation.phase !== "ready"
            && operation.phase !== "rolled_back",
        );
        setRecoveryOperationId(interrupted?.operationId ?? null);
      } catch {
        setRecoveryOperationId(null);
      }
      setView("recovery");
    }
  };

  const restorePreview = async () => {
    setRestoring(true);
    try {
      if (steamReceipt?.operationId) {
        await engineBridge.prepareRuntimeForPatch();
        await engineBridge.rollbackSteamLaunchOptions(steamReceipt.operationId);
      }
      const operationId = receipt?.operationId ?? recoveryOperationId;
      if (operationId) await engineBridge.rollbackOperation(operationId);
      setRestoring(false);
      setView("restored");
    } catch {
      setRestoring(false);
    }
  };

  const resetBuild = () => {
    setPlan(null);
    setChoice(null);
    setProgress(0);
    setTargetProgress(0);
    setReceipt(null);
      setRecoveryOperationId(null);
      setSteamReceipt(null);
    setView("review");
  };

  const phaseIndex = progress < 24 ? 0 : progress < 50 ? 1 : progress < 78 ? 2 : 3;
  const isPlanningSurface = view === "review" || view === "conflict" || view === "ready";

  return (
    <div className={`build-experience build-view-${view}`}>
      {isPlanningSurface && (
        <section className="build-studio">
          <div className="build-studio-copy">
            <header className="route-heading">
              <span>{t.build.eyebrow}</span>
              <h1 className="accent-title"><AccentTitle text={t.build.title} /></h1>
              <p>{t.build.text}</p>
            </header>

            <div className={`build-decision ${view === "conflict" ? "is-conflict" : view === "ready" ? "is-ready" : ""}`}>
              <span>
                {view === "conflict" ? <TriangleAlert /> : view === "ready" ? <CircleCheckBig /> : <Layers3 />}
                {view === "conflict" ? t.build.conflict : view === "ready" ? t.build.readyLabel : t.build.plan}
              </span>
              <h2>{view === "conflict" ? t.build.conflictTitle : view === "ready" ? t.build.readyTitle : t.build.emptyTitle}</h2>
              <p>{view === "conflict" ? t.build.conflictText : view === "ready" ? t.build.readyText : t.build.emptyText}</p>

              {view === "conflict" && (
                <div className="conflict-choices" role="group" aria-label={t.build.conflictTitle}>
                  <button onClick={() => chooseVariant("violet")}>
                    <i className="violet" /><span><strong>{t.build.keepViolet}</strong><small>{t.build.variantViolet}</small></span><ArrowRight />
                  </button>
                  <button onClick={() => chooseVariant("clean")}>
                    <i className="clean" /><span><strong>{t.build.keepClean}</strong><small>{t.build.variantClean}</small></span><ArrowRight />
                  </button>
                  <small>{t.build.replaceHint}</small>
                </div>
              )}

              <div className="build-studio-actions">
                {view === "review" && (
                  <button className="build-main-action" onClick={inspectPlan} disabled={planning}>
                    <Stethoscope />{planning ? t.build.planning : t.build.inspect}<ArrowRight />
                  </button>
                )}
                {view === "ready" && (
                  <button className="build-main-action" onClick={startBuild}>
                    <Play />{t.build.start}<ArrowRight />
                  </button>
                )}
                {view !== "conflict" && (
                  <button className="build-quiet-action" onClick={onHome}>
                    <RotateCcw />{t.build.back}
                  </button>
                )}
                {view === "ready" && (
                  <button className="build-text-action" onClick={() => setView("recovery")}>
                    <ArchiveRestore />{t.build.recoveryPreview}
                  </button>
                )}
              </div>
            </div>
          </div>

          <div className="build-character-scene" aria-label="Sun Wukong build scene">
            <span>WUKONG / BUILD REVIEW</span>
            <div className="build-character-backlight" aria-hidden="true" />
            <img src={wukongBuild} alt="" />
            <i className="build-character-light" aria-hidden="true" />
            <dl className="build-scene-plan">
              <div><dt>{t.build.items}</dt><dd>{plan?.inputs.length ?? 2}</dd></div>
              <div><dt>{t.build.dependencies}</dt><dd>{plan?.operations.length ?? "—"}</dd></div>
              <div className={view === "conflict" ? "conflict" : choice ? "resolved" : ""}>
                <dt>{t.build.conflicts}</dt>
                <dd>{view === "conflict" ? plan?.conflicts.length ?? 1 : choice ? <Check /> : "—"}</dd>
              </div>
              <div className="build-plan-id">
                <dt>{plan ? `DRY RUN · ${plan.planId}` : t.build.unavailable}</dt>
                <dd><PackageOpen /></dd>
              </div>
            </dl>
          </div>
        </section>
      )}

      {view === "progress" && (
        <section className="build-operation">
          <div className="operation-world" aria-hidden="true">
            <img src={enigmaProgress} alt="" />
            <i className="operation-world-glow" />
            <div className="operation-orbit"><i /><i /><i /></div>
            <span>STAGING / PREVIEW</span>
          </div>
          <div className="operation-copy" role="status" aria-live="polite">
            <span>{t.build.progressEyebrow}</span>
            <h1 className="accent-title"><AccentTitle text={t.build.progressTitle} /></h1>
            <p>{t.build.progressText}</p>
            <div className="operation-meter">
              <strong>{String(progress).padStart(2, "0")}<small>%</small></strong>
              <div><i style={{ width: `${progress}%` }} /></div>
            </div>
            <ol className="operation-phases">
              {t.build.phases.map((phase: string, index: number) => (
                <li className={index < phaseIndex ? "done" : index === phaseIndex ? "active" : ""} key={phase}>
                  <span>{index < phaseIndex ? <Check /> : index === phaseIndex ? <LoaderCircle /> : index + 1}</span>
                  <strong>{phase}</strong>
                  <small>{index < phaseIndex ? "OK" : index === phaseIndex ? "…" : "WAIT"}</small>
                </li>
              ))}
            </ol>
            <div className="operation-safety"><ShieldCheck />{t.build.progressNote}</div>
            <button className="operation-error-preview" onClick={() => setView("recovery")}>
              {t.build.interrupt}<ArrowRight />
            </button>
          </div>
        </section>
      )}

      {view === "success" && (
        <section className="build-result success-result">
          <div className="result-art" aria-hidden="true">
            <span>FLYING COURIER / READY</span>
            <div className="result-backlight" />
            <img src={courierSuccess} alt="" />
            <i className="result-light-pass" />
          </div>
          <div className="result-copy">
            <span className="result-status ready"><CircleCheckBig />{t.build.successEyebrow}</span>
            <h1 className="accent-title"><AccentTitle text={t.build.successTitle} /></h1>
            <p>{t.build.successText}</p>
            <SteamActivationPanel
              t={t}
              installation={installation}
              onCommitted={setSteamReceipt}
              onRequestRecovery={() => setView("recovery")}
            />
            <div className="success-config-panel">
              <div className="success-config-copy">
                <span><Save /></span>
                <div><strong>{t.build.configTitle}</strong><small>{t.build.configText}</small></div>
              </div>
              <dl>
                <div><dt>{t.build.selectedContent}</dt><dd>{selectedCount}</dd></div>
                <div><dt>{t.build.resolvedState}</dt><dd><Check /> OK</dd></div>
              </dl>
              <button onClick={onOpenConfigs}><Save />{t.build.openConfigs}<ArrowRight /></button>
            </div>
            <button className="result-secondary" onClick={onHome}>{t.build.finishPreview}<ArrowRight /></button>
          </div>
        </section>
      )}

      {(view === "recovery" || view === "restored") && (
        <section className={`build-result recovery-result ${view === "restored" ? "is-restored" : ""}`}>
          <div className="result-copy">
            <span className={`result-status ${view === "restored" ? "ready" : "danger"}`}>
              {view === "restored" ? <CircleCheckBig /> : <ArchiveRestore />}
              {view === "restored" ? t.build.restoredLabel : t.build.recoveryEyebrow}
            </span>
            <h1 className="accent-title">
              <AccentTitle text={view === "restored" ? t.build.restoredTitle : t.build.recoveryTitle} />
            </h1>
            <p>{view === "restored" ? t.build.restoredText : steamReceipt?.operationId ? t.build.activationRecoveryText : t.build.recoveryText}</p>
            {view === "recovery" ? (
              <button className="restore-action" onClick={restorePreview} disabled={restoring}>
                <span>{restoring ? <LoaderCircle /> : <ArchiveRestore />}</span>
                <strong>{restoring ? t.build.restoring : steamReceipt?.operationId ? t.build.restoreActivation : t.build.restore}</strong>
                <ArrowRight />
              </button>
            ) : (
              <button className="build-main-action" onClick={resetBuild}>
                <RotateCcw />{t.build.rebuild}<ArrowRight />
              </button>
            )}
            <small className="recovery-truth"><ShieldCheck />{t.build.recoverySafe}</small>
          </div>
          <div className="result-art recovery-art" aria-hidden="true">
            <span>PUDGE / RECOVERY</span>
            <div className="result-backlight" />
            <img src={pudgeRecovery} alt="" />
            <i className="result-light-pass" />
          </div>
        </section>
      )}
    </div>
  );
}

type SteamActivationPhase =
  | "loading"
  | "ready"
  | "stopping"
  | "applying"
  | "starting"
  | "complete"
  | "error";

function SteamActivationPanel({
  t,
  installation,
  onCommitted,
  onRequestRecovery,
}: {
  t: typeof copy.ru;
  installation: GameInstallation;
  onCommitted: (receipt: SteamConfigReceipt) => void;
  onRequestRecovery: () => void;
}) {
  const [profiles, setProfiles] = useState<SteamProfileSummary[]>([]);
  const [selectedToken, setSelectedToken] = useState("");
  const [preview, setPreview] = useState<SteamLaunchOptionPreview | null>(null);
  const [confirmed, setConfirmed] = useState(false);
  const [phase, setPhase] = useState<SteamActivationPhase>("loading");
  const [errorCode, setErrorCode] = useState("");
  const [canRecover, setCanRecover] = useState(false);
  const [appliedReceipt, setAppliedReceipt] = useState<SteamConfigReceipt | null>(null);
  const [reloadNonce, setReloadNonce] = useState(0);
  const windowsDesktop = installation.verified
    && navigator.userAgent.toLowerCase().includes("windows");

  useEffect(() => {
    if (!windowsDesktop) {
      setPhase("ready");
      return;
    }
    let active = true;
    setPhase("loading");
    engineBridge.listSteamProfiles()
      .then((items) => {
        if (!active) return;
        setProfiles(items);
        const first = items.find((item) => item.status === "ready" || item.status === "already_managed");
        setSelectedToken(first?.profileToken ?? "");
        setPhase("ready");
      })
      .catch((error) => {
        if (!active) return;
        setErrorCode(error instanceof EngineFault ? error.code : "bridge_error");
        setPhase("error");
      });
    return () => {
      active = false;
    };
  }, [windowsDesktop, reloadNonce]);

  useEffect(() => {
    if (!selectedToken || !windowsDesktop) {
      setPreview(null);
      return;
    }
    let active = true;
    setPreview(null);
    setConfirmed(false);
    engineBridge.previewSteamLaunchOptions(selectedToken)
      .then((result) => {
        if (active) setPreview(result);
      })
      .catch((error) => {
        if (!active) return;
        setErrorCode(error instanceof EngineFault ? error.code : "bridge_error");
        setPhase("error");
      });
    return () => {
      active = false;
    };
  }, [selectedToken, windowsDesktop]);

  const statusLabel = (status: SteamProfileSummary["status"]) => {
    if (status === "ready") return t.build.activationReady;
    if (status === "already_managed") return t.build.activationManaged;
    if (status === "launch_option_conflict") return t.build.activationConflict;
    return t.build.activationInvalid;
  };

  const activate = async () => {
    if (!preview || !confirmed) return;
    setErrorCode("");
    setCanRecover(false);
    try {
      setPhase("stopping");
      await engineBridge.prepareRuntimeForPatch();
      let applied = appliedReceipt;
      if (!applied) {
        setPhase("applying");
        applied = await engineBridge.applySteamLaunchOptions(preview);
        setAppliedReceipt(applied);
        onCommitted(applied);
        setCanRecover(Boolean(applied.operationId));
      }
      setPhase("starting");
      await engineBridge.startSteamAfterProfile(applied.profileToken, applied.operationId);
      await new Promise((resolve) => window.setTimeout(resolve, 900));
      setPhase("complete");
    } catch (error) {
      setErrorCode(error instanceof EngineFault ? error.code : "bridge_error");
      setPhase("error");
    }
  };

  const working = phase === "stopping" || phase === "applying" || phase === "starting";
  const phaseLabel = phase === "stopping"
    ? t.build.activationStopping
    : phase === "applying"
      ? t.build.activationApplying
      : t.build.activationStarting;

  return (
    <section className={`steam-activation phase-${phase}`} aria-labelledby="steam-activation-title">
      <header>
        <span><Power /></span>
        <div>
          <strong id="steam-activation-title">{t.build.activationTitle}</strong>
          <small>{t.build.activationText}</small>
        </div>
      </header>

      {!windowsDesktop ? (
        <div className="steam-activation-notice"><Monitor />{t.build.activationDesktopOnly}</div>
      ) : phase === "loading" ? (
        <div className="steam-activation-notice"><LoaderCircle />{t.build.activationLoading}</div>
      ) : phase === "error" && profiles.length === 0 ? (
        <div className="steam-activation-load-error" role="alert">
          <div><TriangleAlert /><span>{t.build.activationError}<small>{errorCode}</small></span></div>
          <button onClick={() => setReloadNonce((value) => value + 1)}><RotateCcw />{t.build.activationRetry}</button>
        </div>
      ) : profiles.length === 0 ? (
        <div className="steam-activation-notice"><CircleUserRound />{t.build.activationEmpty}</div>
      ) : (
        <>
          <div className="steam-profile-list" role="radiogroup" aria-label={t.build.activationProfile}>
            {profiles.map((profile) => {
              const selectable = profile.status === "ready" || profile.status === "already_managed";
              return (
                <button
                  type="button"
                  role="radio"
                  aria-checked={selectedToken === profile.profileToken}
                  className={selectedToken === profile.profileToken ? "selected" : ""}
                  disabled={!selectable || working || phase === "complete"}
                  key={profile.profileToken}
                  onClick={() => {
                    setSelectedToken(profile.profileToken);
                    setAppliedReceipt(null);
                    setCanRecover(false);
                  }}
                >
                  <span>{profile.profileIndex.toString().padStart(2, "0")}</span>
                  <div><strong>{t.build.activationProfile} {profile.profileIndex}</strong><small>{statusLabel(profile.status)}</small></div>
                  {selectedToken === profile.profileToken ? <Check /> : <CircleUserRound />}
                </button>
              );
            })}
          </div>

          {phase === "complete" ? (
            <div className="steam-activation-complete" role="status"><CircleCheckBig />{t.build.activationComplete}</div>
          ) : working ? (
            <div className="steam-activation-progress" role="status" aria-live="polite">
              <LoaderCircle /><span>{phaseLabel}</span><i />
            </div>
          ) : (
            <>
              <label className="steam-activation-confirm">
                <input
                  type="checkbox"
                  checked={confirmed}
                  onChange={(event) => setConfirmed(event.target.checked)}
                />
                <span><Check /></span>
                <small>{t.build.activationConfirm}</small>
              </label>
              <button
                className="play-action"
                disabled={!preview || !confirmed}
                onClick={activate}
              >
                <span>{phase === "error" ? <RotateCcw /> : <Power />}</span>
                <strong>{phase === "error" ? t.build.activationRetry : t.build.activationAction}</strong>
                <ArrowRight />
              </button>
            </>
          )}

          {phase === "error" && (
            <div className="steam-activation-error" role="alert">
              <TriangleAlert />
              <span>{t.build.activationError}<small>{errorCode}</small></span>
            </div>
          )}
          {canRecover && (
            <button className="steam-activation-recovery" onClick={onRequestRecovery}>
              <ArchiveRestore />{t.build.activationRecovery}<ArrowRight />
            </button>
          )}
        </>
      )}
    </section>
  );
}

function LibraryRoute({
  t,
  language,
  selectedIds,
  onApplyPreset,
}: {
  t: typeof copy.ru;
  language: Language;
  selectedIds: string[];
  onApplyPreset: (modIds: string[]) => void;
}) {
  return (
    <div className="route-page library-route">
      <header className="route-heading">
        <span>{t.library.eyebrow}</span>
        <h1 className="accent-title"><AccentTitle text={t.library.title} /></h1>
        <p>{t.library.text}</p>
      </header>
      <PresetManager language={language} selectedIds={selectedIds} onApply={onApplyPreset} />
      <section className="library-grid">
        <article>
          <i><FolderOpen /></i>
          <div><span>LOCAL / IMPORT</span><h2>{t.library.imports}</h2><p>{t.library.importsText}</p></div>
          <small>{t.library.empty}</small>
        </article>
        <article>
          <i><BookOpen /></i>
          <div><span>BUILDS / SAVED</span><h2>{t.library.saved}</h2><p>{t.library.savedText}</p></div>
          <small>{t.library.empty}</small>
        </article>
        <article className="community-library-card">
          <i><Users /></i>
          <div><span>BETTERFY / COMMUNITY</span><h2>{t.community.libraryTitle}</h2><p>{t.community.libraryText}</p></div>
          <small>{t.community.soon}</small>
        </article>
      </section>
    </div>
  );
}

function CommunityCard({ t }: { t: typeof copy.ru.community }) {
  return (
    <aside className="community-card">
      <div className="community-symbol" aria-hidden="true"><Heart /><i /><i /></div>
      <div>
        <span>{t.label}</span>
        <h2>{t.title}</h2>
        <p>{t.text}</p>
        <small>{t.note}</small>
      </div>
      <a href="https://t.me/BetterFyBot" target="_blank" rel="noreferrer">
        <MessageCircle />{t.action}<ExternalLink />
      </a>
    </aside>
  );
}

function SystemState({
  label,
  value,
  tone = "neutral",
}: {
  label: string;
  value: string;
  tone?: "neutral" | "warning" | "ready";
}) {
  return (
    <div className={`system-state ${tone}`}>
      <i aria-hidden="true" />
      <span><small>{label}</small><strong>{value}</strong></span>
    </div>
  );
}
