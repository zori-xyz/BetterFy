import { useCallback, useEffect, useRef, useState } from "react";
import {
  ArrowDownToLine,
  ArrowRight,
  ArrowUpRight,
  Check,
  ChevronDown,
  CircleDashed,
  CircleUserRound,
  Eye,
  Github,
  Layers3,
  Menu,
  Moon,
  PenLine,
  RotateCcw,
  ShieldCheck,
  Sparkles,
  Sun,
  Wand2,
  X,
} from "lucide-react";
import EmoteStage, { EmoteBackdrop, emoteStyle } from "../../src/studio/EmoteStage";
import { BotStage } from "../../src/studio/BotStage";
import { LocaleBridge } from "../../src/i18n";
import type { StudioMod } from "../../src/studio/model";
import { copy, type Copy, type Language } from "./copy";
import { Account, authUrl, type AuthStage, type DeviceSession, type WebSession } from "./Account";
import { Download, pickRelease, releasesUrl, repo, safeReleaseUrl, type ReleaseMeta, type ReleaseState } from "./Download";

type Theme = "dark" | "light";
type Catalog = typeof import("./catalogData");

const sessionKey = "betterfy:web-session";
const themeKey = "betterfy-site-theme";
const languageKey = "betterfy-site-language";

function readStorage(storage: "local" | "session", key: string) {
  try {
    return (storage === "local" ? localStorage : sessionStorage).getItem(key);
  } catch {
    return null;
  }
}
function writeSession(key: string, value: string) {
  try {
    sessionStorage.setItem(key, value);
    return true;
  } catch {
    return false;
  }
}
function removeSession(key: string) {
  try {
    sessionStorage.removeItem(key);
  } catch {
    // Nothing stored.
  }
}
const reducedMotion = () =>
  typeof window !== "undefined" && window.matchMedia("(prefers-reduced-motion: reduce)").matches;
function initialLanguage(): Language {
  try {
    const param = new URLSearchParams(window.location.search).get("lang");
    if (param === "en" || param === "ru") return param;
  } catch {
    // Fall through to the stored choice.
  }
  return readStorage("local", languageKey) === "en" ? "en" : "ru";
}
function initialTheme(): Theme {
  const saved = readStorage("local", themeKey);
  if (saved === "light" || saved === "dark") return saved;
  try {
    return window.matchMedia("(prefers-color-scheme: light)").matches ? "light" : "dark";
  } catch {
    return "dark";
  }
}

function Wordmark() {
  return (
    <span className="wordmark" aria-label="BetterFy">
      <span className="wordmark-better">Better</span>
      <span className="wordmark-fy">Fy</span>
    </span>
  );
}

// The app's own catalog and showcase, loaded after the first screen.
function useCatalog() {
  const [catalog, setCatalog] = useState<Catalog | null>(null);
  useEffect(() => {
    let active = true;
    import("./catalogData").then((module) => {
      if (active) setCatalog(module);
    });
    return () => {
      active = false;
    };
  }, []);
  return catalog;
}

function Hero({
  t,
  catalog,
  onDownload,
  onOpen,
}: {
  t: Copy;
  catalog: Catalog | null;
  onDownload: () => void;
  onOpen: (mod: StudioMod) => void;
}) {
  const [mood, setMood] = useState(0);
  const Carousel = catalog?.LookCarousel;
  const number = (value: number) => new Intl.NumberFormat(t === copy.ru ? "ru-RU" : "en-GB").format(value);
  return (
    <div className="w-hero-scene" style={emoteStyle(mood)}>
      <section id="top" className="s-hero w-hero">
        <EmoteBackdrop active={mood} />
        <div className="s-hero-copy">
          <span className="s-eyebrow">
            <Sparkles />
            {t.hero.tag}
          </span>
          <h1>
            {t.hero.a}
            <span className="s-hero-accent">
              {t.hero.b}
              <span className="s-hero-sticker">
                <EmoteStage motion active={mood} onChange={setMood} />
              </span>
            </span>
          </h1>
          <p>{t.hero.body}</p>
          <div className="s-hero-actions">
            <button className="s-btn s-btn-primary" onClick={onDownload}>
              <ArrowDownToLine />
              {t.hero.download}
            </button>
            <a className="s-btn s-btn-ghost" href="#how">
              {t.hero.how}
              <ChevronDown />
            </a>
          </div>
        </div>
        {Carousel ? <Carousel motion onOpen={onOpen} /> : <div className="w-carousel-slot" aria-hidden="true" />}
        <p className="w-hero-facts">
          {catalog && (
            <>
              <b>{number(catalog.looksCount)}</b> {t.hero.looks} · <b>{catalog.tuningCount}</b> {t.hero.tuning} ·{" "}
            </>
          )}
          {t.hero.rollback}
        </p>
      </section>
    </div>
  );
}

function Inside({ t, language, catalog }: { t: Copy; language: Language; catalog: Catalog | null }) {
  const [looks, setLooks] = useState<string[]>([]);
  const [tuning, setTuning] = useState<string[]>([]);
  useEffect(() => {
    if (!catalog) return;
    setLooks(catalog.samplePreviews("wardrobe", 3));
    setTuning(catalog.samplePreviews("game", 3));
  }, [catalog]);
  const groups = catalog?.tuningGroups(language) ?? [];
  const number = (value: number) => new Intl.NumberFormat(language === "ru" ? "ru-RU" : "en-GB").format(value);
  return (
    <section className="w-section" id="inside">
      <header className="w-section-head reveal">
        <span className="s-eyebrow">
          <Layers3 />
          {t.inside.tag}
        </span>
        <h2>
          {t.inside.a}
          <span>{t.inside.b}</span>
        </h2>
      </header>
      <div className="w-inside">
        <article className="w-tile reveal" data-spotlight>
          <div className="w-fan" aria-hidden="true">
            {looks.map((src, index) => (
              <img key={src} src={src} alt="" loading="lazy" style={{ ["--i" as string]: index }} />
            ))}
          </div>
          <div className="w-tile-copy">
            <strong className="w-count">{catalog ? number(catalog.looksCount) : "—"}</strong>
            <h3>{t.inside.looks.title}</h3>
            <p>{t.inside.looks.text}</p>
            <ul className="w-chips">
              {t.inside.looks.chips.map((chip) => (
                <li key={chip}>{chip}</li>
              ))}
            </ul>
            <small className="w-credit">{t.inside.credit}</small>
          </div>
        </article>
        <article className="w-tile reveal" data-spotlight id="mods">
          <div className="w-fan is-game" aria-hidden="true">
            {tuning.map((src, index) => (
              <img key={src} src={src} alt="" loading="lazy" style={{ ["--i" as string]: index }} />
            ))}
          </div>
          <div className="w-tile-copy">
            <strong className="w-count">{catalog ? catalog.tuningCount : "—"}</strong>
            <h3>{t.inside.tuning.title}</h3>
            <p>{t.inside.tuning.text}</p>
            <details className="w-mod-list">
              <summary>
                {t.inside.tuning.all}
                <ChevronDown size={15} />
              </summary>
              {groups.map((group) => (
                <div className="mod-group" key={group.id}>
                  <h4>{group.title}</h4>
                  <ul>
                    {group.names.map((name) => (
                      <li key={name}>{name}</li>
                    ))}
                  </ul>
                </div>
              ))}
            </details>
          </div>
        </article>
      </div>
    </section>
  );
}

function How({ t }: { t: Copy }) {
  const icons = [<PenLine key="pick" />, <Wand2 key="check" />, <RotateCcw key="back" />];
  return (
    <section className="w-section" id="how">
      <header className="w-section-head reveal">
        <span className="s-eyebrow">{t.how.tag}</span>
        <h2>
          {t.how.a}
          <span>{t.how.b}</span>
        </h2>
      </header>
      <ol className="w-steps-row">
        {t.how.steps.map((step, index) => (
          <li key={step.title} className="w-step reveal" style={{ ["--i" as string]: index }}>
            <span className="w-step-mark">
              {icons[index]}
              <b>{String(index + 1).padStart(2, "0")}</b>
            </span>
            <h3>{step.title}</h3>
            <p>{step.text}</p>
          </li>
        ))}
      </ol>
    </section>
  );
}

function Safety({ t }: { t: Copy }) {
  return (
    <section className="w-section" id="safety">
      <header className="w-section-head reveal">
        <span className="s-eyebrow">
          <ShieldCheck />
          {t.safety.tag}
        </span>
        <h2>
          {t.safety.a}
          <span>{t.safety.b}</span>
        </h2>
        <p>{t.safety.body}</p>
      </header>
      <div className="w-safety">
        {t.safety.cards.map((card) => (
          <article key={card.id} className={`safety-card is-${card.id} reveal`}>
            <header>
              <h3>{card.title}</h3>
              <span>{card.chip}</span>
            </header>
            <ul>
              {card.items.map((item) => (
                <li key={item}>{item}</li>
              ))}
            </ul>
          </article>
        ))}
      </div>
      <p className="w-footnote reveal">
        <RotateCcw size={15} />
        {t.safety.rollback}
      </p>
    </section>
  );
}

function Status({ t }: { t: Copy }) {
  return (
    <section className="w-section" id="status">
      <header className="w-section-head reveal">
        <span className="s-eyebrow">{t.status.tag}</span>
        <h2>
          {t.status.a}
          <span>{t.status.b}</span>
        </h2>
        <p>{t.status.body}</p>
      </header>
      <div className="w-status reveal">
        <div>
          <h3>
            <Check size={16} />
            {t.status.seen}
          </h3>
          <ul className="console-list">
            {t.status.seenItems.map(([tag, text]) => (
              <li key={tag} className="is-ok">
                <code>{tag}</code>
                <span>{text}</span>
              </li>
            ))}
          </ul>
        </div>
        <div>
          <h3>
            <CircleDashed size={16} />
            {t.status.open}
          </h3>
          <ul className="console-list">
            {t.status.openItems.map((text) => (
              <li key={text} className="is-open">
                <CircleDashed size={14} />
                <span>{text}</span>
              </li>
            ))}
          </ul>
        </div>
      </div>
    </section>
  );
}

function Faq({ t }: { t: Copy }) {
  const [open, setOpen] = useState<number | null>(0);
  return (
    <section className="w-section" id="faq">
      <header className="w-section-head reveal">
        <span className="s-eyebrow">{t.faq.tag}</span>
        <h2>{t.faq.a}</h2>
      </header>
      <div className="w-faq reveal">
        {t.faq.items.map((item, index) => {
          const expanded = open === index;
          return (
            <div className={`faq-item ${expanded ? "is-open" : ""}`} key={item.q}>
              <button
                aria-expanded={expanded}
                aria-controls={`faq-${index}`}
                onClick={() => setOpen(expanded ? null : index)}
              >
                {item.q}
                <ChevronDown size={18} />
              </button>
              <div id={`faq-${index}`} role="region" hidden={!expanded}>
                <p>{item.a}</p>
                {item.more && (
                  <a href={`${import.meta.env.BASE_URL}${item.more.href}`}>
                    {item.more.label}
                    <ArrowUpRight size={13} />
                  </a>
                )}
              </div>
            </div>
          );
        })}
      </div>
    </section>
  );
}

export default function Site() {
  const [theme, setTheme] = useState<Theme>(initialTheme);
  const [language, setLanguage] = useState<Language>(initialLanguage);
  const [profile, setProfile] = useState(false);
  const [download, setDownload] = useState(false);
  const [mobileMenu, setMobileMenu] = useState(false);
  const [preview, setPreview] = useState<StudioMod | null>(null);
  const [authStage, setAuthStage] = useState<AuthStage>("checking");
  const [authSession, setAuthSession] = useState<WebSession | null>(null);
  const [sessionToken, setSessionToken] = useState<string | null>(null);
  const [authCode, setAuthCode] = useState("");
  const [authError, setAuthError] = useState("");
  const [storageWarning, setStorageWarning] = useState(false);
  const [avatarUrl, setAvatarUrl] = useState<string | null>(null);
  const [devices, setDevices] = useState<DeviceSession[]>([]);
  const [downloadError, setDownloadError] = useState("");
  const [release, setRelease] = useState<ReleaseMeta | null>(null);
  const [releaseState, setReleaseState] = useState<ReleaseState>("idle");
  const [releaseAttempt, setReleaseAttempt] = useState(0);
  const returnFocus = useRef<HTMLElement | null>(null);
  const catalog = useCatalog();
  const t = copy[language];
  const modalOpen = profile || download || Boolean(preview);
  const setLanguageStable = useCallback((next: Language) => setLanguage(next), []);

  useEffect(() => {
    const stored = readStorage("session", sessionKey);
    if (stored) setSessionToken(stored);
    else setAuthStage("idle");
  }, []);
  useEffect(() => {
    const root = document.documentElement;
    root.dataset.theme = theme;
    root.style.colorScheme = theme;
    document.querySelector('meta[name="theme-color"]')?.setAttribute("content", theme === "light" ? "#f7f5fa" : "#111116");
  }, [theme]);
  useEffect(() => {
    document.documentElement.lang = language;
    document.title = t.meta.title;
    const setMeta = (selector: string, value: string) => document.querySelector(selector)?.setAttribute("content", value);
    setMeta('meta[name="description"]', t.meta.description);
    setMeta('meta[property="og:title"]', t.meta.title);
    setMeta('meta[name="twitter:title"]', t.meta.title);
    setMeta('meta[property="og:description"]', t.meta.short);
    setMeta('meta[name="twitter:description"]', t.meta.short);
    setMeta('meta[property="og:locale"]', language === "ru" ? "ru_RU" : "en_US");
    try {
      localStorage.setItem(languageKey, language);
    } catch {
      // The choice lives for this visit only.
    }
    try {
      const url = new URL(window.location.href);
      if (url.searchParams.has("lang")) {
        url.searchParams.set("lang", language);
        window.history.replaceState(null, "", url);
      }
    } catch {
      // History is optional.
    }
  }, [language, t]);

  // Dialogs: the page behind goes inert, Tab stays inside, Escape closes and
  // focus returns to what opened the dialog.
  useEffect(() => {
    if (!modalOpen) return undefined;
    returnFocus.current = document.activeElement instanceof HTMLElement ? document.activeElement : null;
    const background = [document.querySelector<HTMLElement>(".site-header"), document.querySelector<HTMLElement>("main")].filter(
      (node): node is HTMLElement => Boolean(node),
    );
    background.forEach((node) => {
      node.inert = true;
      node.setAttribute("aria-hidden", "true");
    });
    document.body.style.overflow = "hidden";
    const onKey = (event: KeyboardEvent) => {
      if (event.key === "Escape") {
        event.preventDefault();
        setProfile(false);
        setDownload(false);
        setPreview(null);
        return;
      }
      if (event.key !== "Tab") return;
      const dialog = document.querySelector<HTMLElement>('[role="dialog"]');
      if (!dialog) return;
      const focusable = Array.from(
        dialog.querySelectorAll<HTMLElement>('a[href],button:not([disabled]),input:not([disabled]),summary,[tabindex]:not([tabindex="-1"])'),
      ).filter((node) => node.offsetParent !== null);
      if (!focusable.length) {
        event.preventDefault();
        dialog.focus();
        return;
      }
      const first = focusable[0];
      const last = focusable[focusable.length - 1];
      if (event.shiftKey && document.activeElement === first) {
        event.preventDefault();
        last.focus();
      } else if (!event.shiftKey && document.activeElement === last) {
        event.preventDefault();
        first.focus();
      }
    };
    window.addEventListener("keydown", onKey);
    return () => {
      window.removeEventListener("keydown", onKey);
      background.forEach((node) => {
        node.inert = false;
        node.removeAttribute("aria-hidden");
      });
      document.body.style.overflow = "";
      returnFocus.current?.focus();
      returnFocus.current = null;
    };
  }, [modalOpen]);
  useEffect(() => {
    if (!modalOpen) return undefined;
    const frame = requestAnimationFrame(() => document.querySelector<HTMLElement>('[role="dialog"] .modal-close')?.focus());
    return () => cancelAnimationFrame(frame);
  }, [modalOpen, profile, download, preview]);
  useEffect(() => {
    if (!mobileMenu) return undefined;
    const closeMenu = (event: KeyboardEvent) => {
      if (event.key !== "Escape") return;
      event.preventDefault();
      setMobileMenu(false);
      requestAnimationFrame(() => document.querySelector<HTMLElement>(".menu-button")?.focus());
    };
    window.addEventListener("keydown", closeMenu);
    return () => window.removeEventListener("keydown", closeMenu);
  }, [mobileMenu]);
  // Sections rise in as they enter the view; with reduced motion or without
  // IntersectionObserver they are simply shown. Re-run on language change and
  // once the catalog arrives, since both re-key nodes.
  useEffect(() => {
    const nodes = Array.from(document.querySelectorAll<HTMLElement>(".reveal:not(.is-visible)"));
    if (reducedMotion() || typeof IntersectionObserver === "undefined") {
      nodes.forEach((node) => node.classList.add("is-visible"));
      return undefined;
    }
    const observer = new IntersectionObserver(
      (entries) =>
        entries.forEach((entry) => {
          if (!entry.isIntersecting) return;
          entry.target.classList.add("is-visible");
          observer.unobserve(entry.target);
        }),
      { threshold: 0.12, rootMargin: "0px 0px -7%" },
    );
    nodes.forEach((node) => observer.observe(node));
    return () => observer.disconnect();
  }, [language, catalog]);

  useEffect(() => {
    if (!sessionToken) {
      setAuthSession(null);
      setAuthStage("idle");
      return undefined;
    }
    const controller = new AbortController();
    setAuthStage("checking");
    fetch(new URL("/v1/session/profile", authUrl), {
      headers: { Authorization: `Bearer ${sessionToken}` },
      credentials: "omit",
      signal: controller.signal,
    })
      .then(async (response) => {
        if (response.status === 401 || response.status === 403) {
          removeSession(sessionKey);
          setSessionToken(null);
          setAuthSession(null);
          setAuthStage("idle");
          return null;
        }
        if (!response.ok) throw new Error("profile_unavailable");
        return response.json() as Promise<WebSession>;
      })
      .then((data) => {
        if (!data) return;
        setAuthSession(data);
        setAuthStage("ready");
      })
      .catch((error) => {
        if (error instanceof DOMException && error.name === "AbortError") return;
        setAuthStage("unavailable");
      });
    return () => controller.abort();
  }, [sessionToken]);
  useEffect(() => {
    if (!sessionToken || !authSession?.avatarAvailable) {
      setAvatarUrl(null);
      return undefined;
    }
    const controller = new AbortController();
    let objectUrl: string | null = null;
    fetch(new URL("/v1/session/avatar", authUrl), {
      headers: { Authorization: `Bearer ${sessionToken}` },
      credentials: "omit",
      signal: controller.signal,
    })
      .then((response) => (response.ok ? response.blob() : Promise.reject()))
      .then((blob) => {
        objectUrl = URL.createObjectURL(blob);
        setAvatarUrl(objectUrl);
      })
      .catch(() => setAvatarUrl(null));
    return () => {
      controller.abort();
      if (objectUrl) URL.revokeObjectURL(objectUrl);
    };
  }, [sessionToken, authSession?.avatarAvailable]);
  useEffect(() => {
    if (!profile || !sessionToken || !authSession) return undefined;
    const controller = new AbortController();
    fetch(new URL("/v1/session/devices", authUrl), {
      headers: { Authorization: `Bearer ${sessionToken}` },
      credentials: "omit",
      signal: controller.signal,
    })
      .then((response) => (response.ok ? response.json() : Promise.reject()))
      .then((data: { sessions?: DeviceSession[] }) => setDevices(Array.isArray(data.sessions) ? data.sessions : []))
      .catch(() => setDevices([]));
    return () => controller.abort();
  }, [profile, sessionToken, authSession]);
  useEffect(() => {
    if (!download) return undefined;
    const controller = new AbortController();
    setReleaseState("loading");
    fetch(releasesUrl, { headers: { Accept: "application/vnd.github+json" }, signal: controller.signal })
      .then(async (response) => {
        if (!response.ok) throw new Error("release_unavailable");
        return response.json();
      })
      .then((data) => {
        const picked = Array.isArray(data) ? pickRelease(data) : null;
        if (!picked) {
          setReleaseState("missing");
          return;
        }
        setRelease(picked);
        setReleaseState("ready");
      })
      .catch(() => {
        if (!controller.signal.aborted) setReleaseState("unavailable");
      });
    return () => controller.abort();
  }, [download, releaseAttempt]);

  const signedIn = (data: WebSession & { sessionToken: string }) => {
    setStorageWarning(!writeSession(sessionKey, data.sessionToken));
    setSessionToken(data.sessionToken);
    setAuthSession(data);
    setAuthStage("ready");
  };
  const verifyCode = async () => {
    if (!/^\d{6}$/.test(authCode)) {
      setAuthError(t.auth.invalid);
      return;
    }
    setAuthStage("verifying");
    setAuthError("");
    try {
      const response = await fetch(new URL("/v1/auth/telegram/code", authUrl), {
        method: "POST",
        headers: { "content-type": "application/json" },
        body: JSON.stringify({ code: authCode, clientKind: "web" }),
        credentials: "omit",
      });
      if (!response.ok) throw new Error("invalid_code");
      const data = (await response.json()) as WebSession & { sessionToken?: string };
      if (!data.sessionToken || !data.displayName) throw new Error("invalid_response");
      signedIn(data as WebSession & { sessionToken: string });
      setAuthCode("");
    } catch (error) {
      setAuthStage("code");
      setAuthError(error instanceof TypeError ? t.auth.offline : t.auth.invalid);
    }
  };
  const logout = () => {
    const token = sessionToken;
    removeSession(sessionKey);
    setStorageWarning(false);
    setSessionToken(null);
    setAuthSession(null);
    setAvatarUrl(null);
    setDevices([]);
    setAuthStage("idle");
    if (token)
      fetch(new URL("/v1/session/logout", authUrl), {
        method: "POST",
        headers: { Authorization: `Bearer ${token}` },
        credentials: "omit",
      }).catch(() => undefined);
  };
  const revoke = async (item: DeviceSession) => {
    if (!sessionToken) return;
    try {
      const response = await fetch(new URL("/v1/session/devices/revoke", authUrl), {
        method: "POST",
        headers: { Authorization: `Bearer ${sessionToken}`, "content-type": "application/json" },
        body: JSON.stringify({ sessionId: item.sessionId }),
        credentials: "omit",
      });
      if (!response.ok) throw new Error();
      if (item.current) logout();
      else setDevices((value) => value.filter((other) => other.sessionId !== item.sessionId));
    } catch {
      // The list stays as it was.
    }
  };
  const downloadBuild = async () => {
    setDownloadError("");
    if (!sessionToken || !authSession) {
      setDownload(false);
      setProfile(true);
      return;
    }
    try {
      const response = await fetch(new URL("/v1/releases/latest", authUrl), {
        headers: { Authorization: `Bearer ${sessionToken}` },
        credentials: "omit",
      });
      if (!response.ok) throw new Error();
      const data = (await response.json()) as { downloadUrl?: string };
      const url = safeReleaseUrl(data.downloadUrl);
      if (!url) throw new Error();
      window.location.assign(url);
    } catch {
      setDownloadError(t.download.error);
    }
  };
  const toggleTheme = () => {
    const next: Theme = theme === "dark" ? "light" : "dark";
    setTheme(next);
    try {
      localStorage.setItem(themeKey, next);
    } catch {
      // The choice lives for this visit only.
    }
  };

  const nav = (
    <nav className="site-nav" aria-label={t.menu}>
      {t.nav.map(([id, label]) => (
        <a key={id} href={`#${id}`} onClick={() => setMobileMenu(false)}>
          {label}
        </a>
      ))}
    </nav>
  );

  return (
    <LocaleBridge language={language} setLanguage={setLanguageStable}>
      <a className="skip-link" href="#content">
        {t.skip}
      </a>
      <header className="site-header">
        <a href="#top" className="w-brand" aria-label="BetterFy">
          <Wordmark />
        </a>
        {nav}
        <div className="w-header-actions">
          <div className="w-lang" role="group" aria-label="Language">
            {(["ru", "en"] as const).map((value) => (
              <button
                key={value}
                className={language === value ? "active" : ""}
                aria-pressed={language === value}
                onClick={() => setLanguage(value)}
              >
                {value.toUpperCase()}
              </button>
            ))}
          </div>
          <button className="w-icon-button" onClick={toggleTheme} aria-label={theme === "dark" ? t.toLight : t.toDark}>
            {theme === "dark" ? <Sun size={16} /> : <Moon size={16} />}
          </button>
          <button className="w-profile-button" aria-label={t.profile} onClick={() => setProfile(true)}>
            {avatarUrl ? <img src={avatarUrl} alt="" /> : <CircleUserRound size={16} />}
            <span>{t.profile}</span>
          </button>
          <button
            className="s-btn s-btn-primary w-header-download"
            aria-label={t.hero.download}
            onClick={() => setDownload(true)}
          >
            <ArrowDownToLine />
            <span>{t.hero.download}</span>
          </button>
          <button
            className="w-icon-button menu-button"
            aria-label={t.menu}
            aria-expanded={mobileMenu}
            aria-controls="mobile-navigation"
            onClick={() => setMobileMenu((value) => !value)}
          >
            {mobileMenu ? <X size={18} /> : <Menu size={18} />}
          </button>
        </div>
        {mobileMenu && (
          <div className="w-mobile-nav" id="mobile-navigation">
            {nav}
          </div>
        )}
      </header>

      <main id="content">
        <Hero t={t} catalog={catalog} onDownload={() => setDownload(true)} onOpen={setPreview} />
        <div className="w-page">
          <Inside t={t} language={language} catalog={catalog} />
          <How t={t} />
          <Safety t={t} />
          <Status t={t} />
          <div className="w-bot reveal">
            <BotStage motion />
          </div>
          <Faq t={t} />
          <section className="w-download-cta reveal" id="download" data-spotlight>
            <div>
              <span className="s-eyebrow">{t.download.tag}</span>
              <h2>
                {t.download.a}
                <span>{t.download.b}</span>
              </h2>
              <p>{t.download.body}</p>
            </div>
            <button className="s-btn s-btn-primary" onClick={() => setDownload(true)}>
              <ArrowDownToLine />
              {t.hero.download}
            </button>
          </section>
        </div>
        <footer className="site-footer">
          <div className="w-footer-brand">
            <Wordmark />
            <p>{t.footer.tag}</p>
          </div>
          <div className="w-footer-links">
            <div>
              <h3>{t.footer.product}</h3>
              <a href="#inside">{t.nav[0][1]}</a>
              <a href={`${repo}/releases`}>{t.footer.releases}</a>
              <a href={`${repo}/blob/main/docs/ROADMAP.md`}>{t.footer.roadmap}</a>
            </div>
            <div>
              <h3>{t.footer.help}</h3>
              <a href="https://t.me/BeterFyBot">{t.footer.telegram}</a>
              <a href="https://t.me/BeterHelp">{t.footer.support}</a>
              <a href={`${repo}/issues`}>{t.footer.bug}</a>
            </div>
            <div>
              <h3>{t.footer.project}</h3>
              <a href={repo}>
                <Github size={13} /> GitHub
              </a>
              <a href={`${repo}/security`}>{t.footer.security}</a>
              <a href={`${import.meta.env.BASE_URL}privacy.html`}>{t.footer.privacy}</a>
              <a href={`${import.meta.env.BASE_URL}terms.html`}>{t.footer.terms}</a>
            </div>
          </div>
          <p className="w-legal">{t.footer.legal}</p>
        </footer>
      </main>

      {profile && (
        <Account
          language={language}
          t={t}
          close={() => setProfile(false)}
          stage={authStage}
          session={authSession}
          token={sessionToken}
          avatar={avatarUrl}
          sessions={devices}
          storageWarning={storageWarning}
          code={authCode}
          setCode={(value) => {
            setAuthCode(value);
            setAuthError("");
          }}
          codeError={authError}
          verifyCode={() => void verifyCode()}
          onSignedIn={signedIn}
          revoke={(item) => void revoke(item)}
          logout={logout}
        />
      )}
      {download && (
        <Download
          language={language}
          t={t}
          close={() => setDownload(false)}
          session={authSession}
          release={release}
          releaseState={releaseState}
          retry={() => setReleaseAttempt((value) => value + 1)}
          openProfile={() => {
            setDownload(false);
            setProfile(true);
          }}
          download={() => void downloadBuild()}
          error={downloadError}
        />
      )}
      {preview && (
        <div className="modal-backdrop" onMouseDown={() => setPreview(null)}>
          <section
            className="w-dialog w-preview"
            role="dialog"
            aria-modal="true"
            aria-labelledby="w-preview-title"
            onMouseDown={(event) => event.stopPropagation()}
          >
            <button className="modal-close" onClick={() => setPreview(null)} aria-label={t.close}>
              <X size={17} />
            </button>
            {preview.image && <img src={preview.image} alt="" />}
            <div>
              <p className="w-micro">{preview.categoryName[language]}</p>
              <h2 id="w-preview-title">{preview.name[language]}</h2>
              <p className="w-credit">
                <Eye size={13} />
                {t.inside.credit}
              </p>
              <button
                className="s-btn s-btn-primary"
                onClick={() => {
                  setPreview(null);
                  setDownload(true);
                }}
              >
                <ArrowDownToLine />
                {t.hero.download}
                <ArrowRight />
              </button>
            </div>
          </section>
        </div>
      )}
    </LocaleBridge>
  );
}
