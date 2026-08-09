import { useEffect, useMemo, useRef, useState } from "react";
import {
  ArrowDownToLine,
  ArrowRight,
  Check,
  ChevronRight,
  CircleUserRound,
  Download,
  Github,
  Globe2,
  Menu,
  PackageCheck,
  ShieldCheck,
  Sparkles,
  X,
  Zap,
} from "lucide-react";
import BetterFyWordmark from "../../src/BetterFyWordmark";

type Language = "ru" | "en";
type ReleaseState =
  | { phase: "loading" }
  | { phase: "ready"; version: string; date: string; url: string; direct: boolean }
  | { phase: "fallback"; url: string };

type WebSession = {
  userId: string;
  displayName: string;
  username?: string;
  accessTier: string;
  avatarAvailable: boolean;
};

type AuthStage = "idle" | "checking" | "code" | "verifying" | "ready";

const RELEASES_URL = "https://github.com/zori-xyz/BetterFy/releases/latest";
const REPOSITORY_URL = "https://github.com/zori-xyz/BetterFy";
const BOT_URL = "https://t.me/BeterFyBot?start=web_login";
const SESSION_STORAGE_KEY = "betterfy:web-session";
const DEFAULT_AUTH_URL = "https://betterfy-auth.zori-xyz.workers.dev";

function allowlistedExternalUrl(value: string | undefined, kind: "auth" | "release") {
  if (!value) return undefined;
  try {
    const url = new URL(value);
    if (url.protocol !== "https:") return undefined;
    if (kind === "release") {
      const expected = "/zori-xyz/BetterFy/releases/";
      return url.hostname === "github.com" && url.pathname.startsWith(expected) ? url.toString() : undefined;
    }
    return url.toString();
  } catch {
    return undefined;
  }
}

const copy = {
  ru: {
    nav: ["Возможности", "Как работает", "Скачать"],
    skip: "Перейти к содержанию",
    account: "Войти",
    eyebrow: "DOTA 2 · MOD PLATFORM",
    heroLine1: "Твоя Dota.",
    heroLine2: "Только лучше.",
    heroBody: "Моды, интерфейс, звуки и скины — в одной сборке. BetterFy покажет конфликты до того, как что-либо попадёт в Dota 2.",
    download: "Скачать для Windows",
    explore: "Как устроена сборка",
    current: "Текущая сборка",
    currentState: "Можно собирать",
    content: "Выбор",
    verify: "Проверка",
    recovery: "Откат",
    chapter: "КАК ЭТО РАБОТАЕТ",
    sectionTitle: "Собери. Проверь. Примени.",
    sectionBody: "Ты выбираешь контент. BetterFy сверяет файлы, зависимости и конфликты, а затем показывает точный план изменений.",
    steps: [
      ["01", "Собери набор", "Моды меняют работу игры. Гардероб — её внешний вид. Они лежат отдельно и не смешиваются в каталоге."],
      ["02", "Сверь состав", "Перед установкой видны замены, зависимости и конфликты. Спорный файл нельзя протащить дальше случайно."],
      ["03", "Подтверди план", "Движок получает только тот список изменений, который ты просмотрел и подтвердил."],
      ["04", "Верни файлы", "Для каждой операции остаётся журнал. Если сборка не подошла, BetterFy знает, что нужно восстановить."],
    ],
    engineEyebrow: "ENGINE / CONTROLLED",
    engineTitle: "Интерфейс не трогает Dota. Этим занимается Rust-движок.",
    engineBody: "Он находит установку, проверяет скачанный пакет, готовит изменения в отдельной папке и записывает каждый шаг. React только показывает состояние и передаёт подтверждённые команды.",
    enginePoints: ["Пакет проверяется до распаковки", "Операцию можно отменить", "Каждая запись попадает в журнал", "Непроверенный файл не устанавливается"],
    proofLabel: "WINDOWS / BUILD",
    proofTitle: "Приложение собирается на Windows.",
    proofBody: "Rust и TypeScript проходят автоматические тесты, после чего GitHub Actions собирает NSIS-инсталлятор. Запись в Dota пока отключена.",
    releaseEyebrow: "LATEST / WINDOWS",
    releaseTitle: "Скачать Windows-сборку.",
    releaseBody: "Кнопка ведёт к последнему `.exe` в GitHub Releases. Если готового установщика ещё нет, откроется список выпусков.",
    checking: "Проверяем последний релиз…",
    latest: "Последний релиз",
    fallback: "Страница релизов",
    releaseButton: "Скачать BetterFy",
    requirements: "Windows 10/11 · x64 · Early Access",
    integrity: "Источник файла: github.com/zori-xyz/BetterFy",
    communityEyebrow: "BETTERFY / COMMUNITY",
    communityTitle: "Поймал баг — пиши.",
    statusEyebrow: "BETTERFY / СЕЙЧАС",
    statusTitle: "Что работает прямо сейчас.",
    statusBody: "Это ранняя сборка. Здесь разделено то, что уже работает, то, что проходит тесты, и то, что пока выключено.",
    statusItems: [
      ["01", "Работает", "Поиск Dota и чтение состояния"],
      ["02", "На тестах", "План сборки, архивы и staging"],
      ["03", "Подключено", "Telegram-вход и единый профиль"],
    ],
    statusNote: "BetterFy пока не изменяет игровые файлы. Сначала мы проверим полный цикл установки и восстановления на Windows.",
    communityBody: "Ошибки, идеи и результаты Windows-тестов собираем через @BeterFyBot. Код и задачи открыты на GitHub.",
    openBot: "Открыть @BeterFyBot",
    github: "Открыть GitHub",
    faqTitle: "Перед установкой.",
    faq: [
      ["BetterFy уже меняет файлы Dota 2?", "Нет. Сейчас приложение находит Dota, читает состояние и проверяет тестовые планы. Установка в игру включится после Windows-тестов восстановления."],
      ["Как обновляется приложение?", "BetterFy проверяет новые выпуски на GitHub. Когда опубликован новый установщик, встроенный updater предложит скачать и установить его."],
      ["Зачем нужен Telegram?", "Через Telegram будет подтверждаться вход и новое устройство. BetterFy не получает доступ к перепискам и не связывает Telegram со Steam автоматически."],
      ["Что будет с исходными файлами?", "Движок уже ведёт журнал тестовых операций. Обещать восстановление реальных файлов мы будем только после полного Windows-теста: установка, ошибка, откат и повторная проверка."],
    ],
    modalEyebrow: "ACCOUNT / EARLY ACCESS",
    modalTitle: "Войди через Telegram.",
    modalBody: "Открой @BeterFyBot, получи одноразовый код и введи шесть цифр здесь.",
    modalPending: "Код живёт 10 минут и срабатывает один раз. BetterFy не получает доступ к перепискам.",
    authCta: "Получить код в @BeterFyBot",
    codeLabel: "Одноразовый код",
    codePlaceholder: "000000",
    verifyCode: "Подтвердить вход",
    verifying: "Проверяем код…",
    invalidCode: "Код не подошёл или уже истёк. Получи новый в боте.",
    signedIn: "Telegram подключён",
    signOut: "Выйти",
    downloadLocked: "Войди через Telegram, чтобы скачать сборку.",
    downloadError: "Сборка пока недоступна. Попробуй ещё раз позже.",
    close: "Закрыть",
    footer: "BetterFy разрабатывается открыто. Код, сборки и список задач лежат на GitHub.",
    footerMeta: "EARLY ACCESS · OPEN DEVELOPMENT",
  },
  en: {
    nav: ["Features", "How it works", "Download"],
    skip: "Skip to content",
    account: "Sign in",
    eyebrow: "DOTA 2 · MOD PLATFORM",
    heroLine1: "Your Dota.",
    heroLine2: "Only better.",
    heroBody: "Mods, interface, audio, and skins in one build. BetterFy shows conflicts before anything reaches Dota 2.",
    download: "Download for Windows",
    explore: "How builds work",
    current: "Current build",
    currentState: "Ready to assemble",
    content: "Select",
    verify: "Check",
    recovery: "Rollback",
    chapter: "HOW IT WORKS",
    sectionTitle: "Assemble. Check. Apply.",
    sectionBody: "You pick the content. BetterFy checks files, dependencies, and conflicts, then shows the exact change plan.",
    steps: [
      ["01", "Build a set", "Mods change how the game works. Wardrobe content changes how it looks. They stay separate in the catalog."],
      ["02", "Check the files", "Replacements, dependencies, and conflicts appear before installation. A disputed file cannot slip through unnoticed."],
      ["03", "Approve the plan", "The engine receives only the list of changes you reviewed and confirmed."],
      ["04", "Restore files", "Every operation leaves a journal. If a build is wrong, BetterFy knows which files belong back in place."],
    ],
    engineEyebrow: "ENGINE / CONTROLLED",
    engineTitle: "The interface never edits Dota. The Rust engine does.",
    engineBody: "It finds the installation, validates downloaded packages, stages changes in a separate directory, and records every step. React only displays state and sends confirmed commands.",
    enginePoints: ["Packages checked before extraction", "Operations can be cancelled", "Every write is journaled", "Unverified files are rejected"],
    proofLabel: "WINDOWS / BUILD",
    proofTitle: "The app builds on Windows.",
    proofBody: "Rust and TypeScript pass automated tests before GitHub Actions produces the NSIS installer. Writes to Dota are still disabled.",
    releaseEyebrow: "LATEST / WINDOWS",
    releaseTitle: "Download the Windows build.",
    releaseBody: "The button points to the latest `.exe` in GitHub Releases. If there is no installer yet, it opens the releases list.",
    checking: "Checking the latest release…",
    latest: "Latest release",
    fallback: "Releases page",
    releaseButton: "Download BetterFy",
    requirements: "Windows 10/11 · x64 · Early Access",
    integrity: "File source: github.com/zori-xyz/BetterFy",
    communityEyebrow: "BETTERFY / COMMUNITY",
    communityTitle: "Found a bug? Tell us.",
    statusEyebrow: "BETTERFY / NOW",
    statusTitle: "What works today.",
    statusBody: "This is an early build. Here is what already works, what is under test, and what is still switched off.",
    statusItems: [
      ["01", "Working", "Dota discovery and read-only state"],
      ["02", "Under test", "Build plans, archives, and staging"],
      ["03", "Connected", "Telegram sign-in and shared profile"],
    ],
    statusNote: "BetterFy does not modify game files yet. The complete install and recovery cycle must pass on Windows first.",
    communityBody: "Send bugs, ideas, and Windows test results through @BeterFyBot. Code and issues are public on GitHub.",
    openBot: "Open @BeterFyBot",
    github: "Open GitHub",
    faqTitle: "Before you install.",
    faq: [
      ["Does BetterFy modify Dota 2 files yet?", "No. The current app finds Dota, reads its state, and checks fixture plans. Installation unlocks after full recovery testing on Windows."],
      ["How does the app update?", "BetterFy checks GitHub for new releases. When a new installer is published, the built-in updater offers to download and install it."],
      ["Why Telegram?", "Telegram will confirm sign-in and new devices. BetterFy cannot read private chats and does not link Telegram to Steam automatically."],
      ["What happens to the original files?", "The engine already journals fixture operations. We will only promise recovery for real game files after the entire Windows cycle passes: install, failure, rollback, and verification."],
    ],
    modalEyebrow: "ACCOUNT / EARLY ACCESS",
    modalTitle: "Sign in with Telegram.",
    modalBody: "Open @BeterFyBot, request a one-time code, and enter the six digits here.",
    modalPending: "The code lasts 10 minutes and works once. BetterFy cannot read your chats.",
    authCta: "Get a code from @BeterFyBot",
    codeLabel: "One-time code",
    codePlaceholder: "000000",
    verifyCode: "Confirm sign-in",
    verifying: "Checking the code…",
    invalidCode: "That code has expired or was already used. Request another one in the bot.",
    signedIn: "Telegram connected",
    signOut: "Sign out",
    downloadLocked: "Sign in with Telegram to download the build.",
    downloadError: "The build is unavailable right now. Try again later.",
    close: "Close",
    footer: "BetterFy is developed in public. Source, builds, and open tasks are on GitHub.",
    footerMeta: "EARLY ACCESS · OPEN DEVELOPMENT",
  },
} as const;

function formatDate(value: string, language: Language) {
  return new Intl.DateTimeFormat(language === "ru" ? "ru-RU" : "en-US", {
    day: "numeric",
    month: "short",
    year: "numeric",
  }).format(new Date(value));
}

function readStoredLanguage(): Language {
  try {
    return window.localStorage.getItem("betterfy-site-language") === "en" ? "en" : "ru";
  } catch {
    return "ru";
  }
}

function storeLanguage(language: Language) {
  try {
    window.localStorage.setItem("betterfy-site-language", language);
  } catch {
    // Privacy modes may deny storage. Language remains valid for this page view.
  }
}

function useLatestRelease(language: Language): ReleaseState {
  const [release, setRelease] = useState<ReleaseState>({ phase: "loading" });

  useEffect(() => {
    const controller = new AbortController();
    fetch("https://api.github.com/repos/zori-xyz/BetterFy/releases/latest", {
      headers: { Accept: "application/vnd.github+json" },
      signal: controller.signal,
    })
      .then((response) => {
        if (!response.ok) throw new Error("release_unavailable");
        return response.json() as Promise<{
          tag_name?: string;
          published_at?: string;
          html_url?: string;
          assets?: Array<{ name?: string; browser_download_url?: string }>;
        }>;
      })
      .then((data) => {
        const installer = data.assets?.find((asset) => asset.name?.toLowerCase().endsWith(".exe"));
        const installerUrl = allowlistedExternalUrl(installer?.browser_download_url, "release");
        const releasePage = allowlistedExternalUrl(data.html_url, "release");
        setRelease({
          phase: "ready",
          version: data.tag_name ?? "BetterFy",
          date: data.published_at ? formatDate(data.published_at, language) : "GitHub Releases",
          url: installerUrl ?? releasePage ?? RELEASES_URL,
          direct: Boolean(installerUrl),
        });
      })
      .catch((error: unknown) => {
        if (error instanceof DOMException && error.name === "AbortError") return;
        setRelease({ phase: "fallback", url: RELEASES_URL });
      });
    return () => controller.abort();
  }, [language]);

  return release;
}

function Site() {
  const [language, setLanguage] = useState<Language>(readStoredLanguage);
  const [menuOpen, setMenuOpen] = useState(false);
  const [accountOpen, setAccountOpen] = useState(false);
  const [headerCompact, setHeaderCompact] = useState(false);
  const [authStage, setAuthStage] = useState<AuthStage>("checking");
  const [authSession, setAuthSession] = useState<WebSession | null>(null);
  const [sessionToken, setSessionToken] = useState<string | null>(() => {
    try { return window.sessionStorage.getItem(SESSION_STORAGE_KEY); } catch { return null; }
  });
  const [authCode, setAuthCode] = useState("");
  const [authError, setAuthError] = useState("");
  const [avatarUrl, setAvatarUrl] = useState<string | null>(null);
  const [downloadError, setDownloadError] = useState("");
  const accountTrigger = useRef<HTMLButtonElement>(null);
  const t = copy[language];
  const release = useLatestRelease(language);
  const authUrl = allowlistedExternalUrl(import.meta.env.VITE_BETTERFY_AUTH_URL?.trim() || DEFAULT_AUTH_URL, "auth")!;

  useEffect(() => {
    if (!sessionToken) {
      setAuthStage("idle");
      setAuthSession(null);
      return;
    }
    const controller = new AbortController();
    setAuthStage("checking");
    fetch(new URL("/v1/session/profile", authUrl), {
      headers: { Authorization: `Bearer ${sessionToken}` },
      credentials: "omit",
      signal: controller.signal,
    })
      .then(async (response) => {
        if (!response.ok) throw new Error("session_invalid");
        return response.json() as Promise<WebSession>;
      })
      .then((profile) => {
        setAuthSession(profile);
        setAuthStage("ready");
      })
      .catch((error: unknown) => {
        if (error instanceof DOMException && error.name === "AbortError") return;
        try { window.sessionStorage.removeItem(SESSION_STORAGE_KEY); } catch { /* no-op */ }
        setSessionToken(null);
        setAuthStage("idle");
      });
    return () => controller.abort();
  }, [authUrl, sessionToken]);

  useEffect(() => {
    if (!sessionToken || !authSession?.avatarAvailable) {
      setAvatarUrl(null);
      return;
    }
    const controller = new AbortController();
    let objectUrl: string | null = null;
    fetch(new URL("/v1/session/avatar", authUrl), {
      headers: { Authorization: `Bearer ${sessionToken}` },
      credentials: "omit",
      signal: controller.signal,
    })
      .then((response) => response.ok ? response.blob() : Promise.reject(new Error("avatar_unavailable")))
      .then((blob) => {
        objectUrl = URL.createObjectURL(blob);
        setAvatarUrl(objectUrl);
      })
      .catch(() => setAvatarUrl(null));
    return () => {
      controller.abort();
      if (objectUrl) URL.revokeObjectURL(objectUrl);
    };
  }, [authSession?.avatarAvailable, authUrl, sessionToken]);

  const verifyCode = async () => {
    const code = authCode.replace(/\D/g, "");
    if (!/^\d{6}$/.test(code)) {
      setAuthError(t.invalidCode);
      return;
    }
    setAuthStage("verifying");
    setAuthError("");
    try {
      const response = await fetch(new URL("/v1/auth/telegram/code", authUrl), {
        method: "POST",
        headers: { "content-type": "application/json" },
        body: JSON.stringify({ code }),
        credentials: "omit",
      });
      if (!response.ok) throw new Error("invalid_code");
      const payload = await response.json() as WebSession & { sessionToken?: string };
      if (!payload.sessionToken || !payload.displayName) throw new Error("invalid_response");
      window.sessionStorage.setItem(SESSION_STORAGE_KEY, payload.sessionToken);
      setAuthSession(payload);
      setSessionToken(payload.sessionToken);
      setAuthStage("ready");
      setAuthCode("");
    } catch {
      setAuthStage("code");
      setAuthError(t.invalidCode);
    }
  };

  const signOut = async () => {
    const token = sessionToken;
    try { window.sessionStorage.removeItem(SESSION_STORAGE_KEY); } catch { /* no-op */ }
    setSessionToken(null);
    setAuthSession(null);
    setAvatarUrl(null);
    setAuthStage("idle");
    if (token) {
      fetch(new URL("/v1/session/logout", authUrl), {
        method: "POST",
        headers: { Authorization: `Bearer ${token}` },
        credentials: "omit",
      }).catch(() => undefined);
    }
  };

  const downloadBuild = async () => {
    setDownloadError("");
    if (!sessionToken || !authSession) {
      setAccountOpen(true);
      setAuthStage("idle");
      return;
    }
    try {
      const response = await fetch(new URL("/v1/releases/latest", authUrl), {
        headers: { Authorization: `Bearer ${sessionToken}` },
        credentials: "omit",
      });
      if (!response.ok) throw new Error("release_unavailable");
      const payload = await response.json() as { downloadUrl?: string };
      const url = allowlistedExternalUrl(payload.downloadUrl, "release");
      if (!url) throw new Error("release_invalid");
      window.location.assign(url);
    } catch {
      setDownloadError(t.downloadError);
    }
  };

  useEffect(() => {
    document.documentElement.lang = language;
    storeLanguage(language);
  }, [language]);

  useEffect(() => {
    const updateHeader = () => setHeaderCompact(window.scrollY > 28);
    updateHeader();
    window.addEventListener("scroll", updateHeader, { passive: true });
    return () => window.removeEventListener("scroll", updateHeader);
  }, []);

  useEffect(() => {
    const targets = Array.from(document.querySelectorAll<HTMLElement>("[data-reveal]"));
    if (!("IntersectionObserver" in window) || window.matchMedia("(prefers-reduced-motion: reduce)").matches) {
      targets.forEach((target) => target.classList.add("is-visible"));
      return;
    }
    document.documentElement.classList.add("site-motion-ready");
    const observer = new IntersectionObserver((entries) => {
      entries.forEach((entry) => {
        if (!entry.isIntersecting) return;
        entry.target.classList.add("is-visible");
        observer.unobserve(entry.target);
      });
    }, { threshold: 0.12, rootMargin: "0px 0px -7%" });
    targets.forEach((target) => observer.observe(target));
    return () => {
      observer.disconnect();
      document.documentElement.classList.remove("site-motion-ready");
    };
  }, []);

  useEffect(() => {
    if (!accountOpen) return;
    const closeOnEscape = (event: KeyboardEvent) => {
      if (event.key === "Escape") setAccountOpen(false);
    };
    document.body.classList.add("modal-open");
    window.addEventListener("keydown", closeOnEscape);
    return () => {
      document.body.classList.remove("modal-open");
      window.removeEventListener("keydown", closeOnEscape);
    };
  }, [accountOpen]);

  const releaseLabel = useMemo(() => {
    if (release.phase === "loading") return t.checking;
    if (release.phase === "fallback") return t.fallback;
    return `${t.latest} · ${release.version}`;
  }, [release, t]);

  return (
    <div className="site-shell">
      <a className="skip-link" href="#main">{t.skip}</a>
      <header className={headerCompact ? "site-header is-compact" : "site-header"}>
        <a className="brand-link" href="#top" aria-label="BetterFy home">
          <BetterFyWordmark compact />
        </a>
        <nav className={menuOpen ? "site-nav is-open" : "site-nav"} aria-label="Primary navigation">
          <a href="#features" onClick={() => setMenuOpen(false)}>{t.nav[0]}</a>
          <a href="#journey" onClick={() => setMenuOpen(false)}>{t.nav[1]}</a>
          <a href="#download" onClick={() => setMenuOpen(false)}>{t.nav[2]}</a>
          <div className="nav-language" aria-label="Language">
            <button className={language === "ru" ? "is-active" : ""} onClick={() => { setLanguage("ru"); setMenuOpen(false); }}>RU</button>
            <button className={language === "en" ? "is-active" : ""} onClick={() => { setLanguage("en"); setMenuOpen(false); }}>EN</button>
          </div>
        </nav>
        <div className="header-actions">
          <div className="language-switch" aria-label="Language">
            <button className={language === "ru" ? "is-active" : ""} onClick={() => setLanguage("ru")}>RU</button>
            <button className={language === "en" ? "is-active" : ""} onClick={() => setLanguage("en")}>EN</button>
          </div>
          <button ref={accountTrigger} className="account-button" onClick={() => setAccountOpen(true)}>
            {avatarUrl ? <img src={avatarUrl} alt="" /> : <CircleUserRound />}
            {authSession?.displayName ?? t.account}
          </button>
          <button className="menu-button" aria-label="Menu" aria-expanded={menuOpen} onClick={() => setMenuOpen((value) => !value)}>
            {menuOpen ? <X /> : <Menu />}
          </button>
        </div>
      </header>

      <main id="main">
        <section className="hero" id="top">
          <div className="hero-copy" data-reveal>
            <span className="eyebrow"><Sparkles />{t.eyebrow}</span>
            <h1><span>{t.heroLine1}</span><strong>{t.heroLine2}</strong></h1>
            <p>{t.heroBody}</p>
            <div className="hero-actions">
              <button className="primary-action" type="button" onClick={downloadBuild}><ArrowDownToLine />{t.download}<ArrowRight /></button>
              <a className="text-action" href="#journey">{t.explore}<ChevronRight /></a>
            </div>
          </div>

          <div className="hero-system" aria-label={t.current} data-reveal>
            <div className="system-aura" />
            <div className="system-topline"><span>{t.current}</span><strong><Check />{t.currentState}</strong></div>
            <div className="system-core">
              <div className="core-mark"><span /><span /><span /></div>
              <div className="core-orbit orbit-one" />
              <div className="core-orbit orbit-two" />
            </div>
            <div className="system-path">
              <div><span>01</span><strong>{t.content}</strong></div>
              <i />
              <div><span>02</span><strong>{t.verify}</strong></div>
              <i />
              <div><span>03</span><strong>{t.recovery}</strong></div>
            </div>
          </div>
        </section>

        <section className="journey" id="journey">
          <div className="section-intro" data-reveal>
            <span className="eyebrow">{t.chapter}</span>
            <h2>{t.sectionTitle}</h2>
            <p>{t.sectionBody}</p>
          </div>
          <div className="journey-list">
            {t.steps.map(([number, title, body]) => (
              <article key={number} data-reveal>
                <span>{number}</span>
                <h3>{title}</h3>
                <p>{body}</p>
              </article>
            ))}
          </div>
        </section>

        <section className="engine" id="features">
          <div className="engine-visual" aria-hidden="true" data-reveal>
            <div className="engine-rail rail-one"><i /><i /><i /></div>
            <div className="engine-rail rail-two"><i /><i /><i /></div>
            <div className="engine-node"><ShieldCheck /></div>
          </div>
          <div className="engine-copy" data-reveal>
            <span className="eyebrow">{t.engineEyebrow}</span>
            <h2>{t.engineTitle}</h2>
            <p>{t.engineBody}</p>
            <ul>{t.enginePoints.map((point) => <li key={point}><Check />{point}</li>)}</ul>
          </div>
          <aside className="proof-card" data-reveal>
            <span>{t.proofLabel}</span>
            <strong>{t.proofTitle}</strong>
            <p>{t.proofBody}</p>
            <a href={`${REPOSITORY_URL}/actions`} target="_blank" rel="noreferrer">GitHub Actions <ArrowRight /></a>
          </aside>
        </section>

        <section className="status-section" aria-labelledby="status-title">
          <div className="status-intro" data-reveal>
            <span className="eyebrow"><Zap />{t.statusEyebrow}</span>
            <h2 id="status-title">{t.statusTitle}</h2>
            <p>{t.statusBody}</p>
          </div>
          <div className="status-track" data-reveal>
            {t.statusItems.map(([number, title, body], index) => (
              <article key={number} className={index === 0 ? "is-current" : ""}>
                <span>{number}</span>
                <div><strong>{title}</strong><p>{body}</p></div>
                <i aria-hidden="true" />
              </article>
            ))}
            <div className="status-note"><ShieldCheck />{t.statusNote}</div>
          </div>
        </section>

        <section className="download-section" id="download">
          <div className="download-copy" data-reveal>
            <span className="eyebrow">{t.releaseEyebrow}</span>
            <h2>{t.releaseTitle}</h2>
            <p>{t.releaseBody}</p>
          </div>
          <div className="release-card" data-reveal>
            <div className="release-icon"><Download /></div>
            <div className="release-details">
              <span>{releaseLabel}</span>
              <strong>{t.requirements}</strong>
              <small>{release.phase === "ready" ? release.date : t.integrity}</small>
            </div>
            <button type="button" onClick={downloadBuild} className={release.phase === "loading" ? "is-loading" : ""}>
              {t.releaseButton}<ArrowRight />
            </button>
            <p><PackageCheck />{authSession ? t.integrity : t.downloadLocked}</p>
            {downloadError && <p className="release-error">{downloadError}</p>}
          </div>
        </section>

        <section className="community" data-reveal>
          <div>
            <span className="eyebrow">{t.communityEyebrow}</span>
            <h2>{t.communityTitle}</h2>
            <p>{t.communityBody}</p>
          </div>
          <div className="community-actions">
            <a className="telegram-action" href={BOT_URL} target="_blank" rel="noreferrer"><Globe2 />{t.openBot}<ArrowRight /></a>
            <a className="secondary-action" href={REPOSITORY_URL} target="_blank" rel="noreferrer"><Github />{t.github}</a>
          </div>
        </section>

        <section className="faq" data-reveal>
          <h2>{t.faqTitle}</h2>
          <div>
            {t.faq.map(([question, answer]) => (
              <details key={question}>
                <summary>{question}<span>+</span></summary>
                <p>{answer}</p>
              </details>
            ))}
          </div>
        </section>
      </main>

      <footer className="site-footer">
        <div className="footer-brand">
          <BetterFyWordmark compact />
          <span>{t.footerMeta}</span>
        </div>
        <p>{t.footer}</p>
        <nav className="footer-links" aria-label="Footer">
          <a href={REPOSITORY_URL} target="_blank" rel="noreferrer">GitHub</a>
          <a href={`${REPOSITORY_URL}/blob/main/docs/README.md`} target="_blank" rel="noreferrer">Docs</a>
          <a href={BOT_URL} target="_blank" rel="noreferrer">@BeterFyBot</a>
        </nav>
      </footer>

      {accountOpen && (
        <div className="modal-backdrop" role="presentation" onMouseDown={(event) => { if (event.target === event.currentTarget) setAccountOpen(false); }}>
          <section className="account-modal" role="dialog" aria-modal="true" aria-labelledby="account-title">
            <button className="modal-close" autoFocus onClick={() => { setAccountOpen(false); accountTrigger.current?.focus(); }} aria-label={t.close}><X /></button>
            <div className={avatarUrl ? "modal-icon has-avatar" : "modal-icon"}>
              {avatarUrl ? <img src={avatarUrl} alt="" /> : <CircleUserRound />}
            </div>
            <span className="eyebrow">{t.modalEyebrow}</span>
            {authSession ? (
              <>
                <h2 id="account-title">{authSession.displayName}</h2>
                <p>{authSession.username ? `@${authSession.username}` : t.signedIn}</p>
                <div className="pending-note"><ShieldCheck /><span>{t.signedIn} · {authSession.accessTier}</span></div>
                <button className="secondary-action modal-signout" type="button" onClick={signOut}>{t.signOut}</button>
              </>
            ) : (
              <>
                <h2 id="account-title">{t.modalTitle}</h2>
                <p>{t.modalBody}</p>
                <div className="pending-note"><ShieldCheck /><span>{t.modalPending}</span></div>
                <a className="telegram-action" href={BOT_URL} target="_blank" rel="noreferrer" onClick={() => setAuthStage("code")}>
                  <Globe2 />{t.authCta}<ArrowRight />
                </a>
                {(authStage === "code" || authStage === "verifying") && (
                  <form className="auth-code-form" onSubmit={(event) => { event.preventDefault(); void verifyCode(); }}>
                    <label htmlFor="telegram-code">{t.codeLabel}</label>
                    <input
                      id="telegram-code"
                      inputMode="numeric"
                      autoComplete="one-time-code"
                      maxLength={6}
                      value={authCode}
                      placeholder={t.codePlaceholder}
                      onChange={(event) => { setAuthCode(event.target.value.replace(/\D/g, "").slice(0, 6)); setAuthError(""); }}
                      disabled={authStage === "verifying"}
                    />
                    <button type="submit" className="primary-action" disabled={authStage === "verifying" || authCode.length !== 6}>
                      {authStage === "verifying" ? t.verifying : t.verifyCode}<ArrowRight />
                    </button>
                    {authError && <p role="alert">{authError}</p>}
                  </form>
                )}
              </>
            )}
          </section>
        </div>
      )}
    </div>
  );
}

export default Site;
