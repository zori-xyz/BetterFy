import { useEffect, useState, type FormEvent } from "react";
import {
  ArrowLeft,
  ArrowRight,
  Check,
  ExternalLink,
  Globe2,
  LoaderCircle,
  LockKeyhole,
  MessageCircle,
  RefreshCcw,
  ShieldCheck,
} from "lucide-react";
import BetterFyWordmark from "./BetterFyWordmark";
import AccentTitle from "./AccentTitle";
import witchDoctorAuth from "./assets/witch-doctor-auth.png";
import {
  authMode,
  beginTelegramDeviceChallenge,
  cancelTelegramDeviceChallenge,
  pollTelegramDeviceChallenge,
  supportsDeviceChallenge,
  verifyTelegramCode,
  type AuthSession,
} from "./auth";
import { useLocale, type Language } from "./i18n";
import { openUrl } from "@tauri-apps/plugin-opener";

type Stage = "login" | "awaiting" | "code" | "checking" | "confirmed";

const copy = {
  ru: {
    brandLine: "DOTA 2 · MOD PLATFORM",
    sceneSlides: [
      {
        title: "Твоя игра.\nТвои правила.",
        text: "Собирай образы, эффекты и моды в одном спокойном пространстве.",
        label: "WITCH DOCTOR / ACCESS",
      },
      {
        title: "Собирай стиль.\nИграй по-своему.",
        text: "Один профиль для модов, образов и будущих сборок BetterFy.",
        label: "YOUR DOTA / YOUR STYLE",
      },
      {
        title: "Меньше рутины.\nБольше игры.",
        text: "Выбирай нужное — техническую часть BetterFy постепенно берёт на себя.",
        label: "BETTERFY / EARLY ACCESS",
      },
    ],
    access: "РАННИЙ ДОСТУП",
    title: "Войди через Telegram",
    intro: "Это единственный способ входа. BetterFy получает только данные, необходимые для профиля, и не видит твои переписки.",
    bot: "Открыть BetterFy Bot",
    botNote: "Подтверди вход в Telegram — код вводить не нужно",
    web: "Продолжить через веб-сайт",
    webNote: "Тот же Telegram-вход откроется в браузере",
    haveCode: "У меня уже есть код",
    privacy: "Запрос действует 10 минут и погашается только на этом устройстве.",
    awaitingStep: "TELEGRAM / CONFIRM",
    awaiting: "Подтверди вход в Telegram",
    awaitingText: "Мы уже открыли @BeterFyBot. Проверь запрос и нажми «Подтвердить вход» — BetterFy продолжит сам.",
    awaitingStatus: "Ждём твоего решения в боте",
    openAgain: "Открыть Telegram ещё раз",
    cancelRequest: "Отменить запрос",
    challengeDenied: "Запрос отклонён. Можно начать новый или войти по коду.",
    challengeExpired: "Запрос истёк. Открой бота ещё раз или используй шестизначный код.",
    challengeFailed: "Не удалось создать запрос. Проверь соединение или войди по коду.",
    codeStep: "TELEGRAM / CODE",
    codeTitle: "Введи код из бота",
    codeText: "Шесть цифр из сообщения @BeterFyBot.",
    codeLabel: "Одноразовый код",
    back: "Назад",
    confirm: "Подтвердить",
    resend: "Запросить новый код",
    demo: "Демо-режим: подойдёт любой шестизначный код, кроме 000000.",
    codeHint: "Код действует 10 минут и срабатывает один раз.",
    wrong: "Код недействителен или уже использован.",
    checking: "Проверяем код",
    checkingText: "BetterFy сверяет одноразовый код и готовит защищённую сессию этого устройства.",
    verifiedStep: "Telegram подтвердил запрос",
    confirmed: "Код подтверждён",
    confirmedText: "Доступ BetterFy открыт. Переходим к подключению Dota 2.",
  },
  en: {
    brandLine: "DOTA 2 · MOD PLATFORM",
    sceneSlides: [
      {
        title: "Your game.\nYour rules.",
        text: "Bring cosmetics, effects, and mods together in one calm space.",
        label: "WITCH DOCTOR / ACCESS",
      },
      {
        title: "Build the look.\nPlay your way.",
        text: "One profile for mods, cosmetics, and future BetterFy builds.",
        label: "YOUR DOTA / YOUR STYLE",
      },
      {
        title: "Less routine.\nMore game.",
        text: "Choose what you want while BetterFy gradually takes care of the technical path.",
        label: "BETTERFY / EARLY ACCESS",
      },
    ],
    access: "EARLY ACCESS",
    title: "Sign in with Telegram",
    intro: "This is the only sign-in method. BetterFy only receives profile essentials and cannot access your chats.",
    bot: "Open BetterFy Bot",
    botNote: "Approve in Telegram — no code entry needed",
    web: "Continue on the website",
    webNote: "The same Telegram flow opens in your browser",
    haveCode: "I already have a code",
    privacy: "The request lasts 10 minutes and can only be redeemed by this device.",
    awaitingStep: "TELEGRAM / CONFIRM",
    awaiting: "Approve the sign-in in Telegram",
    awaitingText: "We opened @BeterFyBot. Review the request and tap “Approve sign-in” — BetterFy will continue automatically.",
    awaitingStatus: "Waiting for your decision in the bot",
    openAgain: "Open Telegram again",
    cancelRequest: "Cancel request",
    challengeDenied: "The request was denied. Start a new one or use a code.",
    challengeExpired: "The request expired. Open the bot again or use a six-digit code.",
    challengeFailed: "The request could not be created. Check your connection or use a code.",
    codeStep: "TELEGRAM / CODE",
    codeTitle: "Enter the bot code",
    codeText: "Six digits from the @BeterFyBot message.",
    codeLabel: "One-time code",
    back: "Back",
    confirm: "Confirm",
    resend: "Request a new code",
    demo: "Demo mode: any six-digit code except 000000 works.",
    codeHint: "The code lasts 10 minutes and works once.",
    wrong: "The code is invalid or has already been used.",
    checking: "Checking your code",
    checkingText: "BetterFy verifies the one-time code and prepares this device session.",
    verifiedStep: "Telegram approved the request",
    confirmed: "Code confirmed",
    confirmedText: "BetterFy access is ready. Moving on to connect Dota 2.",
  },
} satisfies Record<Language, any>;

export default function AuthFlow({ onComplete }: { onComplete: (session: AuthSession) => void }) {
  const { language, setLanguage } = useLocale();
  const t = copy[language];
  const [stage, setStage] = useState<Stage>("login");
  const [code, setCode] = useState("");
  const [error, setError] = useState(false);
  const [startingChallenge, setStartingChallenge] = useState(false);
  const [challengeLink, setChallengeLink] = useState<string | null>(null);
  const [challengePollMs, setChallengePollMs] = useState(2000);
  const [challengeExpiresAt, setChallengeExpiresAt] = useState(0);
  const [challengeMessage, setChallengeMessage] = useState<string | null>(null);
  const [sceneIndex, setSceneIndex] = useState(0);
  const scene = t.sceneSlides[sceneIndex % t.sceneSlides.length];

  useEffect(() => {
    const timer = window.setInterval(
      () => setSceneIndex((current) => (current + 1) % t.sceneSlides.length),
      9600,
    );
    return () => window.clearInterval(timer);
  }, [t.sceneSlides.length]);

  const openCode = () => {
    void cancelTelegramDeviceChallenge();
    setError(false);
    setCode("");
    setStage("code");
  };

  const startTelegram = async () => {
    setChallengeMessage(null);
    if (!supportsDeviceChallenge()) {
      window.open("https://t.me/BeterFyBot", "_blank", "noopener,noreferrer");
      openCode();
      return;
    }
    setStartingChallenge(true);
    try {
      const challenge = await beginTelegramDeviceChallenge();
      setChallengeLink(challenge.deepLink);
      setChallengePollMs(challenge.pollAfterSeconds * 1000);
      setChallengeExpiresAt(challenge.expiresAt);
      await openUrl(challenge.deepLink);
      setStage("awaiting");
    } catch {
      void cancelTelegramDeviceChallenge();
      setChallengeMessage(t.challengeFailed);
    } finally {
      setStartingChallenge(false);
    }
  };

  const reopenTelegram = async () => {
    if (!challengeLink) return;
    if (supportsDeviceChallenge()) await openUrl(challengeLink);
    else window.open(challengeLink, "_blank", "noopener,noreferrer");
  };

  useEffect(() => {
    if (stage !== "awaiting") return undefined;
    let cancelled = false;
    let timer = 0;
    const poll = async () => {
      if (cancelled) return;
      if (Math.floor(Date.now() / 1000) >= challengeExpiresAt) {
        setChallengeMessage(t.challengeExpired);
        setStage("login");
        return;
      }
      try {
        const result = await pollTelegramDeviceChallenge();
        if (cancelled) return;
        if (result.state === "confirmed" && result.profile) {
          setStage("confirmed");
          await new Promise((resolve) => window.setTimeout(resolve, 1000));
          onComplete(result.profile);
          return;
        }
        if (result.state === "denied" || result.state === "expired") {
          setChallengeMessage(result.state === "denied" ? t.challengeDenied : t.challengeExpired);
          setStage("login");
          return;
        }
      } catch {
        // A temporary network failure does not destroy the local challenge.
      }
      timer = window.setTimeout(poll, challengePollMs);
    };
    timer = window.setTimeout(poll, challengePollMs);
    return () => {
      cancelled = true;
      window.clearTimeout(timer);
    };
  }, [challengeExpiresAt, challengePollMs, onComplete, stage, t.challengeDenied, t.challengeExpired]);

  const verify = async (event: FormEvent) => {
    event.preventDefault();
    if (code.length !== 6) return;
    setError(false);
    setStage("checking");
    try {
      const checkStartedAt = performance.now();
      const session = await verifyTelegramCode(code);
      const remainingCheckTime = Math.max(0, 800 - (performance.now() - checkStartedAt));
      if (remainingCheckTime > 0) {
        await new Promise((resolve) => window.setTimeout(resolve, remainingCheckTime));
      }
      setStage("confirmed");
      await new Promise((resolve) => window.setTimeout(resolve, 1150));
      onComplete(session);
    } catch {
      setError(true);
      setStage("code");
    }
  };

  return (
    <main className={`auth-stage auth-stage-${stage}`}>
      <header className="auth-header" data-tauri-drag-region>
        <div className="auth-brand">
          <BetterFyWordmark />
          <small>{t.brandLine}</small>
        </div>
        <div className="locale-switch">
          <button className={language === "ru" ? "active" : ""} onClick={() => setLanguage("ru")}>RU</button>
          <button className={language === "en" ? "active" : ""} onClick={() => setLanguage("en")}>EN</button>
        </div>
      </header>

      <section className="auth-scene" aria-hidden="true">
        <div className="scene-light" />
        <div className="scene-copy" key={`${language}-${sceneIndex}`}>
          <span>{scene.label}</span>
          <h2>{scene.title.split("\n").map((line: string) => <i key={line}>{line}</i>)}</h2>
          <p>{scene.text}</p>
        </div>
        <div className="auth-art-placeholder">
          <i className="auth-light-ribbon" />
          <img src={witchDoctorAuth} alt="" />
        </div>
      </section>

      <section className="auth-workspace">
        {stage === "login" && (
          <div className="auth-view view-enter">
            <span className="section-label">{t.access}</span>
            <h1 className="accent-title"><AccentTitle text={t.title} /></h1>
            <p>{t.intro}</p>

            {challengeMessage && <div className="auth-inline-error" role="alert"><LockKeyhole /><span>{challengeMessage}</span></div>}

            <div className="telegram-actions">
              <button
                type="button"
                className="telegram-primary"
                onClick={startTelegram}
                disabled={startingChallenge}
              >
                <span>{startingChallenge ? <LoaderCircle className="spin" /> : <MessageCircle />}</span>
                <span><strong>{t.bot}</strong><small>{t.botNote}</small></span>
                <ArrowRight />
              </button>
              <a
                className="telegram-web"
                href="https://zori-xyz.github.io/BetterFy/"
                target="_blank"
                rel="noreferrer"
              >
                <span><Globe2 /></span>
                <span><strong>{t.web}</strong><small>{t.webNote}</small></span>
                <ExternalLink />
              </a>
            </div>

            <div className="privacy-note"><ShieldCheck /><span>{t.privacy}</span></div>
            <button className="have-code-button" onClick={openCode}>{t.haveCode}<ArrowRight /></button>
          </div>
        )}

        {stage === "awaiting" && (
          <div className="auth-view awaiting-view view-enter" role="status" aria-live="polite">
            <div className="telegram-wait-mark"><MessageCircle /><i /></div>
            <span className="section-label">{t.awaitingStep}</span>
            <h1 className="accent-title"><AccentTitle text={t.awaiting} /></h1>
            <p>{t.awaitingText}</p>
            <div className="challenge-wait-status"><LoaderCircle className="spin" /><span>{t.awaitingStatus}</span></div>
            <div className="challenge-actions">
              {challengeLink && <button className="challenge-reopen" type="button" onClick={reopenTelegram}><MessageCircle />{t.openAgain}<ExternalLink /></button>}
              <button type="button" onClick={() => {
                void cancelTelegramDeviceChallenge();
                setStage("login");
              }}><ArrowLeft />{t.cancelRequest}</button>
            </div>
          </div>
        )}

        {stage === "code" && (
          <form className={`auth-view code-view view-enter ${error ? "is-error" : ""}`} onSubmit={verify}>
            <button className="back-button" type="button" onClick={() => setStage("login")}><ArrowLeft />{t.back}</button>
            <span className="section-label">{t.codeStep}</span>
            <h1 className="accent-title"><AccentTitle text={t.codeTitle} /></h1>
            <p>{t.codeText}</p>

            <label className="otp-field">
              <span>{t.codeLabel}</span>
              <input
                autoFocus
                autoComplete="one-time-code"
                inputMode="numeric"
                pattern="[0-9]*"
                value={code}
                onChange={(event) => {
                  setCode(event.target.value.replace(/\D/g, "").slice(0, 6));
                  setError(false);
                }}
                aria-invalid={error}
                aria-describedby="code-feedback"
              />
              <div className="otp-cells" aria-hidden="true">
                {Array.from({ length: 6 }, (_, index) => (
                  <i className={code[index] ? "filled" : index === code.length ? "current" : ""} key={index}>
                    {code[index] ?? ""}
                  </i>
                ))}
              </div>
            </label>

            <div id="code-feedback" className={`code-feedback ${error ? "error" : ""}`} role={error ? "alert" : "note"}>
              {error ? <LockKeyhole /> : <ShieldCheck />}
              <span>{error ? t.wrong : authMode === "demo" ? t.demo : t.codeHint}</span>
            </div>

            <button className="confirm-code" disabled={code.length !== 6}>
              <span>{t.confirm}</span><ArrowRight />
            </button>
            <button className="resend-code" type="button" onClick={() => { setCode(""); setError(false); }}>
              <RefreshCcw />{t.resend}
            </button>
          </form>
        )}

        {stage === "checking" && (
          <div className="auth-view checking-view view-enter" role="status" aria-live="polite">
            <div className="verification-orbit">
              <i /><i /><span><LoaderCircle /></span>
            </div>
            <span className="section-label">BETTERFY ID / VERIFY</span>
            <h1 className="accent-title"><AccentTitle text={t.checking} /></h1>
            <p>{t.checkingText}</p>
            <div className="verify-progress"><i /></div>
            <div className="verify-step"><Check />{t.verifiedStep}</div>
          </div>
        )}

        {stage === "confirmed" && (
          <div className="auth-view confirmed-view view-enter" role="status" aria-live="polite">
            <div className="verification-success"><Check /></div>
            <span className="section-label">BETTERFY ID / READY</span>
            <h1 className="accent-title"><AccentTitle text={t.confirmed} /></h1>
            <p>{t.confirmedText}</p>
          </div>
        )}
      </section>
    </main>
  );
}
