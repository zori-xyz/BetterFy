import { useEffect, useState, type FormEvent, type ReactNode } from "react";
import {
  AlertTriangle,
  ArrowLeft,
  ArrowRight,
  Check,
  Eye,
  EyeOff,
  Gamepad2,
  KeyRound,
  LoaderCircle,
  LockKeyhole,
  LogOut,
  Mail,
  Monitor,
  Send,
  ShieldCheck,
  WifiOff,
  X,
} from "lucide-react";
import { CodeCells } from "../../src/CodeCells";
import { emoteMoods } from "../../src/studio/EmoteStage";
import type { Copy, Language } from "./copy";

export type WebSession = {
  userId: string;
  displayName: string;
  username?: string;
  accessTier: string;
  accessExpiresAt?: number;
  sessionId?: string;
  avatarAvailable: boolean;
};
export type DeviceSession = {
  sessionId: string;
  clientKind: "web" | "desktop" | "unknown";
  lastUsedAt: number;
  expiresAt: number;
  current: boolean;
};
export type AuthStage = "idle" | "checking" | "code" | "verifying" | "ready" | "unavailable";

export const authUrl = import.meta.env.VITE_BETTERFY_AUTH_URL || "https://betterfy-auth.zori-xyz.workers.dev";
export const botUrl = "https://t.me/BeterFyBot?start=web_login";

type View = "choice" | "telegram-code" | "id-login" | "id-register" | "id-register-code" | "email" | "email-code";

const moodFor = (id: string) =>
  emoteMoods[[...id].reduce((sum, char) => (sum * 31 + char.charCodeAt(0)) % 9973, 7) % emoteMoods.length];

async function post(path: string, payload: object, token?: string | null) {
  const response = await fetch(new URL(path, authUrl), {
    method: "POST",
    headers: { "content-type": "application/json", ...(token ? { Authorization: `Bearer ${token}` } : {}) },
    body: JSON.stringify(payload),
    credentials: "omit",
  });
  if (!response.ok) throw new Error(String(response.status));
  return response.json();
}

function Field({ id, label, children }: { id: string; label: string; children: ReactNode }) {
  return (
    <div className="w-field">
      <label htmlFor={id}>{label}</label>
      {children}
    </div>
  );
}

// The account dialog: sign-in (Telegram code, BetterFy ID, email code),
// BetterFy ID registration, and once signed in the profile with its email
// link and active devices. Styled like the app's sign-in screens.
export function Account({
  language,
  t,
  close,
  stage,
  session,
  token,
  avatar,
  sessions,
  storageWarning,
  code,
  setCode,
  codeError,
  verifyCode,
  onSignedIn,
  revoke,
  logout,
}: {
  language: Language;
  t: Copy;
  close: () => void;
  stage: AuthStage;
  session: WebSession | null;
  token: string | null;
  avatar: string | null;
  sessions: DeviceSession[];
  storageWarning: boolean;
  code: string;
  setCode: (value: string) => void;
  codeError: string;
  verifyCode: () => void;
  onSignedIn: (data: WebSession & { sessionToken: string }) => void;
  revoke: (item: DeviceSession) => void;
  logout: () => void;
}) {
  const a = t.auth;
  const [view, setView] = useState<View>("choice");
  const [identifier, setIdentifier] = useState("");
  const [username, setUsername] = useState("");
  const [email, setEmail] = useState("");
  const [password, setPassword] = useState("");
  const [showPassword, setShowPassword] = useState(false);
  const [emailCode, setEmailCode] = useState("");
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState("");
  const [emailIdentity, setEmailIdentity] = useState<{ linked: boolean; emailHint: string | null } | null>(null);

  useEffect(() => {
    if (!token) return undefined;
    const controller = new AbortController();
    fetch(new URL("/v1/session/email", authUrl), {
      headers: { Authorization: `Bearer ${token}` },
      credentials: "omit",
      signal: controller.signal,
    })
      .then((response) => (response.ok ? response.json() : Promise.reject()))
      .then((data) => setEmailIdentity(data))
      .catch(() => undefined);
    return () => controller.abort();
  }, [token]);

  const go = (next: View) => {
    setError("");
    setView(next);
  };

  const run = (task: () => Promise<void>) => async (event?: FormEvent) => {
    event?.preventDefault();
    setBusy(true);
    setError("");
    try {
      await task();
    } catch (cause) {
      setError(cause instanceof TypeError ? a.offline : a.invalid);
    } finally {
      setBusy(false);
    }
  };

  const signedIn = (data: WebSession & { sessionToken?: string }) => {
    if (!data.sessionToken || !data.displayName) throw new Error("invalid_response");
    setPassword("");
    setEmailCode("");
    onSignedIn(data as WebSession & { sessionToken: string });
    setView("choice");
  };

  const loginId = run(async () => signedIn(await post("/v1/auth/id/login", { identifier, password, clientKind: "web" })));
  const startRegister = run(async () => {
    await post("/v1/auth/id/register/start", { username, email, password, language });
    setPassword("");
    setEmailCode("");
    setView("id-register-code");
  });
  const finishRegister = run(async () =>
    signedIn(await post("/v1/auth/id/register/verify", { email, code: emailCode, clientKind: "web" })),
  );
  const startEmail = run(async () => {
    await post(session ? "/v1/session/email/start" : "/v1/auth/email/start", { email, language }, session ? token : null);
    setEmailCode("");
    setView("email-code");
  });
  const finishEmail = run(async () => {
    const data = await post(
      session ? "/v1/session/email/verify" : "/v1/auth/email/verify",
      { email, code: emailCode, ...(session ? {} : { clientKind: "web" }) },
      session ? token : null,
    );
    if (session) {
      setEmailIdentity({ linked: true, emailHint: data.emailHint });
      setView("choice");
    } else signedIn(data);
  });

  const access = session?.accessTier === "premium" ? a.premium : a.early;
  const mood = moodFor(session?.userId ?? "guest");
  const date = (seconds: number) =>
    new Intl.DateTimeFormat(language === "ru" ? "ru-RU" : "en-GB", { day: "2-digit", month: "short", year: "numeric" }).format(
      new Date(seconds * 1000),
    );
  const back = (to: View) => (
    <button className="w-back" type="button" onClick={() => go(to)}>
      <ArrowLeft size={15} />
      {a.back}
    </button>
  );
  const problem = error && (
    <p className="w-form-error" role="alert">
      {error}
    </p>
  );
  const passwordField = (autoComplete: string) => (
    <Field id="w-password" label={a.password}>
      <span className="w-password">
        <input
          id="w-password"
          type={showPassword ? "text" : "password"}
          autoComplete={autoComplete}
          required
          minLength={12}
          maxLength={128}
          value={password}
          onChange={(event) => setPassword(event.target.value)}
        />
        <button type="button" onClick={() => setShowPassword((value) => !value)} aria-label={showPassword ? a.hidePassword : a.showPassword}>
          {showPassword ? <EyeOff size={16} /> : <Eye size={16} />}
        </button>
      </span>
    </Field>
  );

  return (
    <div className="modal-backdrop" onMouseDown={close}>
      <section
        className="w-dialog w-account"
        role="dialog"
        aria-modal="true"
        aria-labelledby="w-account-title"
        onMouseDown={(event) => event.stopPropagation()}
        style={{ ["--card-rgb" as string]: mood.rgb }}
      >
        <button className="modal-close" onClick={close} aria-label={t.close}>
          <X size={17} />
        </button>

        <header className="w-account-head">
          <span className="w-account-avatar">
            <span>{avatar ? <img src={avatar} alt="" /> : <img src={`${import.meta.env.BASE_URL}betterfy-logo.png`} alt="" />}</span>
            <img className="w-account-badge" src={mood.still} alt="" />
          </span>
          <div>
            <p className="w-micro">{a.tag}</p>
            <h2 id="w-account-title">{session ? session.displayName : a.title}</h2>
            <p>{session ? (session.username ? `@${session.username} · ${access}` : access) : a.body}</p>
          </div>
        </header>

        {storageWarning && (
          <p className="w-note is-warning" role="status">
            <AlertTriangle size={16} />
            {a.storage}
          </p>
        )}
        {stage === "unavailable" && !session && (
          <p className="w-note is-error" role="alert">
            <WifiOff size={16} />
            {a.offline}
          </p>
        )}

        {session && view === "choice" && (
          <div className="w-account-body">
            <div className="w-facts">
              <span>
                <ShieldCheck size={16} />
                <small>{a.access}</small>
                <b>{access}</b>
              </span>
              <span>
                <Monitor size={16} />
                <small>{a.sessions}</small>
                <b>{sessions.length}</b>
              </span>
            </div>
            <div className="w-row">
              <span className="w-row-icon">
                <Mail size={18} />
              </span>
              <div>
                <b>{a.emailSignIn}</b>
                <small>{emailIdentity?.linked ? `${a.linked} · ${emailIdentity.emailHint}` : a.linkNote}</small>
              </div>
              {!emailIdentity?.linked && (
                <button type="button" className="w-text-action" onClick={() => go("email")}>
                  {a.link}
                  <ArrowRight size={14} />
                </button>
              )}
            </div>
            <div className="w-devices">
              <h3>{a.active}</h3>
              {sessions.map((item) => (
                <article key={item.sessionId}>
                  <span className="w-row-icon">{item.clientKind === "desktop" ? <Gamepad2 size={17} /> : <Monitor size={17} />}</span>
                  <div>
                    <b>{item.clientKind === "desktop" ? a.desktop : a.web}</b>
                    <small>
                      {a.until} {date(item.expiresAt)}
                    </small>
                  </div>
                  {item.current ? <em>{a.current}</em> : <button onClick={() => revoke(item)}>{a.revoke}</button>}
                </article>
              ))}
            </div>
            <button className="w-signout" onClick={logout}>
              <LogOut size={15} />
              {a.signout}
            </button>
          </div>
        )}

        {!session && view === "choice" && (
          <div className="w-account-body">
            <p className="w-choose">{a.choose}</p>
            <a className="w-choice is-telegram" href={botUrl} target="_blank" rel="noreferrer" onClick={() => setCode("")}>
              <span className="w-choice-icon">
                <Send size={18} />
              </span>
              <span>
                <b>{a.telegram}</b>
                <small>{a.telegramNote}</small>
              </span>
              <ArrowRight size={17} />
            </a>
            <button className="w-choice" type="button" onClick={() => go("id-login")}>
              <span className="w-choice-icon">
                <KeyRound size={18} />
              </span>
              <span>
                <b>{a.id}</b>
                <small>{a.idNote}</small>
              </span>
              <ArrowRight size={17} />
            </button>
            <button className="w-have-code" type="button" onClick={() => go("telegram-code")}>
              <KeyRound size={14} />
              {a.have}
            </button>
            <p className="w-safe">
              <ShieldCheck size={15} />
              <span>
                <b>{a.safe}</b> {a.safeText}
              </span>
            </p>
          </div>
        )}

        {!session && view === "telegram-code" && (
          <form
            className="w-account-body"
            onSubmit={(event) => {
              event.preventDefault();
              verifyCode();
            }}
          >
            {back("choice")}
            <label className="w-field-label" htmlFor="w-telegram-code">
              {a.label}
            </label>
            <CodeCells id="w-telegram-code" autoFocus value={code} onChange={setCode} error={Boolean(codeError)} />
            <small className={`w-hint ${codeError ? "is-error" : ""}`}>{codeError || a.oneTime}</small>
            <button className="w-primary" type="submit" disabled={stage === "verifying" || code.length !== 6}>
              {stage === "verifying" ? <LoaderCircle size={16} className="spin" /> : <LockKeyhole size={16} />}
              {stage === "verifying" ? a.checking : a.enter}
            </button>
          </form>
        )}

        {!session && view === "id-login" && (
          <form className="w-account-body" onSubmit={loginId}>
            {back("choice")}
            <Field id="w-identifier" label={a.identifier}>
              <input
                id="w-identifier"
                autoFocus
                required
                autoComplete="username"
                maxLength={254}
                value={identifier}
                onChange={(event) => setIdentifier(event.target.value)}
              />
            </Field>
            {passwordField("current-password")}
            <button
              className="w-text-action"
              type="button"
              onClick={() => {
                setEmail(identifier.includes("@") ? identifier : "");
                go("email");
              }}
            >
              {a.forgot}
            </button>
            {problem}
            <button className="w-primary" type="submit" disabled={busy}>
              {busy ? <LoaderCircle size={16} className="spin" /> : <LockKeyhole size={16} />}
              {busy ? a.checking : a.enter}
            </button>
            <p className="w-switch">
              {a.noId}
              <button type="button" onClick={() => go("id-register")}>
                {a.createShort}
                <ArrowRight size={14} />
              </button>
            </p>
          </form>
        )}

        {!session && view === "id-register" && (
          <form className="w-account-body" onSubmit={startRegister}>
            {back("id-login")}
            <h3 className="w-form-title">{a.create}</h3>
            <Field id="w-username" label={a.username}>
              <input
                id="w-username"
                autoFocus
                required
                autoComplete="username"
                minLength={3}
                maxLength={24}
                pattern="[A-Za-z][A-Za-z0-9_]{2,23}"
                value={username}
                onChange={(event) => setUsername(event.target.value)}
              />
            </Field>
            <Field id="w-email" label={a.email}>
              <input
                id="w-email"
                type="email"
                required
                autoComplete="email"
                maxLength={254}
                value={email}
                onChange={(event) => setEmail(event.target.value)}
              />
            </Field>
            {passwordField("new-password")}
            {problem}
            <button className="w-primary" type="submit" disabled={busy}>
              {busy ? <LoaderCircle size={16} className="spin" /> : <Check size={16} />}
              {busy ? a.checking : a.create}
            </button>
          </form>
        )}

        {(view === "id-register-code" || view === "email-code") && (
          <form className="w-account-body" onSubmit={view === "id-register-code" ? finishRegister : finishEmail}>
            {back(view === "id-register-code" ? "id-register" : "email")}
            <h3 className="w-form-title">{a.emailCode}</h3>
            <p className="w-lead">{view === "id-register-code" ? a.registerNote : a.emailNote}</p>
            <span className="w-destination">{email}</span>
            <label className="w-field-label" htmlFor="w-email-code">
              {a.emailCode}
            </label>
            <CodeCells id="w-email-code" autoFocus value={emailCode} onChange={setEmailCode} error={Boolean(error)} />
            {problem}
            <button className="w-primary" type="submit" disabled={busy || emailCode.length !== 6}>
              {busy ? <LoaderCircle size={16} className="spin" /> : <Check size={16} />}
              {busy ? a.checking : a.enter}
            </button>
            {view === "email-code" && (
              <button className="w-text-action" type="button" disabled={busy} onClick={() => void startEmail()}>
                {a.retry}
              </button>
            )}
          </form>
        )}

        {view === "email" && (
          <form className="w-account-body" onSubmit={startEmail}>
            {back(session ? "choice" : "id-login")}
            <h3 className="w-form-title">{session ? a.link : a.email}</h3>
            <p className="w-lead">{session ? a.linkNote : a.first}</p>
            <Field id="w-email-address" label={a.email}>
              <input
                id="w-email-address"
                type="email"
                autoFocus
                required
                autoComplete="email"
                maxLength={254}
                value={email}
                onChange={(event) => setEmail(event.target.value)}
              />
            </Field>
            {problem}
            <button className="w-primary" type="submit" disabled={busy}>
              {busy ? <LoaderCircle size={16} className="spin" /> : <Mail size={16} />}
              {busy ? a.checking : a.send}
            </button>
          </form>
        )}
      </section>
    </div>
  );
}
