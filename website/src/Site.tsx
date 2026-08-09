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

const RELEASES_URL = "https://github.com/zori-xyz/BetterFy/releases/latest";
const REPOSITORY_URL = "https://github.com/zori-xyz/BetterFy";
const BOT_URL = "https://t.me/BeterFyBot?start=web_login";

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
    heroBody: "Выбирай нужное. BetterFy проверит состав, подготовит сборку и оставит тебе понятный путь назад.",
    download: "Скачать для Windows",
    explore: "Посмотреть, как работает",
    current: "Текущая сборка",
    currentState: "Готова к выбору",
    content: "Моды и образы",
    verify: "Проверка плана",
    recovery: "Возврат состояния",
    chapter: "ОДНО СПОКОЙНОЕ ПРОСТРАНСТВО",
    sectionTitle: "От выбора до игры — без технического шума.",
    sectionBody: "Каталог, сборка, проверка и восстановление связаны в один предсказуемый маршрут.",
    steps: [
      ["01", "Выбери", "Функциональные моды и визуальный гардероб разделены. Ты всегда понимаешь, что меняет игру, а что — её внешний вид."],
      ["02", "Проверь", "Перед изменениями BetterFy собирает план, показывает конфликты и не продолжает без понятного результата."],
      ["03", "Примени", "Движок выполняет только подтверждённый план, ведёт журнал операции и проверяет получившееся состояние."],
      ["04", "Верни", "Восстановление — часть основного маршрута, а не спрятанная аварийная кнопка."],
    ],
    engineEyebrow: "ENGINE / CONTROLLED",
    engineTitle: "Под капотом — осторожность. На экране — ясность.",
    engineBody: "BetterFy отделяет интерфейс от работы с файлами. Путь к Dota, загрузки, проверка архивов и будущий патчинг контролируются нативным движком.",
    enginePoints: ["Проверка пакета до распаковки", "Отменяемые операции", "Журнал и граница восстановления", "Никаких обещаний без проверки"],
    proofLabel: "ТЕКУЩЕЕ СОСТОЯНИЕ",
    proofTitle: "Основа уже собирается на Windows.",
    proofBody: "Приложение проходит автоматические проверки Rust и TypeScript, а NSIS-инсталлятор собирается на Windows. Production-патчинг открывается только после полного набора защитных проверок.",
    releaseEyebrow: "LATEST / WINDOWS",
    releaseTitle: "Забери свежую версию.",
    releaseBody: "Сайт ищет последний Windows-инсталлятор в официальных GitHub Releases. Если релиз ещё не опубликован, откроется страница выпусков.",
    checking: "Проверяем последний релиз…",
    latest: "Последний релиз",
    fallback: "Страница релизов",
    releaseButton: "Скачать BetterFy",
    requirements: "Windows 10/11 · x64 · Early Access",
    integrity: "Версия и файлы берутся только из официального репозитория BetterFy.",
    communityEyebrow: "BETTERFY / COMMUNITY",
    communityTitle: "Строим BetterFy вместе.",
    statusEyebrow: "BETTERFY / СЕЙЧАС",
    statusTitle: "Честный статус, без тумана.",
    statusBody: "Мы открываем возможности поэтапно: только после проверки их полного сценария на Windows.",
    statusItems: [
      ["01", "Уже в сборке", "Поиск Dota и чтение состояния"],
      ["02", "Проверяется", "План, архивы и безопасный staging"],
      ["03", "Следом", "Telegram-профиль и production-патчинг"],
    ],
    statusNote: "Запись в файлы игры остаётся закрытой до доказанного восстановления.",
    communityBody: "Ранние сборки, обратная связь и доступ к будущему аккаунту живут рядом с @BeterFyBot.",
    openBot: "Открыть @BeterFyBot",
    github: "Открыть GitHub",
    faqTitle: "Коротко о важном.",
    faq: [
      ["BetterFy уже меняет файлы Dota 2?", "Пока нет. Текущая сборка проверяет продуктовый маршрут и безопасные части движка. Запись в игру откроется после завершения журналирования, резервного копирования и rollback-проверок."],
      ["Как будут приходить обновления?", "Новые подписанные релизы публикуются на GitHub. Приложение умеет проверять обновление и устанавливать его через встроенный updater."],
      ["Зачем нужен Telegram?", "Telegram станет единой точкой подтверждения аккаунта и устройства. Он не доказывает владение Steam-аккаунтом и не получает доступ к перепискам."],
      ["Можно вернуть исходное состояние?", "Это обязательная часть архитектуры движка. Публичное обещание восстановления появится только после Windows-тестов полного цикла."],
    ],
    modalEyebrow: "ACCOUNT / EARLY ACCESS",
    modalTitle: "Один профиль для сайта и приложения.",
    modalBody: "Telegram-вход подключится к этому экрану после запуска защищённого auth-сервиса и настройки разрешённого домена в BotFather.",
    modalPending: "Сейчас @BeterFyBot остаётся каналом раннего доступа. Сайт не имитирует успешный вход и не хранит Telegram-данные локально.",
    authCta: "Продолжить в Telegram",
    authReady: "Перейти к защищённому входу",
    close: "Закрыть",
    footer: "Открытый проект для тех, кто хочет собрать свою Dota спокойно.",
    footerMeta: "EARLY ACCESS · OPEN DEVELOPMENT",
  },
  en: {
    nav: ["Features", "How it works", "Download"],
    skip: "Skip to content",
    account: "Sign in",
    eyebrow: "DOTA 2 · MOD PLATFORM",
    heroLine1: "Your Dota.",
    heroLine2: "Only better.",
    heroBody: "Choose what belongs. BetterFy checks the composition, prepares the build, and keeps a clear way back.",
    download: "Download for Windows",
    explore: "See how it works",
    current: "Current build",
    currentState: "Ready for selection",
    content: "Mods and looks",
    verify: "Plan verification",
    recovery: "State recovery",
    chapter: "ONE CALM WORKSPACE",
    sectionTitle: "From discovery to play — without technical noise.",
    sectionBody: "Catalog, build, verification, and recovery become one predictable route.",
    steps: [
      ["01", "Choose", "Functional mods and the visual Wardrobe stay separate. You always know what changes the game and what changes its look."],
      ["02", "Inspect", "Before any change, BetterFy composes a plan, surfaces conflicts, and stops when the outcome is unclear."],
      ["03", "Apply", "The engine executes only the approved plan, journals the operation, and verifies the resulting state."],
      ["04", "Recover", "Recovery belongs to the primary journey instead of hiding behind an emergency button."],
    ],
    engineEyebrow: "ENGINE / CONTROLLED",
    engineTitle: "Caution underneath. Clarity on screen.",
    engineBody: "BetterFy separates presentation from file operations. Dota discovery, downloads, archive inspection, and future patching are controlled by the native engine.",
    enginePoints: ["Package inspection before extraction", "Cancellable operations", "Journaled recovery boundary", "No claims without evidence"],
    proofLabel: "CURRENT STATE",
    proofTitle: "The foundation already builds on Windows.",
    proofBody: "The app passes automated Rust and TypeScript checks, and the NSIS installer builds on Windows. Production patching stays locked until every recovery gate is proven.",
    releaseEyebrow: "LATEST / WINDOWS",
    releaseTitle: "Get the latest build.",
    releaseBody: "The site looks for the latest Windows installer in official GitHub Releases. If no release is published yet, it opens the releases page.",
    checking: "Checking the latest release…",
    latest: "Latest release",
    fallback: "Releases page",
    releaseButton: "Download BetterFy",
    requirements: "Windows 10/11 · x64 · Early Access",
    integrity: "Versions and files come only from the official BetterFy repository.",
    communityEyebrow: "BETTERFY / COMMUNITY",
    communityTitle: "Build BetterFy with us.",
    statusEyebrow: "BETTERFY / NOW",
    statusTitle: "A clear status. No fog.",
    statusBody: "Capabilities open in stages, only after their complete Windows journey is verified.",
    statusItems: [
      ["01", "In the build", "Dota discovery and read-only state"],
      ["02", "Under verification", "Plans, archives, and safe staging"],
      ["03", "Up next", "Telegram profile and production patching"],
    ],
    statusNote: "Game-file writes remain locked until recovery is proven.",
    communityBody: "Early builds, feedback, and future account access meet at @BeterFyBot.",
    openBot: "Open @BeterFyBot",
    github: "Open GitHub",
    faqTitle: "The important bits.",
    faq: [
      ["Does BetterFy already modify Dota 2 files?", "Not yet. The current build verifies the product journey and safe engine foundations. Game writes unlock only after journaling, backup, and rollback checks are complete."],
      ["How will updates arrive?", "New signed releases are published on GitHub. The app can check for an update and install it through the built-in updater."],
      ["Why Telegram?", "Telegram will confirm the BetterFy account and device. It does not prove Steam ownership and never grants access to private chats."],
      ["Can I restore the original state?", "That is a required part of the engine architecture. The public recovery promise arrives only after complete Windows lifecycle tests."],
    ],
    modalEyebrow: "ACCOUNT / EARLY ACCESS",
    modalTitle: "One profile across web and desktop.",
    modalBody: "Telegram sign-in will connect here once the protected auth service is live and the website domain is allowlisted in BotFather.",
    modalPending: "For now, @BeterFyBot remains the Early Access channel. This site does not simulate a successful login or store Telegram data locally.",
    authCta: "Continue in Telegram",
    authReady: "Continue to secure sign-in",
    close: "Close",
    footer: "An open project for players who want to compose their Dota calmly.",
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
  const accountTrigger = useRef<HTMLButtonElement>(null);
  const t = copy[language];
  const release = useLatestRelease(language);
  const authUrl = allowlistedExternalUrl(import.meta.env.VITE_BETTERFY_AUTH_URL?.trim(), "auth");

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

  const releaseUrl = release.phase === "loading" ? RELEASES_URL : release.url;

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
          <button ref={accountTrigger} className="account-button" onClick={() => setAccountOpen(true)}><CircleUserRound />{t.account}</button>
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
              <a className="primary-action" href={releaseUrl} target="_blank" rel="noreferrer"><ArrowDownToLine />{t.download}<ArrowRight /></a>
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
            <a href={releaseUrl} target="_blank" rel="noreferrer" className={release.phase === "loading" ? "is-loading" : ""}>
              {t.releaseButton}<ArrowRight />
            </a>
            <p><PackageCheck />{t.integrity}</p>
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
            <div className="modal-icon"><CircleUserRound /></div>
            <span className="eyebrow">{t.modalEyebrow}</span>
            <h2 id="account-title">{t.modalTitle}</h2>
            <p>{t.modalBody}</p>
            <div className="pending-note"><ShieldCheck /><span>{t.modalPending}</span></div>
            <a className="telegram-action" href={authUrl || BOT_URL} target="_blank" rel="noreferrer">
              <Globe2 />{authUrl ? t.authReady : t.authCta}<ArrowRight />
            </a>
          </section>
        </div>
      )}
    </div>
  );
}

export default Site;
