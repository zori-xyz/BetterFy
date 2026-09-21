import { useEffect, useState, type FormEvent } from "react";
import {
  ArrowLeft,
  ArrowRight,
  Check,
  ExternalLink,
  Eye,
  EyeOff,
  Globe2,
  LoaderCircle,
  LockKeyhole,
  Mail,
  MessageCircle,
  RefreshCcw,
  Send,
  ShieldCheck,
} from "lucide-react";
import BetterFyWordmark from "./BetterFyWordmark";
import AccentTitle from "./AccentTitle";
import AuthAmbient from "./studio/AuthAmbient";
import {
  authMode,
  beginIdRegistration,
  beginEmailSignIn,
  beginTelegramDeviceChallenge,
  cancelTelegramDeviceChallenge,
  pollTelegramDeviceChallenge,
  signInWithId,
  supportsDeviceChallenge,
  verifyEmailSignIn,
  verifyIdRegistration,
  verifyTelegramCode,
  type AuthSession,
} from "./auth";
import { useLocale, type Language } from "./i18n";
import { openUrl } from "@tauri-apps/plugin-opener";

type Stage = "login" | "id-login" | "id-register" | "id-register-code" | "email" | "email-code" | "awaiting" | "code" | "checking" | "confirmed";

function IdSeal() {
  return <svg className="betterfy-id-seal" viewBox="0 0 82 52" aria-hidden="true" focusable="false"><path d="M24 2C10 5 2 14 2 26s8 21 22 24c-8-6-12-14-12-24S16 8 24 2Z"/><circle cx="41" cy="26" r="23"/><path d="M58 2c14 3 22 12 22 24s-8 21-22 24c8-6 12-14 12-24S66 8 58 2Z"/><text x="41" y="34" textAnchor="middle">ID</text></svg>;
}

function IdMark() {
  return <span className="betterfy-id-mark" aria-label="BetterFy ID"><BetterFyWordmark /><IdSeal /></span>;
}

const copy = {
  ru: {
    brandLine: "DOTA 2 · MOD PLATFORM",
    visualLabel: "ДОСТУП К BETTERFY",
    visualSteps: ["Telegram", "Это устройство", "Профиль BetterFy"],
    fastest: "Самый быстрый способ входа",
    access: "РАННИЙ ДОСТУП",
    titleLead: "Твой",
    titleName: "BetterFy",
    intro: "Свой профиль для приложения и сайта. Войди с паролем или подтверди вход одним нажатием в Telegram.",
    idTitle: "BetterFy ID",
    choiceTitle: "Выбери способ",
    idNote: "Почта или никнейм и пароль",
    telegramTitle: "Войти через Telegram",
    telegramNote: "Подтверждение в боте",
    choiceNote: "Выбери удобный способ входа.",
    idLoginTitle: "Войти в BetterFy ID",
    idRegisterTitle: "Создать BetterFy ID",
    idRegisterText: "Никнейм, почта и пароль. Подтвердим почту одноразовым кодом.",
    idIdentifier: "Почта или никнейм",
    idUsername: "Никнейм латиницей",
    idPassword: "Пароль",
    idPasswordHint: "Не меньше 12 символов",
    idLoginAction: "Войти",
    idRegisterAction: "Создать аккаунт",
    idNew: "Нет BetterFy ID?",
    idExisting: "Уже есть BetterFy ID?",
    idForgot: "Забыл пароль? Получить код на почту",
    idInvalid: "Проверь данные и попробуй ещё раз.",
    idUnavailable: "BetterFy ID сейчас недоступен. Попробуй позже или войди через Telegram.",
    idRegistrationCode: "Подтверди почту",
    idRegistrationCodeText: "Введи код из письма, чтобы закончить создание BetterFy ID.",
    emailStep: "BETTERFY ID",
    emailTitle: "Твоя почта. Твой вход.",
    emailText: "Отправим одноразовый код. Пароль не нужен.",
    emailLabel: "Адрес почты",
    emailPlaceholder: "name@example.com",
    emailContinue: "Получить код",
    emailFirst: "Почта ещё не привязана? Войди через Telegram и подключи её в профиле.",
    emailCodeTitle: "Проверь почту",
    emailCodeText: "Если эта почта подключена к BetterFy ID, письмо придёт в течение минуты. Код действует 10 минут.",
    emailConfirmed: "Вход выполнен",
    emailCodeLabel: "Код из письма",
    emailResend: "Отправить новый код",
    emailError: "Не удалось отправить код. Проверь адрес или попробуй Telegram.",
    emailVerifyError: "Код не подошёл или уже использован.",
    bot: "Открыть BetterFy Bot",
    telegramAction: "Открыть в Telegram",
    botNote: "Подтверди вход в Telegram — код вводить не нужно",
    botCardText: "Один переход — и BetterFy продолжит вход автоматически.",
    divider: "или",
    web: "Продолжить через веб-сайт",
    webNote: "Тот же Telegram-вход откроется в браузере",
    haveCode: "У меня уже есть код",
    preview: "Посмотреть приложение",
    previewNote: "Без входа и доступа к файлам Dota 2",
    footerLine: "Простые решения для Dota 2",
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
    challengePollFailed: "Не удаётся получить подтверждение на этом устройстве. Проверь соединение или начни вход заново.",
    challengeVaultFailed: "Telegram подтвердил вход, но Windows не сохранила сеанс. Попробуй снова и сообщи нам об этой ошибке.",
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
    visualLabel: "BETTERFY ACCESS",
    visualSteps: ["Telegram", "This device", "BetterFy profile"],
    fastest: "The fastest way to sign in",
    access: "EARLY ACCESS",
    titleLead: "Your",
    titleName: "BetterFy",
    intro: "Your profile for the app and website. Use a password or approve sign-in with one tap in Telegram.",
    idTitle: "BetterFy ID",
    choiceTitle: "Choose a method",
    idNote: "Email or username and password",
    telegramTitle: "Sign in with Telegram",
    telegramNote: "Approve in the bot",
    choiceNote: "Choose how you want to sign in.",
    idLoginTitle: "Sign in to BetterFy ID",
    idRegisterTitle: "Create BetterFy ID",
    idRegisterText: "Choose a username in Latin letters, email and password. Verify your email with a one-time code.",
    idIdentifier: "Email or username",
    idUsername: "Username",
    idPassword: "Password",
    idPasswordHint: "At least 12 characters",
    idLoginAction: "Sign in",
    idRegisterAction: "Create account",
    idNew: "No BetterFy ID yet?",
    idExisting: "Already have BetterFy ID?",
    idForgot: "Forgot password? Get an email code",
    idInvalid: "Check your details and try again.",
    idUnavailable: "BetterFy ID is unavailable right now. Try later or use Telegram.",
    idRegistrationCode: "Verify your email",
    idRegistrationCodeText: "Enter the code from your email to finish creating BetterFy ID.",
    emailStep: "BETTERFY ID",
    emailTitle: "Your email. Your sign-in.",
    emailText: "We will send a one-time code. No password needed.",
    emailLabel: "Email address",
    emailPlaceholder: "name@example.com",
    emailContinue: "Send a code",
    emailFirst: "Email not linked yet? Sign in with Telegram and connect it in your profile.",
    emailCodeTitle: "Check your email",
    emailCodeText: "If this email is linked to BetterFy ID, the message will arrive shortly. The code is valid for 10 minutes.",
    emailConfirmed: "Signed in",
    emailCodeLabel: "Email code",
    emailResend: "Send another code",
    emailError: "Could not send the code. Check the address or try Telegram.",
    emailVerifyError: "The code is invalid or already used.",
    bot: "Open BetterFy Bot",
    telegramAction: "Open in Telegram",
    botNote: "Approve in Telegram — no code entry needed",
    botCardText: "One handoff, then BetterFy continues automatically.",
    divider: "or",
    web: "Continue on the website",
    webNote: "The same Telegram flow opens in your browser",
    haveCode: "I already have a code",
    preview: "Explore the application",
    previewNote: "No sign-in and no access to Dota 2 files",
    footerLine: "Simple solutions for Dota 2",
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
    challengePollFailed: "This device cannot receive the confirmation. Check your connection or start sign-in again.",
    challengeVaultFailed: "Telegram approved sign-in, but Windows could not save the session. Try again and report this error.",
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

export default function AuthFlow({
  onComplete,
  onPreview,
}: {
  onComplete: (session: AuthSession) => void;
  onPreview: () => void;
}) {
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
  const [email, setEmail] = useState("");
  const [emailCode, setEmailCode] = useState("");
  const [emailBusy, setEmailBusy] = useState(false);
  const [emailMessage, setEmailMessage] = useState<string | null>(null);
  const [confirmedByEmail, setConfirmedByEmail] = useState(false);
  const [identifier, setIdentifier] = useState("");
  const [username, setUsername] = useState("");
  const [password, setPassword] = useState("");
  const [showPassword, setShowPassword] = useState(false);
  const [idBusy, setIdBusy] = useState(false);
  const [idMessage, setIdMessage] = useState<string | null>(null);
  const finishId = async (session: AuthSession) => {
    setPassword("");
    setConfirmedByEmail(true);
    setStage("confirmed");
    await new Promise((resolve) => window.setTimeout(resolve, 950));
    onComplete(session);
  };
  const loginId = async (event: FormEvent) => {
    event.preventDefault(); setIdBusy(true); setIdMessage(null);
    try { await finishId(await signInWithId(identifier, password)); }
    catch (cause) { setIdMessage(cause instanceof Error && cause.message === "auth_id_invalid_credentials" ? t.idInvalid : t.idUnavailable); }
    finally { setIdBusy(false); }
  };
  const registerId = async (event: FormEvent) => {
    event.preventDefault(); setIdBusy(true); setIdMessage(null);
    try { await beginIdRegistration(username, email, password, language); setPassword(""); setEmailCode(""); setStage("id-register-code"); }
    catch { setIdMessage(t.idUnavailable); }
    finally { setIdBusy(false); }
  };
  const confirmRegistration = async (event: FormEvent) => {
    event.preventDefault(); if (emailCode.length !== 6) return;
    setIdBusy(true); setIdMessage(null);
    try { await finishId(await verifyIdRegistration(email, emailCode)); }
    catch { setIdMessage(t.emailVerifyError); }
    finally { setIdBusy(false); }
  };
  const requestEmail = async (event?: FormEvent) => {
    event?.preventDefault();
    setEmailBusy(true);
    setEmailMessage(null);
    try {
      await beginEmailSignIn(email, language);
      setEmailCode("");
      setStage("email-code");
    } catch {
      setEmailMessage(t.emailError);
    } finally {
      setEmailBusy(false);
    }
  };

  const confirmEmail = async (event: FormEvent) => {
    event.preventDefault();
    if (emailCode.length !== 6) return;
    setEmailBusy(true);
    setEmailMessage(null);
    try {
      const session = await verifyEmailSignIn(email, emailCode);
      setConfirmedByEmail(true);
      setStage("confirmed");
      await new Promise((resolve) => window.setTimeout(resolve, 950));
      onComplete(session);
    } catch {
      setEmailMessage(t.emailVerifyError);
    } finally {
      setEmailBusy(false);
    }
  };
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
    let consecutiveFailures = 0;
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
        consecutiveFailures = 0;
        setChallengeMessage(null);
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
      } catch (cause) {
        if (++consecutiveFailures >= 2) {
          const code = cause instanceof Error ? cause.message : cause;
          setChallengeMessage(code === "auth_vault_unavailable" ? t.challengeVaultFailed : t.challengePollFailed);
        }
      }
      timer = window.setTimeout(poll, challengePollMs);
    };
    timer = window.setTimeout(poll, challengePollMs);
    return () => {
      cancelled = true;
      window.clearTimeout(timer);
    };
  }, [challengeExpiresAt, challengePollMs, onComplete, stage, t.challengeDenied, t.challengeExpired, t.challengePollFailed, t.challengeVaultFailed]);

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
      {stage === "login" && <><div className="auth-id-environment" aria-hidden="true" /><AuthAmbient /></>}
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

      <section className="auth-workspace">
        {stage === "login" && (
          <div className="auth-login-layout view-enter">
            <div className="auth-login-copy">
              <span className="section-label">{t.access}</span>
              <h1 className="auth-id-main-title"><span>{t.titleLead}</span><span>{t.titleName}<b>ID</b></span></h1>
              <p>{t.intro}</p>
              {challengeMessage && <div className="auth-inline-error" role="alert"><LockKeyhole /><span>{challengeMessage}</span></div>}
            </div>

            <div className="auth-login-card">
              <div className="auth-id-heading"><IdMark /></div>
              <h2>{t.choiceTitle}</h2>
              <p>{t.choiceNote}</p>
              <div className="auth-choice-actions">
                <button
                  type="button"
                  className="auth-id-choice"
                  onClick={() => { setIdMessage(null); setStage("id-login"); }}
                >
                  <span className="auth-choice-icon"><IdSeal /></span>
                  <span className="auth-choice-label"><strong>{t.idTitle}</strong><small>{t.idNote}</small></span>
                  <ArrowRight />
                </button>
                <button
                  type="button"
                  className="auth-telegram-choice"
                  onClick={startTelegram}
                  disabled={startingChallenge}
                >
                  <span className="auth-choice-icon">{startingChallenge ? <LoaderCircle className="spin" /> : <Send />}</span>
                  <span className="auth-choice-label"><strong>{t.telegramTitle}</strong><small>{t.telegramNote}</small></span>
                  <ArrowRight />
                </button>
              </div>
              <div className="auth-choice-fallback">
                <a href="https://zori-xyz.github.io/BetterFy/" target="_blank" rel="noreferrer"><Globe2 />{t.web}<ExternalLink /></a>
                <button type="button" onClick={openCode}>{t.haveCode}<ArrowRight /></button>
              </div>
            </div>

            <div className="auth-login-footer">
              <div className="auth-footer-note"><span>BetterFy</span><i />© 2026<i />{t.footerLine}</div>
              <button className="auth-preview-button" type="button" onClick={onPreview}>{t.preview}<ArrowRight /></button>
            </div>
          </div>
        )}

        {stage === "id-login" && (
          <form className="auth-view auth-email-view auth-id-form view-enter" onSubmit={loginId}>
            <button className="back-button" type="button" onClick={() => setStage("login")}><ArrowLeft />{t.back}</button>
            <IdMark />
            <h1>{t.idLoginTitle}</h1>
            <label className="auth-email-field" htmlFor="betterfy-id-identifier">{t.idIdentifier}</label>
            <input id="betterfy-id-identifier" autoFocus required autoComplete="username" maxLength={254} value={identifier} onChange={(event) => setIdentifier(event.target.value)} />
            <label className="auth-email-field" htmlFor="betterfy-id-password">{t.idPassword}</label>
            <div className="auth-password-field"><input id="betterfy-id-password" required minLength={12} maxLength={128} type={showPassword ? "text" : "password"} autoComplete="current-password" value={password} onChange={(event) => setPassword(event.target.value)} /><button type="button" onClick={() => setShowPassword((value) => !value)} aria-label={showPassword ? "Hide password" : "Show password"}>{showPassword ? <EyeOff /> : <Eye />}</button></div>
            <button className="auth-id-text-action" type="button" onClick={() => { setEmail(identifier.includes("@") ? identifier : ""); setStage("email"); }}>{t.idForgot}</button>
            {idMessage && <p className="auth-email-error" role="alert">{idMessage}</p>}
            <button className="auth-email-submit" type="submit" disabled={idBusy}>{idBusy ? <LoaderCircle className="spin" /> : <LockKeyhole />}{t.idLoginAction}<ArrowRight /></button>
            <div className="auth-id-form-footer"><span>{t.idNew}</span><button type="button" onClick={() => { setIdMessage(null); setStage("id-register"); }}>{t.idRegisterAction}<ArrowRight /></button></div>
            <button className="auth-email-alternative" type="button" onClick={startTelegram}><Send />{t.telegramTitle}<ArrowRight /></button>
          </form>
        )}

        {stage === "id-register" && (
          <form className="auth-view auth-email-view auth-id-form view-enter" onSubmit={registerId}>
            <button className="back-button" type="button" onClick={() => setStage("id-login")}><ArrowLeft />{t.back}</button>
            <IdMark />
            <h1>{t.idRegisterTitle}</h1><p>{t.idRegisterText}</p>
            <label className="auth-email-field" htmlFor="betterfy-id-username">{t.idUsername}</label>
            <input id="betterfy-id-username" autoFocus required minLength={3} maxLength={24} pattern="[A-Za-z][A-Za-z0-9_]{2,23}" autoComplete="username" value={username} onChange={(event) => setUsername(event.target.value)} />
            <label className="auth-email-field" htmlFor="betterfy-id-email">{t.emailLabel}</label>
            <input id="betterfy-id-email" required type="email" autoComplete="email" maxLength={254} value={email} onChange={(event) => setEmail(event.target.value)} />
            <label className="auth-email-field" htmlFor="betterfy-id-new-password">{t.idPassword} <small>{t.idPasswordHint}</small></label>
            <div className="auth-password-field"><input id="betterfy-id-new-password" required minLength={12} maxLength={128} type={showPassword ? "text" : "password"} autoComplete="new-password" value={password} onChange={(event) => setPassword(event.target.value)} /><button type="button" onClick={() => setShowPassword((value) => !value)} aria-label={showPassword ? "Hide password" : "Show password"}>{showPassword ? <EyeOff /> : <Eye />}</button></div>
            {idMessage && <p className="auth-email-error" role="alert">{idMessage}</p>}
            <button className="auth-email-submit" type="submit" disabled={idBusy}>{idBusy ? <LoaderCircle className="spin" /> : <Mail />}{t.idRegisterAction}<ArrowRight /></button>
            <div className="auth-id-form-footer"><span>{t.idExisting}</span><button type="button" onClick={() => setStage("id-login")}>{t.idLoginAction}<ArrowRight /></button></div>
          </form>
        )}

        {stage === "id-register-code" && (
          <form className="auth-view auth-email-view auth-id-form view-enter" onSubmit={confirmRegistration}>
            <button className="back-button" type="button" onClick={() => setStage("id-register")}><ArrowLeft />{t.back}</button>
            <IdMark /><h1>{t.idRegistrationCode}</h1><p>{t.idRegistrationCodeText}</p>
            <span className="auth-email-destination">{email}</span>
            <label className="auth-email-field" htmlFor="betterfy-id-registration-code">{t.emailCodeLabel}</label>
            <input id="betterfy-id-registration-code" className="auth-email-code-input" autoFocus autoComplete="one-time-code" inputMode="numeric" pattern="[0-9]*" maxLength={6} value={emailCode} onChange={(event) => setEmailCode(event.target.value.replace(/\D/g, "").slice(0, 6))} />
            {idMessage && <p className="auth-email-error" role="alert">{idMessage}</p>}
            <button className="auth-email-submit" type="submit" disabled={idBusy || emailCode.length !== 6}>{idBusy ? <LoaderCircle className="spin" /> : <ShieldCheck />}{t.confirm}<ArrowRight /></button>
          </form>
        )}

        {stage === "email" && (
          <form className="auth-view auth-email-view view-enter" onSubmit={requestEmail}>
            <button className="back-button" type="button" onClick={() => setStage("login")}><ArrowLeft />{t.back}</button>
            <div className="auth-id-symbol"><Mail /></div>
            <span className="section-label">{t.emailStep}</span>
            <h1>{t.emailTitle}</h1>
            <p>{t.emailText}</p>
            <label className="auth-email-field" htmlFor="betterfy-email">{t.emailLabel}</label>
            <input id="betterfy-email" type="email" inputMode="email" autoComplete="email" autoFocus required maxLength={254} placeholder={t.emailPlaceholder} value={email} onChange={(event) => setEmail(event.target.value)} />
            {emailMessage && <p className="auth-email-error" role="alert">{emailMessage}</p>}
            <button className="auth-email-submit" type="submit" disabled={emailBusy}>{emailBusy ? <LoaderCircle className="spin" /> : <Mail />}{t.emailContinue}<ArrowRight /></button>
            <p className="auth-email-help">{t.emailFirst}</p>
            <button className="auth-email-alternative" type="button" onClick={startTelegram}><Send />{t.telegramTitle}<ArrowRight /></button>
          </form>
        )}

        {stage === "email-code" && (
          <form className="auth-view auth-email-view view-enter" onSubmit={confirmEmail}>
            <button className="back-button" type="button" onClick={() => { setEmailMessage(null); setStage("email"); }}><ArrowLeft />{t.back}</button>
            <div className="auth-id-symbol"><Mail /></div>
            <span className="section-label">{t.emailStep}</span>
            <h1>{t.emailCodeTitle}</h1>
            <p>{t.emailCodeText}</p>
            <span className="auth-email-destination">{email}</span>
            <label className="auth-email-field" htmlFor="betterfy-email-code">{t.emailCodeLabel}</label>
            <input id="betterfy-email-code" className="auth-email-code-input" autoComplete="one-time-code" inputMode="numeric" pattern="[0-9]*" autoFocus maxLength={6} value={emailCode} onChange={(event) => { setEmailMessage(null); setEmailCode(event.target.value.replace(/\D/g, "").slice(0, 6)); }} aria-invalid={Boolean(emailMessage)} />
            {emailMessage && <p className="auth-email-error" role="alert">{emailMessage}</p>}
            <button className="auth-email-submit" type="submit" disabled={emailBusy || emailCode.length !== 6}>{emailBusy ? <LoaderCircle className="spin" /> : <ShieldCheck />}{t.confirm}<ArrowRight /></button>
            <button className="resend-code" type="button" disabled={emailBusy} onClick={() => void requestEmail()}><RefreshCcw />{t.emailResend}</button>
          </form>
        )}

        {stage === "awaiting" && (
          <div className="auth-view awaiting-view view-enter" role="status" aria-live="polite">
            <div className="telegram-wait-mark"><MessageCircle /><i /></div>
            <span className="section-label">{t.awaitingStep}</span>
            <h1 className="accent-title"><AccentTitle text={t.awaiting} /></h1>
            <p>{t.awaitingText}</p>
            <div className="challenge-wait-status"><LoaderCircle className="spin" /><span>{t.awaitingStatus}</span></div>
            {challengeMessage && <p className="auth-inline-error" role="alert">{challengeMessage}</p>}
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
            <h1 className="accent-title"><AccentTitle text={confirmedByEmail ? t.emailConfirmed : t.confirmed} /></h1>
            <p>{t.confirmedText}</p>
          </div>
        )}
      </section>
    </main>
  );
}
