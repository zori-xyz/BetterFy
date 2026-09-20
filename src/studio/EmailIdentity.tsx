import { useEffect, useState, type FormEvent } from "react";
import { ArrowRight, Check, Mail } from "lucide-react";
import { beginEmailLink, fetchEmailIdentity, verifyEmailLink, type AuthSession } from "../auth";
import { useLocale } from "../i18n";

export default function EmailIdentity({ session }: { session: AuthSession }) {
  const { language } = useLocale();
  const ru = language === "ru";
  const [hint, setHint] = useState<string | null>(null);
  const [phase, setPhase] = useState<"loading" | "ready" | "code" | "linked" | "unavailable">("loading");
  const [email, setEmail] = useState("");
  const [code, setCode] = useState("");
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState("");

  useEffect(() => {
    let active = true;
    fetchEmailIdentity(session)
      .then((status) => {
        if (!active) return;
        setHint(status.emailHint);
        setPhase(status.linked ? "linked" : "ready");
      })
      .catch(() => { if (active) setPhase("unavailable"); });
    return () => { active = false; };
  }, [session]);

  const start = async (event: FormEvent) => {
    event.preventDefault();
    setBusy(true);
    setError("");
    try {
      await beginEmailLink(session, email, language);
      setPhase("code");
    } catch {
      setError(ru ? "Не удалось отправить код. Почта уже привязана или сервис недоступен." : "Could not send a code. The email may be linked or the service is unavailable.");
    } finally { setBusy(false); }
  };

  const verify = async (event: FormEvent) => {
    event.preventDefault();
    setBusy(true);
    setError("");
    try {
      setHint(await verifyEmailLink(session, email, code));
      setPhase("linked");
    } catch {
      setError(ru ? "Код не подошёл или уже использован." : "The code is invalid or already used.");
    } finally { setBusy(false); }
  };

  return (
    <section className="s-id-connect" aria-labelledby="id-connect-title">
      <div className="s-id-connect-heading">
        <span className="s-id-connect-icon"><Mail /></span>
        <div><h2 id="id-connect-title">{ru ? "Почта для входа по коду" : "Email code sign-in"}</h2><p>{ru ? "Добавь вход по почтовому коду к Telegram-профилю. Это не создаёт BetterFy ID с паролем." : "Add email-code access to this Telegram profile. This does not create a password-based BetterFy ID."}</p></div>
      </div>
      {phase === "loading" && <p role="status">{ru ? "Проверяем способ входа…" : "Checking sign-in methods…"}</p>}
      {phase === "unavailable" && <p role="status">{ru ? "Почтовый вход сейчас недоступен. Telegram продолжит работать." : "Email sign-in is unavailable. Telegram will keep working."}</p>}
      {phase === "linked" && <p className="s-id-linked" role="status"><Check />{ru ? "Почта подключена" : "Email connected"}{hint ? `: ${hint}` : ""}</p>}
      {phase === "ready" && (
        <form onSubmit={start} className="s-id-connect-form">
          <label htmlFor="profile-email">{ru ? "Адрес почты" : "Email address"}</label>
          <div><input id="profile-email" type="email" autoComplete="email" required maxLength={254} value={email} onChange={(event) => setEmail(event.target.value)} placeholder="name@example.com" /><button type="submit" disabled={busy}>{ru ? "Подключить" : "Connect"}<ArrowRight /></button></div>
        </form>
      )}
      {phase === "code" && (
        <form onSubmit={verify} className="s-id-connect-form">
          <label htmlFor="profile-email-code">{ru ? `Код из письма на ${email}` : `Code sent to ${email}`}</label>
          <div><input id="profile-email-code" autoComplete="one-time-code" inputMode="numeric" pattern="[0-9]*" maxLength={6} required value={code} onChange={(event) => setCode(event.target.value.replace(/\D/g, "").slice(0, 6))} /><button type="submit" disabled={busy || code.length !== 6}>{ru ? "Подтвердить" : "Confirm"}<ArrowRight /></button></div>
        </form>
      )}
      {error && <p className="s-form-error" role="alert">{error}</p>}
    </section>
  );
}
