import { lazy, Suspense, useEffect, useLayoutEffect, useMemo, useRef, useState } from "react";
import { flushSync } from "react-dom";
import {
  ArchiveRestore,
  ArrowLeft,
  ArrowRight,
  Bell,
  BookOpen,
  Boxes,
  Check,
  CircleCheckBig,
  CircleUserRound,
  ClipboardCheck,
  Copy,
  Crown,
  ChevronRight,
  ExternalLink,
  FolderOpen,
  Gamepad2,
  Home,
  Languages,
  Layers3,
  Library,
  LoaderCircle,
  LogOut,
  MessageCircle,
  Monitor,
  Link2,
  PackageOpen,
  Play,
  Plus,
  Power,
  RotateCcw,
  Save,
  Search,
  Send,
  Settings,
  ShieldCheck,
  Shirt,
  SlidersHorizontal,
  Sparkles,
  Stethoscope,
  Sun,
  Moon,
  TriangleAlert,
  Users,
  Wrench,
  X,
} from "lucide-react";
import AuthFlow from "./AuthFlow";
import AppUpdater from "./AppUpdater";
import AccentTitle from "./AccentTitle";
import BetterFyWordmark from "./BetterFyWordmark";
import bigBrainEmoticon from "./assets/dota-emoticons/big-brain.gif";
import bigBrainEmoticonStill from "./assets/dota-emoticons/big-brain.png";
import creepDanceEmoticon from "./assets/dota-emoticons/creepdance.gif";
import creepDanceEmoticonStill from "./assets/dota-emoticons/creepdance.png";
import kittyJugEmoticon from "./assets/dota-emoticons/kitty-jug.gif";
import kittyJugEmoticonStill from "./assets/dota-emoticons/kitty-jug.png";
import marciOmnomEmoticon from "./assets/dota-emoticons/marci-omnom.gif";
import marciOmnomEmoticonStill from "./assets/dota-emoticons/marci-omnom.png";
import poghanimEmoticon from "./assets/dota-emoticons/poghanim.gif";
import poghanimEmoticonStill from "./assets/dota-emoticons/poghanim.png";
import puckChampEmoticon from "./assets/dota-emoticons/puckchamp.gif";
import puckChampEmoticonStill from "./assets/dota-emoticons/puckchamp.png";
import swaghanimEmoticon from "./assets/dota-emoticons/swaghanim.gif";
import swaghanimEmoticonStill from "./assets/dota-emoticons/swaghanim.png";
import betterfyAccessRibbonScene from "./assets/scenes/betterfy-access-ribbon-v1.jpg";
import betterfyWorkbenchScene from "./assets/scenes/betterfy-workbench-v1.jpg";
import {
  fetchDeviceSessions,
  fetchTelegramAvatar,
  restoreDesktopSession,
  revokeAuthSession,
  revokeDeviceSession,
  type AuthSession,
  type DeviceSession,
} from "./auth";
import {
  EngineFault,
  engineBridge,
  type BuildPlan,
  type BuildReceipt,
  type GameInstallation,
  type SteamConfigReceipt,
  type SteamLaunchOptionPreview,
  type SteamProfileSummary,
  type SystemDiagnosticReport,
} from "./engine";
import { useLocale, type Language } from "./i18n";
import type { MinifyCatalogItem } from "./minifyCatalog";
import type { BetterFyCatalogMod } from "./modCatalog";
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
const homeShowcaseIds = new Set([
  "heroes-juggernaut-arcana-purple",
  "creeps-crownfall-radiant-creeps",
  "heroes-maid-marci",
  "couriers-onibi",
  "heroes-io-purple",
  "heroes-dawnbreaker-death-knight",
  "heroes-rubick-plagueroad-apothacary",
  "heroes-primal-beast-prehistoric-predator",
  "item-effects-darkness-radiance",
  "heroes-shadow-fiend-white",
  "heroes-queen-of-pain-rose",
  "heroes-anti-mage-shadow-slayer",
  "heroes-ghost-void-spirit",
  "heroes-winter-ember-spirit",
  "heroes-lina-crystal-empress",
  "heroes-arc-warden-black-hole",
  "heroes-nightmare-chaos-knight",
  "heroes-bloody-enigma",
  "heroes-ice-phoenix",
  "heroes-morphling-darktrench-purple",
  "heroes-earthshaker-red-arcana",
  "terrains-ti6-immortal-gardens",
]);
const communityMoods = [
  { key: "brain", src: bigBrainEmoticon, still: bigBrainEmoticonStill },
  { key: "puck", src: puckChampEmoticon, still: puckChampEmoticonStill },
  { key: "creep", src: creepDanceEmoticon, still: creepDanceEmoticonStill },
  { key: "pog", src: poghanimEmoticon, still: poghanimEmoticonStill },
  { key: "swag", src: swaghanimEmoticon, still: swaghanimEmoticonStill },
  { key: "marci", src: marciOmnomEmoticon, still: marciOmnomEmoticonStill },
  { key: "kitty", src: kittyJugEmoticon, still: kittyJugEmoticonStill },
] as const;

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
      panelTitle: "Запускаем BetterFy",
      panelText: "Восстанавливаем локальную сессию и готовим интерфейс.",
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
      eyebrow: "ТВОЯ DOTA. ТВОИ МОДЫ.",
      titleEmpty: "Выбери моды\nдля своей Dota",
      titleActive: "Продолжи свой\nнабор модов",
      subtitle: "Листай моды и скины, добавляй понравившиеся и проверяй их одной сборкой.",
      demoSubtitle: "Смотри моды и облики, собирай набор. В этом предварительном просмотре файлы Dota 2 не меняются.",
      actionEmpty: "Выбрать моды",
      actionActive: "Выбрать ещё",
      editSelection: "Проверить сборку",
      actionHint: "Выбор хранится только на этом устройстве",
      artTitle: "Стол текущей сборки BetterFy",
      showcaseLabel: "Выбранный мод и соседние варианты",
      selectedLabel: "В СБОРКЕ",
      selectedUnit: "элементов",
      sceneEmpty: "Добавь первый мод, и он появится здесь",
      sceneActive: "Можно открыть план и проверить совместимость",
      emptyTitle: "Здесь появится твой набор",
      carouselHint: "Открыть весь каталог модов и скинов",
      buildEyebrow: "АКТИВНАЯ СБОРКА",
      buildTitle: "Первая сборка ждёт контент",
      buildActiveTitle: "Выбор готов к проверке",
      buildText: "Открой каталог и добавь первый мод или образ — выбор сразу появится в сборке.",
      buildActiveText: "Открой план: BetterFy покажет зависимости и конфликты до любых изменений.",
      browse: "Открыть каталог",
      empty: "Пока пусто",
      state: "DOTA 2",
      connected: "Подключена",
      connectedNote: "Путь к игре подтверждён",
      demoConnected: "Preview без доступа",
      demoConnectedNote: "Файлы игры не читаются",
      build: "ТВОЙ НАБОР",
      buildValue: "Пока пусто",
      buildNote: "Выбор сохранён локально",
      access: "АККАУНТ",
      accessValue: "Telegram",
      accessPreview: "Без входа",
      accessNote: "Можно смотреть всё приложение",
      preview: "Prototype shell · игровые файлы не изменяются",
    },
    catalog: {
      prototype: "Каталог · выбор сохраняется локально, загрузка и патчинг пока не выполняются",
    },
    build: {
      eyebrow: "ТВОЯ СБОРКА",
      title: "Твой набор модов",
      titleEmpty: "Начни с первого мода",
      text: "Выбранные моды сохранены на этом устройстве. Ниже показана отдельная демонстрация проверки на тестовых вариантах; твой набор пока не устанавливается.",
      textEmpty: "Открой каталог, выбери моды и вернись сюда: BetterFy соберёт их в один понятный план.",
      selection: "ТВОЯ СБОРКА",
      selectionEmpty: "Здесь появится твой loadout",
      gameDomain: "ИГРА",
      wardrobeDomain: "ГАРДЕРОБ",
      gameEmpty: "Добавить игровой мод",
      wardrobeEmpty: "Добавить облик",
      gameNote: "Функциональные изменения. В preview они не передаются движку.",
      wardrobeNote: "Внешний вид и эффекты. Источник каждого элемента сохраняется.",
      removeItem: "Убрать из сборки",
      selectionReady: "Состав сохранён",
      selectedPrefix: "Выбрано",
      selectionHint: "Выбранные моды пока не передаются в движок и не изменяют Dota 2.",
      addContent: "Изменить состав",
      chooseContent: "Выбрать моды",
      moreItems: "ещё",
      flow: ["Выбери", "Проверь", "Подтверди", "Готово"],
      demoLabel: "БЕЗОПАСНАЯ ДЕМОНСТРАЦИЯ",
      emptyTitle: "Посмотреть проверку в действии",
      emptyText: "Два тестовых варианта покажут, как BetterFy находит конфликт и просит выбрать один. Файлы Dota 2 не затрагиваются.",
      plan: "ПЛАН СБОРКИ",
      items: "Контент",
      dependencies: "Зависимости",
      conflicts: "Конфликты",
      unavailable: "Ожидает каталога",
      back: "Вернуться на главную",
      inspect: "Посмотреть, как работает проверка",
      planning: "Готовим тестовый план…",
      conflictTitle: "Один ресурс — два варианта",
      conflictText: "Violet и Clean меняют одно и то же. Оставь один вариант — BetterFy не продолжит без твоего выбора.",
      conflict: "НУЖЕН ТВОЙ ВЫБОР",
      inspectAgain: "Перестроить план",
      keepViolet: "Выбрать Violet",
      keepClean: "Выбрать Clean",
      variantViolet: "Ambient Violet",
      variantClean: "Ambient Clean",
      replaceHint: "Будет выбран только один вариант ресурса",
      readyLabel: "ТЕСТОВЫЙ ПЛАН / ГОТОВО",
      readyTitle: "Решения приняты",
      readyText: "План согласован. Дальше BetterFy покажет полный цикл подготовки в безопасном preview-режиме.",
      start: "Запустить preview-сборку",
      recoveryPreview: "Посмотреть сценарий восстановления",
      progressEyebrow: "BUILD / IN PROGRESS",
      progressTitle: "Сборка обретает форму",
      progressText: "Показываем будущий engine-процесс. Сейчас работает только интерфейсная симуляция без записи файлов.",
      phases: ["Проверка плана", "Разрешение ресурсов", "Сборка staging", "Проверка результата"],
      progressNote: "Игровая папка не изменяется",
      interrupt: "Смоделировать ошибку",
      successEyebrow: "BUILD / READY",
      successTitle: "Preview завершён",
      successText: "Синтетическая сборка прошла локальную проверку. Файлы Dota 2 не изменялись; реальная активация остаётся закрыта до Windows-проверки.",
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
      activationErrors: {
        shutdown_timeout: "Dota 2 или Steam не закрылись за отведённое время. Закрой их вручную и повтори.",
        dota_close_unavailable: "BetterFy не смог запросить закрытие Dota 2. Сохрани игру, закрой её вручную и повтори.",
        steam_not_found: "Не удалось найти проверенный steam.exe. Проверь установку Steam.",
        steam_shutdown_failed: "Steam не принял команду завершения. Закрой клиент вручную и повтори.",
        steam_start_failed: "Профиль применён, но Steam не удалось запустить. Открой Steam вручную или восстанови настройку.",
        steam_start_timeout: "Steam не появился за отведённое время. Проверь его вручную перед повтором.",
        runtime_busy: "Dota 2 или Steam всё ещё запущены. Изменение профиля заблокировано.",
        steam_config_plan_stale: "Steam-профиль изменился после проверки. Обнови план и подтверди его снова.",
        steam_recovery_required: "Найдена незавершённая операция. Сначала запусти Recovery.",
        steam_config_rollback_conflict: "Steam-профиль изменён после активации. BetterFy не будет перезаписывать эти изменения.",
        fallback: "Настройка не применена. Повтори проверку или открой Recovery, если изменение уже было записано.",
      },
      activationRecovery: "Откатить настройку BetterFy",
      configTitle: "Сохрани эту конфигурацию",
      configText: "Вернись к ней позже или передай пресет другому игроку через менеджер конфигов.",
      openConfigs: "Открыть менеджер конфигов",
      selectedContent: "Выбрано в каталоге",
      resolvedState: "План разрешён",
      operation: "Операция",
      finishPreview: "Завершить preview",
      recoveryEyebrow: "RECOVERY / RESTORE",
      recoveryTitle: "Очистим временную сборку",
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
      text: "Здесь можно сохранить конфиг. Импорт модов и история установленных сборок ещё готовятся.",
      imports: "Локальные импорты",
      importsText: "Добавление личных файлов появится вместе с безопасной проверкой архивов.",
      saved: "Сохранённые сборки",
      savedText: "После первой сборки здесь появятся версии, которые можно восстановить или повторить.",
      empty: "Пока нет материалов",
      prototype: "Конфиги сохраняются локально · импорт мод-архивов пока отключён",
    },
    community: {
      label: "BETTERFY / COMMUNITY",
      title: "Строй BetterFy вместе с нами",
      text: "Предлагай моды, сообщай о багах и получай помощь напрямую от команды.",
      action: "Открыть @BeterFyBot",
      note: "Один канал для идей, поддержки и ранних сборок.",
      libraryTitle: "Сборки сообщества",
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
      diagnosticsText: "Проверить Windows, Dota 2, процессы, Steam-профили и staging BetterFy.",
      runDiagnostics: "Запустить проверку",
      runDiagnosticsAgain: "Проверить снова",
      checking: "Проверяем…",
      diagnosticError: "Диагностика недоступна",
      diagnosticVersion: "BetterFy",
      diagnosticCopy: "Скопировать безопасный отчёт",
      diagnosticCopied: "Отчёт скопирован",
      diagnosticPrivacy: "Без путей, Steam ID и данных Telegram",
      diagnosticOverall: {
        ready: "Система готова к тесту",
        attention: "Есть пункты для проверки",
        blocked: "Сначала нужен обязательный шаг",
        unsupported: "Режим разработки",
      },
      diagnosticLabels: {
        platform: "Платформа",
        game: "Dota 2",
        runtime: "Процессы",
        steam_profiles: "Steam-профили",
        staging: "Staging BetterFy",
        content: "Проверенный контент",
      },
      diagnosticDetails: {
        windows_supported: "Windows runtime доступен",
        development_only: "Интеграция проверяется только на Windows",
        game_verified: "Установка и обязательные файлы подтверждены",
        game_missing: "Установка Dota 2 не подтверждена",
        demo_game: "В браузере используется демонстрационный путь",
        runtime_ready: "Steam и Dota 2 закрыты — запись разрешена",
        dota_running: "Dota 2 запущена; перед патчингом она будет закрыта",
        steam_running: "Steam запущен; перед патчингом он будет закрыт",
        runtime_unavailable: "Не удалось прочитать состояние процессов",
        profiles_missing: "Локальные профили не найдены — войди в Steam один раз",
        profiles_need_attention: "Некоторые launch options требуют решения",
        profiles_managed: "Есть профиль, уже активированный BetterFy",
        profiles_ready: "Локальные профили готовы к активации",
        profiles_unavailable: "Профили недоступны в этой среде",
        staging_recovery_available: "Есть незавершённая или ошибочная staging-операция",
        staging_history_ready: "Журнал staging читается корректно",
        staging_clean: "Незавершённых staging-операций нет",
        staging_unavailable: "Журнал staging требует проверки",
        content_store_ready: "Проверенные пакеты доступны движку",
        content_store_empty: "Хранилище готово; пакеты появятся при первой сборке",
        content_store_invalid: "Целостность хранилища контента нарушена",
      },
      profile: "Профиль",
      profileText: "Аккаунт BetterFy, доступ и связь с Telegram.",
      connected: "Подключен",
      synced: "Аккаунт синхронизирован с Telegram.",
      previewProfile: "Предварительный просмотр без входа в Telegram.",
      notConnected: "Без входа",
      accessCardText: "Твой текущий уровень доступа BetterFy.",
      telegramText: "Связь с аккаунтом и уведомления",
      accessText: "Текущий план",
      sessionsText: "Устройства с доступом",
      signedIn: "Telegram",
      access: "Доступ",
      earlyAccess: "Ранний доступ",
      premiumAccess: "BetterFy Premium",
      activeUntil: "до",
      recurring: "продлевается каждые 30 дней",
      sessions: "Активные входы",
      currentSession: "Это приложение",
      webSession: "Браузер",
      desktopSession: "Приложение Windows",
      unknownSession: "Ранее созданный вход",
      sessionUntil: "до",
      revokeSession: "Завершить",
      sessionsUnavailable: "Не удалось обновить список входов.",
      plan: "Founding Tester",
      community: "Комьюнити и поддержка",
      communityText: "Новости, ранние сборки и связь с командой.",
      openBot: "Открыть @BeterFyBot",
      signout: "Выйти из профиля",
    },
  },
  en: {
    loading: {
      label: "BETTERFY / DESKTOP",
      title: "Preparing your Dota",
      subtitle: "Starting the shell. Dota 2 files stay untouched until you allow access.",
      panelTitle: "Starting BetterFy",
      panelText: "Restoring the local session and preparing the interface.",
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
      eyebrow: "YOUR DOTA. YOUR MODS.",
      titleEmpty: "Choose mods for\nyour Dota",
      titleActive: "Keep shaping\nyour loadout",
      subtitle: "Browse mods and skins, add what you like, and review everything as one build.",
      demoSubtitle: "Browse mods and skins. Preview mode keeps Dota 2 files unchanged.",
      actionEmpty: "Choose mods",
      actionActive: "Choose more",
      editSelection: "Review build",
      actionHint: "Your selection stays on this device",
      artTitle: "BetterFy current build table",
      showcaseLabel: "Featured mod and nearby picks",
      selectedLabel: "IN THIS BUILD",
      selectedUnit: "items",
      sceneEmpty: "Pick a first mod and it will appear here",
      sceneActive: "Open the plan to review compatibility",
      emptyTitle: "Your loadout will take shape here",
      carouselHint: "Open the complete mods and skins catalog",
      buildEyebrow: "ACTIVE BUILD",
      buildTitle: "Your first build needs content",
      buildActiveTitle: "Your selection is ready to check",
      buildText: "Open the catalog and add a first mod or cosmetic — it will appear in the build immediately.",
      buildActiveText: "Open the plan to review dependencies and conflicts before any change.",
      browse: "Open catalog",
      empty: "Nothing selected",
      state: "DOTA 2",
      connected: "Connected",
      connectedNote: "Game path is verified",
      demoConnected: "Preview only",
      demoConnectedNote: "Game files are not read",
      build: "YOUR LOADOUT",
      buildValue: "Nothing selected",
      buildNote: "Selection is stored locally",
      access: "ACCOUNT",
      accessValue: "Telegram",
      accessPreview: "No sign-in",
      accessNote: "The complete interface is available",
      preview: "Prototype shell · no game files are changed",
    },
    catalog: {
      prototype: "Catalog · selection is stored locally; download and patching are not active yet",
    },
    build: {
      eyebrow: "YOUR LOADOUT",
      title: "Your mod selection",
      titleEmpty: "Start with your first mod",
      text: "Your selection is saved on this device. The check below is a separate demonstration with test variants; your selection is not installed yet.",
      textEmpty: "Open the catalog, choose some mods, and return here. BetterFy will turn them into one clear plan.",
      selection: "YOUR LOADOUT",
      selectionEmpty: "Your loadout will appear here",
      gameDomain: "GAME",
      wardrobeDomain: "WARDROBE",
      gameEmpty: "Add a game mod",
      wardrobeEmpty: "Add a cosmetic",
      gameNote: "Functional changes. Preview does not send them to the engine.",
      wardrobeNote: "Appearance and effects. Every item keeps its source.",
      removeItem: "Remove from build",
      selectionReady: "Selection saved",
      selectedPrefix: "Selected",
      selectionHint: "Selected mods are not sent to the engine yet and do not change Dota 2.",
      addContent: "Edit selection",
      chooseContent: "Choose mods",
      moreItems: "more",
      flow: ["Choose", "Check", "Confirm", "Ready"],
      demoLabel: "SAFE DEMONSTRATION",
      emptyTitle: "See the check in action",
      emptyText: "Two test variants show how BetterFy finds a conflict and asks you to keep one. Dota 2 files stay untouched.",
      plan: "BUILD PLAN",
      items: "Content",
      dependencies: "Dependencies",
      conflicts: "Conflicts",
      unavailable: "Waiting for catalog",
      back: "Return Home",
      inspect: "See how checking works",
      planning: "Building dry-run plan…",
      conflictTitle: "One resource, two variants",
      conflictText: "Violet and Clean change the same thing. Keep one variant — BetterFy will not continue without your choice.",
      conflict: "YOUR CHOICE IS NEEDED",
      inspectAgain: "Rebuild plan",
      keepViolet: "Choose Violet",
      keepClean: "Choose Clean",
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
      successTitle: "Preview complete",
      successText: "The synthetic build passed its local checks. No Dota 2 files changed; real activation stays closed until Windows verification.",
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
      activationErrors: {
        shutdown_timeout: "Dota 2 or Steam did not close in time. Close them manually and try again.",
        dota_close_unavailable: "BetterFy could not request Dota 2 to close. Save your game, close it manually, and try again.",
        steam_not_found: "A verified steam.exe could not be found. Check the Steam installation.",
        steam_shutdown_failed: "Steam did not accept the shutdown request. Close it manually and try again.",
        steam_start_failed: "The profile was applied, but Steam could not start. Open Steam manually or restore the setting.",
        steam_start_timeout: "Steam did not appear in time. Check it manually before retrying.",
        runtime_busy: "Dota 2 or Steam is still running. The profile change remains blocked.",
        steam_config_plan_stale: "The Steam profile changed after review. Refresh and confirm a new plan.",
        steam_recovery_required: "An unfinished operation was found. Run Recovery first.",
        steam_config_rollback_conflict: "The Steam profile changed after activation. BetterFy will not overwrite those changes.",
        fallback: "The setting was not applied. Retry the check, or open Recovery if a change was already written.",
      },
      activationRecovery: "Roll back the BetterFy setting",
      configTitle: "Save this configuration",
      configText: "Return to it later or share the preset through the config manager.",
      openConfigs: "Open config manager",
      selectedContent: "Selected in Discover",
      resolvedState: "Plan resolved",
      operation: "Operation",
      finishPreview: "Finish preview",
      recoveryEyebrow: "RECOVERY / RESTORE",
      recoveryTitle: "Clear the temporary build",
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
      text: "Save a config here. Mod imports and installed build history are still in development.",
      imports: "Local imports",
      importsText: "Personal files arrive with safe archive validation.",
      saved: "Saved builds",
      savedText: "After your first build, repeatable and recoverable versions will appear here.",
      empty: "No materials yet",
      prototype: "Configs are stored locally · mod archive import is still disabled",
    },
    community: {
      label: "BETTERFY / COMMUNITY",
      title: "Build BetterFy with us",
      text: "Suggest mods, report bugs, and get direct help from the team.",
      action: "Open @BeterFyBot",
      note: "One channel for ideas, support, and early builds.",
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
      diagnosticsText: "Check Windows, Dota 2, processes, Steam profiles, and BetterFy staging.",
      runDiagnostics: "Run diagnostics",
      runDiagnosticsAgain: "Run again",
      checking: "Checking…",
      diagnosticError: "Diagnostics unavailable",
      diagnosticVersion: "BetterFy",
      diagnosticCopy: "Copy safe report",
      diagnosticCopied: "Report copied",
      diagnosticPrivacy: "No paths, Steam IDs, or Telegram data",
      diagnosticOverall: {
        ready: "System ready for testing",
        attention: "A few items need attention",
        blocked: "A required step is missing",
        unsupported: "Development environment",
      },
      diagnosticLabels: {
        platform: "Platform",
        game: "Dota 2",
        runtime: "Processes",
        steam_profiles: "Steam profiles",
        staging: "BetterFy staging",
        content: "Verified content",
      },
      diagnosticDetails: {
        windows_supported: "Windows runtime is available",
        development_only: "Integration is verified on Windows only",
        game_verified: "Installation and required markers are verified",
        game_missing: "The Dota 2 installation is not verified",
        demo_game: "Browser mode uses a demonstration path",
        runtime_ready: "Steam and Dota 2 are closed — writes are allowed",
        dota_running: "Dota 2 is running and will be closed before patching",
        steam_running: "Steam is running and will be closed before patching",
        runtime_unavailable: "Process state could not be inspected",
        profiles_missing: "No local profiles found — sign in to Steam once",
        profiles_need_attention: "Some launch options need a decision",
        profiles_managed: "A profile is already activated by BetterFy",
        profiles_ready: "Local profiles are ready for activation",
        profiles_unavailable: "Profiles are unavailable in this environment",
        staging_recovery_available: "An interrupted or failed staging operation exists",
        staging_history_ready: "The staging journal is readable",
        staging_clean: "No unfinished staging operations",
        staging_unavailable: "The staging journal needs attention",
        content_store_ready: "Verified packages are available to the engine",
        content_store_empty: "The store is ready; packages appear with the first build",
        content_store_invalid: "Content-store integrity is invalid",
      },
      profile: "Profile",
      profileText: "Your BetterFy account, access, and Telegram connection.",
      connected: "Connected",
      synced: "Account synchronized with Telegram.",
      previewProfile: "Interface preview without Telegram sign-in.",
      notConnected: "Not signed in",
      accessCardText: "Your current BetterFy access level.",
      telegramText: "Account connection and notifications",
      accessText: "Current plan",
      sessionsText: "Devices with access",
      signedIn: "Telegram",
      access: "Access",
      earlyAccess: "Early Access",
      premiumAccess: "BetterFy Premium",
      activeUntil: "until",
      recurring: "renews every 30 days",
      sessions: "Active sessions",
      currentSession: "This app",
      webSession: "Browser",
      desktopSession: "Windows app",
      unknownSession: "Earlier session",
      sessionUntil: "until",
      revokeSession: "End",
      sessionsUnavailable: "The session list could not be refreshed.",
      plan: "Founding Tester",
      community: "Community and support",
      communityText: "News, early builds, and direct contact with the team.",
      openBot: "Open @BeterFyBot",
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
    let active = true;
    const restore = async () => {
      const [restored] = await Promise.all([
        restoreDesktopSession().catch(() => null),
        new Promise((resolve) => window.setTimeout(resolve, 2350)),
      ]);
      if (!active) return;
      if (!restored) {
        transitionUI(() => setStage("auth"));
        return;
      }
      const stored = readStoredInstallation();
      setSession(restored);
      setInstallation(stored);
      transitionUI(() => setStage(stored ? "workspace" : "setup"));
    };
    void restore();
    return () => { active = false; };
  }, [stage]);

  const completeAuth = (nextSession: AuthSession) => {
    const stored = readStoredInstallation();
    setSession(nextSession);
    setInstallation(stored);
    transitionUI(() => setStage(stored ? "workspace" : "setup"));
  };

  const enterPreview = () => {
    setSession({
      userId: "betterfy-preview",
      displayName: "Preview",
      accessTier: "preview",
      source: "demo",
    });
    setInstallation({
      path: "Preview / Dota 2",
      executablePath: "Preview only",
      steamLibrary: "BetterFy interface preview",
      client: "Prototype preview",
      source: "demo",
      verified: false,
    });
    transitionUI(() => setStage("workspace"));
  };

  const completeSetup = (game: GameInstallation) => {
    setInstallation(game);
    setStorageItem(installationStorageKey, JSON.stringify(game));
    transitionUI(() => setStage("workspace"));
  };

  let content;
  if (stage === "loading") content = <LoadingScreen />;
  else if (stage === "auth") content = <AuthFlow onComplete={completeAuth} onPreview={enterPreview} />;
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
      <img className="launch-background" src={betterfyAccessRibbonScene} alt="" aria-hidden="true" />
      <div className="launch-atmosphere" aria-hidden="true" />
      <header className="launch-topline">
        <BetterFyWordmark />
        <span>{t.label}</span>
      </header>
      <div className="launch-content">
        <div className="launch-copy">
          <span>{t.label}</span>
          <h1 className="accent-title"><AccentTitle text={t.title} /></h1>
          <p>{t.subtitle}</p>
        </div>
        <section className="launch-panel" role="status" aria-live="polite">
          <div className="launch-panel-head">
            <span><LoaderCircle /></span>
            <div><strong>{t.panelTitle}</strong><small>{t.panelText}</small></div>
          </div>
          <div className="launch-sequence">
            <ol>
              {t.steps.map((step: string, index: number) => (
                <li key={step}><i aria-hidden="true">{index + 1}</i><span>{step}</span><b aria-hidden="true" /></li>
              ))}
            </ol>
          </div>
        </section>
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
  const diagnosticCopyTimer = useRef<number | null>(null);
  const [route, setRoute] = useState<WorkspaceRoute>("home");
  const [motion, setMotion] = useState(() => getStorageItem("betterfy:motion") !== "off");
  const [startup, setStartup] = useState(() => getStorageItem("betterfy:startup-preference") === "on");
  const [diagnostic, setDiagnostic] = useState<"idle" | "checking" | "complete" | "error">("idle");
  const [diagnosticReport, setDiagnosticReport] = useState<SystemDiagnosticReport | null>(null);
  const [diagnosticCopied, setDiagnosticCopied] = useState(false);
  const [selectedModIds, setSelectedModIds] = useState<string[]>(() =>
    getStoredStringArray("betterfy:selected-mods"));
  const [selectedGameModIds, setSelectedGameModIds] = useState<string[]>(() =>
    getStoredStringArray("betterfy:selected-minify-mods"));
  const [telegramAvatarUrl, setTelegramAvatarUrl] = useState<string | null>(null);
  const [deviceSessions, setDeviceSessions] = useState<DeviceSession[]>([]);
  const [deviceSessionsError, setDeviceSessionsError] = useState(false);
  const [revokingSessionId, setRevokingSessionId] = useState<string | null>(null);

  const accessSummary = useMemo(() => {
    if (!session || session.source === "demo") return t.panel.notConnected;
    if (session.accessTier !== "premium") return t.panel.earlyAccess;
    const expiry = session.accessExpiresAt
      ? new Intl.DateTimeFormat(language === "ru" ? "ru-RU" : "en-GB", {
          day: "2-digit",
          month: "short",
          year: "numeric",
        }).format(new Date(session.accessExpiresAt * 1000))
      : null;
    return [t.panel.premiumAccess, expiry ? `${t.panel.activeUntil} ${expiry}` : null].filter(Boolean).join(" · ");
  }, [language, session, t.panel.activeUntil, t.panel.earlyAccess, t.panel.notConnected, t.panel.premiumAccess]);

  useEffect(() => {
    let objectUrl: string | null = null;
    let active = true;
    if (!session) {
      setTelegramAvatarUrl(null);
      return undefined;
    }
    fetchTelegramAvatar(session)
      .then((blob) => {
        if (!active || !blob) return;
        objectUrl = URL.createObjectURL(blob);
        setTelegramAvatarUrl(objectUrl);
      })
      .catch(() => setTelegramAvatarUrl(null));
    return () => {
      active = false;
      if (objectUrl) URL.revokeObjectURL(objectUrl);
    };
  }, [session]);

  useEffect(() => {
    if (panel !== "profile" || !session) return undefined;
    let active = true;
    setDeviceSessionsError(false);
    fetchDeviceSessions(session)
      .then((devices) => { if (active) setDeviceSessions(devices); })
      .catch(() => { if (active) setDeviceSessionsError(true); });
    return () => { active = false; };
  }, [panel, session]);

  const endDeviceSession = async (device: DeviceSession) => {
    if (!session || revokingSessionId) return;
    setRevokingSessionId(device.sessionId);
    setDeviceSessionsError(false);
    try {
      const revoked = await revokeDeviceSession(session, device.sessionId);
      if (!revoked) throw new Error("session_revoke_failed");
      if (device.current) {
        onSignOut();
        return;
      }
      setDeviceSessions((current) => current.filter((item) => item.sessionId !== device.sessionId));
    } catch {
      setDeviceSessionsError(true);
    } finally {
      setRevokingSessionId(null);
    }
  };

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

  useEffect(() => () => {
    clearPanelCloseTimer();
    if (diagnosticCopyTimer.current !== null) {
      window.clearTimeout(diagnosticCopyTimer.current);
    }
  }, []);

  useEffect(() => {
    document.documentElement.classList.toggle("motion-disabled", !motion);
    setStorageItem("betterfy:motion", motion ? "on" : "off");
    return () => document.documentElement.classList.remove("motion-disabled");
  }, [motion]);

  useEffect(() => {
    if (!panel) return;
    const closeOnEscape = (event: KeyboardEvent) => {
      if (event.key === "Escape") closePanel();
    };
    window.addEventListener("keydown", closeOnEscape);
    return () => window.removeEventListener("keydown", closeOnEscape);
  }, [panel, panelClosing]);

  const toggleStartup = () => {
    setStartup((value) => {
      setStorageItem("betterfy:startup-preference", value ? "off" : "on");
      return !value;
    });
  };

  const runDiagnostics = async () => {
    setDiagnostic("checking");
    setDiagnosticCopied(false);
    try {
      const report = await engineBridge.collectSystemDiagnostics(installation.path);
      setDiagnosticReport(report);
      setDiagnostic("complete");
    } catch {
      setDiagnosticReport(null);
      setDiagnostic("error");
    }
  };

  const copyDiagnostics = async () => {
    if (!diagnosticReport) return;
    try {
      await navigator.clipboard.writeText(JSON.stringify(diagnosticReport, null, 2));
      setDiagnosticCopied(true);
      if (diagnosticCopyTimer.current !== null) {
        window.clearTimeout(diagnosticCopyTimer.current);
      }
      diagnosticCopyTimer.current = window.setTimeout(() => {
        setDiagnosticCopied(false);
        diagnosticCopyTimer.current = null;
      }, 1800);
    } catch {
      setDiagnosticCopied(false);
    }
  };

  const toggleCatalogMod = (id: string) => {
    setSelectedModIds((current) => {
      const next = current.includes(id) ? current.filter((item) => item !== id) : [...current, id];
      setStorageItem("betterfy:selected-mods", JSON.stringify(next));
      return next;
    });
  };

  const toggleGameMod = (id: string) => {
    setSelectedGameModIds((current) => {
      const next = current.includes(id) ? current.filter((item) => item !== id) : [...current, id];
      setStorageItem("betterfy:selected-minify-mods", JSON.stringify(next));
      return next;
    });
  };

  const applyPreset = (modIds: string[], wardrobeIds: string[]) => {
    const nextGame = [...new Set(modIds)];
    const nextWardrobe = [...new Set(wardrobeIds)];
    setSelectedGameModIds(nextGame);
    setSelectedModIds(nextWardrobe);
    setStorageItem("betterfy:selected-minify-mods", JSON.stringify(nextGame));
    setStorageItem("betterfy:selected-mods", JSON.stringify(nextWardrobe));
  };

  const selectedBuildIds = useMemo(
    () => [...selectedGameModIds, ...selectedModIds],
    [selectedGameModIds, selectedModIds],
  );

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
    <main className="app-frame">
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
          <button className={telegramAvatarUrl ? "rail-profile has-avatar" : "rail-profile"} onClick={() => openPanel("profile")}>
            {telegramAvatarUrl ? <img src={telegramAvatarUrl} alt="" /> : <CircleUserRound />}
            <span>{t.nav.profile}</span>
          </button>
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
              language={language}
              installation={installation}
              signedIn={Boolean(session && session.source !== "demo")}
              selectedIds={selectedBuildIds}
              onOpenBuild={() => navigate("build")}
              onOpenCatalog={() => navigate("discover")}
            />
          )}
          {route === "discover" && (
            <Suspense fallback={<div className="catalog-route-loading"><LoaderCircle /></div>}>
              <ModCatalogRoute
                language={language}
                selectedWardrobeIds={selectedModIds}
                selectedGameIds={selectedGameModIds}
                onToggleWardrobe={toggleCatalogMod}
                onToggleGame={toggleGameMod}
              />
            </Suspense>
          )}
          {route === "build" && (
            <BuildRoute
              t={t}
              language={language}
              installation={installation}
              selectedWardrobeIds={selectedModIds}
              selectedGameIds={selectedGameModIds}
              onRemoveWardrobe={toggleCatalogMod}
              onRemoveGame={toggleGameMod}
              onHome={() => navigate("home")}
              onOpenCatalog={() => navigate("discover")}
              onOpenConfigs={() => navigate("library")}
            />
          )}
          {route === "library" && (
            <LibraryRoute
              t={t}
              language={language}
              selectedWardrobeIds={selectedModIds}
              selectedGameIds={selectedGameModIds}
              onApplyPreset={applyPreset}
            />
          )}
        </div>

      </section>

      {panel && (
        <div className={`side-panel-layer ${panelClosing ? "is-closing" : ""}`} onMouseDown={closePanel}>
          <aside
            className={`side-panel ${panel === "profile" ? "profile-panel" : "settings-panel"}`}
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
              <div className="panel-content settings-panel-content">
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
                  <div className={`diagnostic-result ${diagnosticReport?.overall ?? diagnostic}`}>
                    <Stethoscope />
                    <div>
                      <strong>{t.panel.diagnostics}</strong>
                      <small>
                        {diagnostic === "checking"
                          ? t.panel.checking
                          : diagnostic === "error"
                            ? t.panel.diagnosticError
                            : diagnosticReport
                              ? t.panel.diagnosticOverall[diagnosticReport.overall]
                              : t.panel.diagnosticsText}
                      </small>
                    </div>
                    <button onClick={runDiagnostics} disabled={diagnostic === "checking"}>
                      {diagnosticReport ? t.panel.runDiagnosticsAgain : t.panel.runDiagnostics}
                    </button>
                  </div>
                  {diagnosticReport && (
                    <section className="system-diagnostic-report" aria-live="polite">
                      <header>
                        <div>
                          <span>{t.panel.diagnosticVersion}</span>
                          <strong>v{diagnosticReport.appVersion} · {diagnosticReport.platform}</strong>
                        </div>
                        <em className={diagnosticReport.overall}>{t.panel.diagnosticOverall[diagnosticReport.overall]}</em>
                      </header>
                      <ul>
                        {diagnosticReport.checks.map((check) => (
                          <li key={check.code} className={check.state}>
                            <span>{check.state === "ready" ? <CircleCheckBig /> : <TriangleAlert />}</span>
                            <div>
                              <strong>{t.panel.diagnosticLabels[check.code]}</strong>
                              <small>{t.panel.diagnosticDetails[check.detail] ?? check.detail}</small>
                            </div>
                            {check.value > 0 && <b>{String(check.value).padStart(2, "0")}</b>}
                          </li>
                        ))}
                      </ul>
                      <footer>
                        <small><ShieldCheck />{t.panel.diagnosticPrivacy}</small>
                        <button onClick={copyDiagnostics}>
                          {diagnosticCopied ? <ClipboardCheck /> : <Copy />}
                          {diagnosticCopied ? t.panel.diagnosticCopied : t.panel.diagnosticCopy}
                        </button>
                      </footer>
                    </section>
                  )}
                </section>
                <div className="settings-footnote"><ShieldCheck />{t.panel.diagnosticPrivacy}</div>
              </div>
            ) : (
              <div className="panel-content profile-panel-content">
                <p>{t.panel.profileText}</p>
                <div className="profile-identity">
                  <span className="profile-brand-avatar" aria-label={session?.displayName ?? "BetterFy"}>
                    {telegramAvatarUrl ? <img src={telegramAvatarUrl} alt="" /> : <CircleUserRound />}
                  </span>
                  <div className="profile-identity-copy">
                    <h3>{session?.displayName ?? "BetterFy Tester"}</h3>
                    <p>{session?.username ? `@${session.username}` : session?.source === "demo" ? t.panel.notConnected : "@BeterFyBot"}</p>
                    <small><Link2 />{session?.source === "demo" ? t.panel.previewProfile : t.panel.synced}</small>
                  </div>
                  <i><span />{session?.source === "demo" ? t.panel.notConnected : t.panel.connected}</i>
                </div>

                <section className="profile-access-card">
                  <div className="profile-access-head">
                    <span><Crown /></span>
                    <div><strong>{session?.source === "demo" ? t.panel.notConnected : session?.accessTier === "premium" ? t.panel.premiumAccess : t.panel.plan}</strong><small>{session?.source === "demo" ? t.panel.previewProfile : t.panel.accessCardText}</small></div>
                    <em><Crown />{session?.source === "demo" ? t.panel.notConnected : session?.accessTier === "premium" ? t.panel.recurring : t.panel.earlyAccess}</em>
                  </div>
                  <dl>
                    <div><dt>{t.panel.signedIn}</dt><dd>{session?.username ? `@${session.username}` : session?.source === "demo" ? t.panel.notConnected : t.panel.signedIn}</dd></div>
                    <div><dt>{t.panel.access}</dt><dd>{accessSummary}</dd></div>
                    <div><dt>{t.panel.sessions}</dt><dd>{String(deviceSessions.length).padStart(2, "0")}</dd></div>
                  </dl>
                </section>

                <section className="profile-menu-group">
                  <a href="https://t.me/BeterFyBot" target="_blank" rel="noreferrer">
                    <i><Send /></i><span><strong>{t.panel.signedIn}</strong><small>{t.panel.telegramText}</small></span><em><span />{session?.source === "demo" ? t.panel.notConnected : t.panel.connected}</em><ChevronRight />
                  </a>
                  <div>
                    <i><Crown /></i><span><strong>{t.panel.access}</strong><small>{t.panel.accessText}</small></span><em>{accessSummary}</em>
                  </div>
                  <details className="profile-sessions">
                    <summary><i><Monitor /></i><span><strong>{t.panel.sessions}</strong><small>{t.panel.sessionsText}</small></span><em>{String(deviceSessions.length).padStart(2, "0")}</em><ChevronRight /></summary>
                    <div>
                      {deviceSessions.map((device) => {
                        const label = device.current
                          ? t.panel.currentSession
                          : device.clientKind === "desktop"
                            ? t.panel.desktopSession
                            : device.clientKind === "web"
                              ? t.panel.webSession
                              : t.panel.unknownSession;
                        const expiry = new Intl.DateTimeFormat(language === "ru" ? "ru-RU" : "en-GB", {
                          day: "2-digit", month: "short", hour: "2-digit", minute: "2-digit",
                        }).format(new Date(device.expiresAt * 1000));
                        return (
                          <article key={device.sessionId}>
                            <span><strong>{label}</strong><small>{t.panel.sessionUntil} {expiry}</small></span>
                            {!device.current && <button disabled={revokingSessionId === device.sessionId} onClick={() => void endDeviceSession(device)}>{t.panel.revokeSession}</button>}
                          </article>
                        );
                      })}
                      {deviceSessionsError && <p>{t.panel.sessionsUnavailable}</p>}
                    </div>
                  </details>
                </section>

                <section className="profile-menu-group profile-community-group">
                  <a href="https://t.me/BeterFyBot" target="_blank" rel="noreferrer" aria-label={t.panel.openBot}>
                    <i><MessageCircle /></i><span><strong>{t.panel.community}</strong><small>{t.panel.communityText}</small></span><ChevronRight />
                  </a>
                </section>
                <button className="panel-signout" onClick={() => { void revokeAuthSession(session).then(onSignOut).catch(() => setDeviceSessionsError(true)); }}><LogOut />{t.panel.signout}</button>
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
  language,
  installation,
  signedIn,
  selectedIds,
  onOpenBuild,
  onOpenCatalog,
}: {
  t: typeof copy.ru;
  language: Language;
  installation: GameInstallation;
  signedIn: boolean;
  selectedIds: string[];
  onOpenBuild: () => void;
  onOpenCatalog: () => void;
}) {
  const [showcaseMods, setShowcaseMods] = useState<BetterFyCatalogMod[]>([]);
  const [activeShowcaseIndex, setActiveShowcaseIndex] = useState(0);
  useEffect(() => {
    let active = true;
    void import("./modCatalog").then(({ wardrobeCatalogItems }) => {
      if (!active) return;
      const curated = wardrobeCatalogItems.filter((mod) => (
        homeShowcaseIds.has(mod.id) && mod.presentation.previewUrl
      ));
      const pool = curated.length >= 7
        ? curated
        : wardrobeCatalogItems.filter((mod) => mod.presentation.previewUrl);
      let seed = Math.floor(Date.now() / 86_400_000) ^ 0x5bf17;
      const random = () => {
        seed = (seed * 1_664_525 + 1_013_904_223) >>> 0;
        return seed / 4_294_967_296;
      };
      const shuffled = [...pool];
      for (let index = shuffled.length - 1; index > 0; index -= 1) {
        const target = Math.floor(random() * (index + 1));
        [shuffled[index], shuffled[target]] = [shuffled[target], shuffled[index]];
      }
      setShowcaseMods(shuffled.slice(0, 18));
      setActiveShowcaseIndex(0);
    });
    return () => { active = false; };
  }, []);
  useEffect(() => {
    if (showcaseMods.length < 2) return;
    const motionQuery = window.matchMedia("(prefers-reduced-motion: reduce)");
    const timer = window.setInterval(() => {
      if (document.hidden || motionQuery.matches || document.documentElement.classList.contains("motion-disabled")) return;
      setActiveShowcaseIndex((current) => (current + 1) % showcaseMods.length);
    }, 5200);
    return () => window.clearInterval(timer);
  }, [showcaseMods.length]);
  const selectedCount = selectedIds.length;
  const hasSelection = selectedCount > 0;
  const title = hasSelection ? t.home.titleActive : t.home.titleEmpty;
  const activeMod = showcaseMods[activeShowcaseIndex];
  const showPrevious = () => setActiveShowcaseIndex((current) => (
    showcaseMods.length ? (current - 1 + showcaseMods.length) % showcaseMods.length : 0
  ));
  const showNext = () => setActiveShowcaseIndex((current) => (
    showcaseMods.length ? (current + 1) % showcaseMods.length : 0
  ));
  const visibleShowcase = showcaseMods.length
    ? [-2, -1, 0, 1, 2].map((offset) => ({
        mod: showcaseMods[(activeShowcaseIndex + offset + showcaseMods.length) % showcaseMods.length],
        offset,
      }))
    : [];

  return (
    <div className="home-content">
      <section className={`home-workbench ${hasSelection ? "has-selection" : "is-empty"}`} aria-label={t.home.artTitle}>
        <img className="home-workbench-scene" src={betterfyWorkbenchScene} alt="" aria-hidden="true" />
        <div className="home-workbench-shade" aria-hidden="true" />

        <header className="home-workbench-intro">
          <div className="hero-copy">
            <span className="eyebrow"><Sparkles />{t.home.eyebrow}</span>
            <h1>{title.split("\n").map((line: string) => <span key={line}>{line}</span>)}</h1>
            <p>{installation.verified ? t.home.subtitle : t.home.demoSubtitle}</p>
          </div>
          <div className="home-workbench-actions">
            <button className="primary-action" onClick={onOpenCatalog}>
              <span><Layers3 /></span><b>{hasSelection ? t.home.actionActive : t.home.actionEmpty}</b><ArrowRight />
            </button>
            {hasSelection && <button className="home-edit-selection" onClick={onOpenBuild}><Check /><span>{t.home.editSelection}</span><b>{selectedCount}</b></button>}
            <small>{t.home.actionHint}</small>
          </div>
        </header>

        <div className="home-loadout-stage" aria-label={t.home.showcaseLabel}>
          <div
            className="home-coverflow"
            role="region"
            tabIndex={0}
            onKeyDown={(event) => {
              if (event.key === "ArrowLeft") { event.preventDefault(); showPrevious(); }
              if (event.key === "ArrowRight") { event.preventDefault(); showNext(); }
            }}
          >
            {visibleShowcase.map(({ mod, offset }) => (
              <button
                className={`home-coverflow-card position-${offset + 3} ${offset === 0 ? "is-active" : ""} ${selectedIds.includes(mod.id) ? "is-selected" : ""}`}
                key={mod.id}
                onClick={() => {
                  if (offset === 0) onOpenCatalog();
                  else setActiveShowcaseIndex((current) => (current + offset + showcaseMods.length) % showcaseMods.length);
                }}
                aria-current={offset === 0 ? "true" : undefined}
                aria-label={offset === 0 ? `${mod.metadata.name}. ${language === "ru" ? "Открыть в каталоге" : "Open in catalog"}` : mod.metadata.name}
              >
                {mod.presentation.previewUrl && <img src={mod.presentation.previewUrl} alt="" onError={(event) => { event.currentTarget.hidden = true; }} />}
                <span><small>{mod.metadata.categoryLabel[language]}</small><strong>{mod.metadata.name}</strong>{offset === 0 && <i>{language === "ru" ? "Смотреть в каталоге" : "View in catalog"}<ArrowRight /></i>}</span>
                {selectedIds.includes(mod.id) && <b><Check /></b>}
              </button>
            ))}
            {!activeMod && <div className="home-coverflow-card is-active is-loading" aria-hidden="true"><LoaderCircle /></div>}
            <nav className="home-drop-controls" key={activeShowcaseIndex} aria-label={language === "ru" ? "Переключение модов" : "Mod showcase controls"}>
              <button onClick={showPrevious} disabled={!showcaseMods.length} aria-label={language === "ru" ? "Предыдущий" : "Previous"}><ArrowLeft /></button>
              <span><b>{String(activeShowcaseIndex + 1).padStart(2, "0")}</b><i>/</i>{String(showcaseMods.length || 18).padStart(2, "0")}</span>
              <button onClick={showNext} disabled={!showcaseMods.length} aria-label={language === "ru" ? "Следующий" : "Next"}><ArrowRight /></button>
            </nav>
          </div>
        </div>

        <footer className="home-workbench-footer">
          <div><span>{t.home.selectedLabel}</span><strong>{hasSelection ? `${selectedCount} ${t.home.selectedUnit}` : t.home.empty}</strong><small>{hasSelection ? t.home.sceneActive : t.home.sceneEmpty}</small></div>
          <button className="home-carousel-hint" onClick={onOpenCatalog}><Search /><span>{t.home.carouselHint}</span><ArrowRight /></button>
        </footer>
      </section>

      <section className="home-status-board" aria-label={language === "ru" ? "Состояние BetterFy" : "BetterFy status"}>
        <article className={installation.verified ? "is-ready" : "is-preview"}>
          <span><Monitor /></span>
          <div><small>{t.home.state}</small><strong>{installation.verified ? t.home.connected : t.home.demoConnected}</strong><p>{installation.verified ? t.home.connectedNote : t.home.demoConnectedNote}</p></div>
        </article>
        <article className={hasSelection ? "is-ready" : ""}>
          <span><Layers3 /></span>
          <div><small>{t.home.build}</small><strong>{hasSelection ? `${selectedCount} ${t.home.selectedUnit}` : t.home.buildValue}</strong><p>{t.home.buildNote}</p></div>
        </article>
        <article className={signedIn ? "is-ready" : ""}>
          <span><CircleUserRound /></span>
          <div><small>{t.home.access}</small><strong>{signedIn ? t.home.accessValue : t.home.accessPreview}</strong><p>{t.home.accessNote}</p></div>
        </article>
      </section>

      <CommunityCard t={t.community} />
    </div>
  );
}

function BuildRoute({
  t,
  language,
  installation,
  selectedWardrobeIds,
  selectedGameIds,
  onRemoveWardrobe,
  onRemoveGame,
  onHome,
  onOpenCatalog,
  onOpenConfigs,
}: {
  t: typeof copy.ru;
  language: Language;
  installation: GameInstallation;
  selectedWardrobeIds: string[];
  selectedGameIds: string[];
  onRemoveWardrobe: (id: string) => void;
  onRemoveGame: (id: string) => void;
  onHome: () => void;
  onOpenCatalog: () => void;
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
  const [selectedWardrobeMods, setSelectedWardrobeMods] = useState<BetterFyCatalogMod[]>([]);
  const [selectedGameMods, setSelectedGameMods] = useState<Array<MinifyCatalogItem & { previewUrl: string | null }>>([]);

  useEffect(() => {
    let active = true;
    void Promise.all([import("./modCatalog"), import("./minifyCatalog")]).then(([wardrobe, game]) => {
      if (!active) return;
      const wardrobeById = new Map(wardrobe.wardrobeCatalogItems.map((mod) => [mod.id, mod]));
      const gameById = new Map(game.minifyMods.map((mod) => [mod.id, mod]));
      setSelectedWardrobeMods(selectedWardrobeIds
        .map((id) => wardrobeById.get(id))
        .filter((mod): mod is BetterFyCatalogMod => Boolean(mod)));
      setSelectedGameMods(selectedGameIds
        .map((id) => gameById.get(id))
        .filter((mod): mod is MinifyCatalogItem => Boolean(mod))
        .map((mod) => ({ ...mod, previewUrl: game.minifyPreviewUrl(mod.preview) })));
    });
    return () => { active = false; };
  }, [selectedGameIds, selectedWardrobeIds]);

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
  const selectedCount = selectedWardrobeIds.length + selectedGameIds.length;
  const visibleWardrobeMods = selectedWardrobeMods.slice(0, 4);
  const visibleGameMods = selectedGameMods.slice(0, 4);
  const flowStep = view === "review" ? 0 : view === "conflict" || view === "ready" ? 1 : view === "progress" ? 2 : 3;

  return (
    <div className={`build-experience build-view-${view}`}>
      {isPlanningSurface && (
        <section className="build-studio build-preflight-stage">
          <header className="route-heading build-preflight-heading">
            <div>
              <span>{t.build.eyebrow} · {t.build.selectedPrefix} {selectedCount}</span>
              <h1 className="accent-title"><AccentTitle text={selectedCount ? t.build.title : t.build.titleEmpty} /></h1>
              <p>{selectedCount ? t.build.text : t.build.textEmpty}</p>
            </div>
          </header>

          <div className={`build-preflight-board ${selectedCount ? "has-selection" : "is-empty"}`}>
            <section className="build-selection-stage" aria-label={t.build.selection}>
              <header>
                <span><Layers3 />{t.build.selection}</span>
                {selectedCount > 0 && <button onClick={onOpenCatalog}><Plus />{t.build.addContent}</button>}
              </header>

              <div className={`build-blueprint ${selectedCount ? "has-items" : "is-empty"}`}>
                <div className="build-blueprint-axis" aria-hidden="true">
                  <span>BETTERFY</span><i /><small>LOADOUT / LOCAL</small>
                </div>

                <section className="build-blueprint-game" aria-label={t.build.gameDomain}>
                  <header>
                    <span><Wrench />{t.build.gameDomain}</span>
                    <strong>{String(selectedGameIds.length).padStart(2, "0")}</strong>
                  </header>
                  <div className="build-game-stack">
                    {visibleGameMods.map((mod, index) => (
                      <article key={mod.id} style={{ "--build-index": index } as React.CSSProperties}>
                        <i />
                        <span><small>{mod.category}</small><strong>{mod.name[language]}</strong></span>
                        <b>{Object.values(mod.evidence).reduce((total, value) => total + value, 0)}</b>
                        <button onClick={() => onRemoveGame(mod.id)} aria-label={`${t.build.removeItem}: ${mod.name[language]}`}><X /></button>
                      </article>
                    ))}
                    {!visibleGameMods.length && (
                      <button className="build-domain-empty" onClick={onOpenCatalog}><Plus /><span>{t.build.gameEmpty}</span></button>
                    )}
                    {selectedGameIds.length > visibleGameMods.length && <button className="build-domain-more" onClick={onOpenCatalog}>+{selectedGameIds.length - visibleGameMods.length}</button>}
                  </div>
                  <p>{t.build.gameNote}</p>
                </section>

                <section className="build-blueprint-wardrobe" aria-label={t.build.wardrobeDomain}>
                  <header>
                    <span><Shirt />{t.build.wardrobeDomain}</span>
                    <strong>{String(selectedWardrobeIds.length).padStart(2, "0")}</strong>
                  </header>
                  <div className="build-look-strip">
                    {visibleWardrobeMods.map((mod, index) => (
                      <figure key={mod.id} style={{ "--build-index": index } as React.CSSProperties}>
                        {mod.presentation.previewUrl && <img src={mod.presentation.previewUrl} alt="" onError={(event) => { event.currentTarget.hidden = true; }} />}
                        <figcaption><small>{mod.metadata.categoryLabel[language]}</small><strong>{mod.metadata.name}</strong></figcaption>
                        <button onClick={() => onRemoveWardrobe(mod.id)} aria-label={`${t.build.removeItem}: ${mod.metadata.name}`}><X /></button>
                      </figure>
                    ))}
                    {!visibleWardrobeMods.length && (
                      <button className="build-domain-empty" onClick={onOpenCatalog}><Plus /><span>{t.build.wardrobeEmpty}</span></button>
                    )}
                    {selectedWardrobeIds.length > visibleWardrobeMods.length && <button className="build-domain-more" onClick={onOpenCatalog}>+{selectedWardrobeIds.length - visibleWardrobeMods.length}</button>}
                  </div>
                  <p>{t.build.wardrobeNote}</p>
                </section>

                {!selectedCount && (
                  <div className="build-blueprint-empty-action">
                    <Layers3 /><strong>{t.build.selectionEmpty}</strong>
                  </div>
                )}
                <i className="build-blueprint-light" aria-hidden="true" />
              </div>
            </section>

            <aside className={`build-decision build-checkpoint ${view === "conflict" ? "is-conflict" : view === "ready" ? "is-ready" : ""}`}>
              <span>
                {view === "conflict" ? <TriangleAlert /> : view === "ready" ? <CircleCheckBig /> : <Stethoscope />}
                {view === "conflict" ? t.build.conflict : view === "ready" ? t.build.readyLabel : t.build.demoLabel}
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
                {view === "review" && selectedCount > 0 && (
                  <button className="build-main-action" onClick={inspectPlan} disabled={planning}>
                    <Stethoscope />{planning ? t.build.planning : t.build.inspect}<ArrowRight />
                  </button>
                )}
                {view === "review" && selectedCount === 0 && (
                  <>
                    <button className="build-main-action" onClick={onOpenCatalog}><Plus />{t.build.chooseContent}<ArrowRight /></button>
                    <button className="build-text-action" onClick={inspectPlan} disabled={planning}>
                      <Stethoscope />{planning ? t.build.planning : t.build.inspect}
                    </button>
                  </>
                )}
                {view === "ready" && (
                  <button className="build-main-action" onClick={startBuild}><Play />{t.build.start}<ArrowRight /></button>
                )}
                {view === "ready" && (
                  <button className="build-text-action" onClick={() => setView("recovery")}><ArchiveRestore />{t.build.recoveryPreview}</button>
                )}
              </div>
            </aside>

            <footer className="build-preflight-footer">
              <ol className="build-flow" aria-label={t.build.plan}>
                {t.build.flow.map((label: string, index: number) => (
                  <li className={index < flowStep ? "done" : index === flowStep ? "active" : ""} key={label}>
                    <i>{index < flowStep ? <Check /> : String(index + 1).padStart(2, "0")}</i><span>{label}</span>
                  </li>
                ))}
              </ol>
              <small><ShieldCheck />{plan ? `FIXTURE · ${plan.planId}` : t.build.selectionHint}</small>
            </footer>
          </div>
        </section>
      )}

      {view === "progress" && (
        <section className="build-operation">
          <div className="operation-world" aria-hidden="true">
            <span>STAGING / PREVIEW</span>
            <div className="operation-field">
              <i style={{ "--progress": `${progress}%` } as React.CSSProperties} />
              <strong>{String(progress).padStart(2, "0")}</strong>
              <small>BUILD PROFILE</small>
            </div>
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
          <div className="result-copy">
            <div className="result-ready-mark" aria-hidden="true"><CircleCheckBig /></div>
            <span className="result-status ready">{t.build.successEyebrow}</span>
            <h1 className="accent-title"><AccentTitle text={t.build.successTitle} /></h1>
            <p>{t.build.successText}</p>
            <dl className="success-summary">
              <div><dt>{t.build.selectedContent}</dt><dd>{String(selectedCount).padStart(2, "0")}</dd></div>
              <div><dt>{t.build.resolvedState}</dt><dd><Check /> OK</dd></div>
              <div><dt>{t.build.operation}</dt><dd>{receipt?.operationId ?? "LOCAL PREVIEW"}</dd></div>
            </dl>
            <SteamActivationPanel
              t={t}
              installation={installation}
              onCommitted={setSteamReceipt}
              onRequestRecovery={() => setView("recovery")}
            />
            <div className="success-config-panel">
              <div><strong>{t.build.configTitle}</strong><small>{t.build.configText}</small></div>
              <button onClick={onOpenConfigs}><Save />{t.build.openConfigs}</button>
            </div>
            <button className="result-secondary result-finish" onClick={onHome}>{t.build.finishPreview}<ArrowRight /></button>
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
            <span>RECOVERY / LOCAL</span>
            <div className="recovery-visual">
              <ArchiveRestore />
              <strong>{view === "restored" ? "RESTORED" : "ROLL BACK"}</strong>
              <div><i /><i /><i /></div>
              <small>JOURNAL → BACKUP → VERIFY</small>
            </div>
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

  const activationErrorDetail = errorCode in t.build.activationErrors
    ? t.build.activationErrors[errorCode as keyof typeof t.build.activationErrors]
    : t.build.activationErrors.fallback;

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
          <div><TriangleAlert /><span>{t.build.activationError}<small>{activationErrorDetail}</small></span></div>
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
              <span>{t.build.activationError}<small>{activationErrorDetail}</small></span>
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
  selectedWardrobeIds,
  selectedGameIds,
  onApplyPreset,
}: {
  t: typeof copy.ru;
  language: Language;
  selectedWardrobeIds: string[];
  selectedGameIds: string[];
  onApplyPreset: (modIds: string[], wardrobeIds: string[]) => void;
}) {
  return (
    <div className="route-page library-route">
      <header className="route-heading">
        <span>{t.library.eyebrow}</span>
        <h1 className="accent-title"><AccentTitle text={t.library.title} /></h1>
        <p>{t.library.text}</p>
      </header>
      <PresetManager
        language={language}
        selectedModIds={selectedGameIds}
        selectedWardrobeIds={selectedWardrobeIds}
        onApply={onApplyPreset}
      />
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
  const [moodIndex, setMoodIndex] = useState(0);
  const mood = communityMoods[moodIndex];
  useEffect(() => {
    const motionQuery = window.matchMedia("(prefers-reduced-motion: reduce)");
    const timer = window.setInterval(() => {
      if (motionQuery.matches || document.documentElement.classList.contains("motion-disabled")) return;
      setMoodIndex((current) => (current + 1) % communityMoods.length);
    }, 7600);
    return () => window.clearInterval(timer);
  }, []);
  const [titleBeforeBrand, titleAfterBrand = ""] = t.title.split("BetterFy");
  return (
    <aside className={`community-card mood-${mood.key}`}>
      <i className="community-color-field" key={`field-${mood.key}`} aria-hidden="true" />
      <div className="community-emblem" aria-hidden="true">
        <span key={mood.key}>
          <img className="is-animated" src={mood.src} alt="" />
          <img className="is-static" src={mood.still} alt="" />
        </span>
      </div>
      <div className="community-copy">
        <span>{t.label}</span>
        <h2>
          {titleBeforeBrand}
          <b key={`title-${mood.key}`}><span className="community-brand-better">Better</span><span className="community-brand-fy">Fy</span></b>
          {titleAfterBrand}
        </h2>
        <p>{t.text}</p>
        <small>{t.note}</small>
      </div>
      <a href="https://t.me/BeterFyBot" target="_blank" rel="noreferrer">
        <MessageCircle />{t.action}<ExternalLink />
      </a>
    </aside>
  );
}
