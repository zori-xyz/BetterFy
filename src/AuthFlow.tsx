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
import { verifyTelegramCode, type AuthSession } from "./auth";
import { useLocale, type Language } from "./i18n";

type Stage = "login" | "code" | "checking" | "confirmed";

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
    botNote: "Получить одноразовый код в Telegram",
    web: "Продолжить через веб-сайт",
    webNote: "Тот же Telegram-вход откроется в браузере",
    haveCode: "У меня уже есть код",
    privacy: "Код действует 10 минут и привязан к этому устройству.",
    codeStep: "TELEGRAM / CODE",
    codeTitle: "Введи код из бота",
    codeText: "Шесть цифр из сообщения @BeterFyBot.",
    codeLabel: "Одноразовый код",
    back: "Назад",
    confirm: "Подтвердить",
    resend: "Запросить новый код",
    demo: "Демонстрационный интерфейс: сейчас подойдёт любой шестизначный код, кроме 000000.",
    wrong: "Код недействителен или уже использован.",
    checking: "Проверяем код",
    checkingText: "В рабочей версии проверка будет выполняться BetterFy backend.",
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
    botNote: "Get a one-time code in Telegram",
    web: "Continue on the website",
    webNote: "The same Telegram flow opens in your browser",
    haveCode: "I already have a code",
    privacy: "The code lasts 10 minutes and is bound to this device.",
    codeStep: "TELEGRAM / CODE",
    codeTitle: "Enter the bot code",
    codeText: "Six digits from the @BeterFyBot message.",
    codeLabel: "One-time code",
    back: "Back",
    confirm: "Confirm",
    resend: "Request a new code",
    demo: "Interface demo: any six-digit code except 000000 works for now.",
    wrong: "The code is invalid or has already been used.",
    checking: "Checking your code",
    checkingText: "The production build will verify it through the BetterFy backend.",
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
    setError(false);
    setCode("");
    setStage("code");
  };

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

            <div className="telegram-actions">
              <a
                className="telegram-primary"
                href="https://t.me/BeterFyBot"
                target="_blank"
                rel="noreferrer"
                onClick={() => window.setTimeout(openCode, 0)}
              >
                <span><MessageCircle /></span>
                <span><strong>{t.bot}</strong><small>{t.botNote}</small></span>
                <ArrowRight />
              </a>
              <a
                className="telegram-web"
                href="https://t.me/BeterFyBot"
                target="_blank"
                rel="noreferrer"
                onClick={() => window.setTimeout(openCode, 0)}
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
              <span>{error ? t.wrong : t.demo}</span>
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
            <div className="verify-step"><Check />Telegram challenge received</div>
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
