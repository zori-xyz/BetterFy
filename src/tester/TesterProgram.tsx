import { invoke } from "@tauri-apps/api/core";
import { useEffect, useRef, useState, type ChangeEvent } from "react";
import {
  Bug,
  Camera,
  Check,
  CircleAlert,
  Crosshair,
  Eye,
  EyeOff,
  ImagePlus,
  MapPin,
  Monitor,
  NotebookPen,
  Package,
  Plus,
  Send,
  Settings2,
  Skull,
  Trash2,
  X,
} from "lucide-react";
import { refreshDesktopProfile, type AuthSession } from "../auth";
import { useLocale } from "../i18n";
import { getStorageItem, setStorageItem } from "../storage";
import { useDemoSettings } from "../studio/builder/demo";
import { attachPicture, snapshotWindow } from "./pictures";
import {
  testerRecorder,
  useTesterJournal,
  type PreviousRun,
  type TesterEvent,
} from "./recorder";
import { sendTesterReport } from "./send";
import "./tester.css";

const consentKey = "betterfy:tester-consent";
type Consent = "on" | "off" | null;
type SendState = "idle" | "packing" | "flying" | "done" | "error";

const screens: Record<string, [string, string]> = {
  home: ["Главная", "Home"],
  catalog: ["Каталог", "Catalog"],
  build: ["Моя сборка", "My build"],
  library: ["Библиотека", "Library"],
  settings: ["Настройки", "Settings"],
  profile: ["Профиль", "Profile"],
};

const sendErrors: Record<string, [string, string]> = {
  tester_rate_limited: [
    "На сегодня отчётов достаточно, курьер отдыхает до завтра.",
    "That's enough reports for today; the courier rests until tomorrow.",
  ],
  tester_not_enrolled: [
    "Этот аккаунт больше не в программе тестирования.",
    "This account is no longer in the tester program.",
  ],
  tester_report_too_large: [
    "Отчёт слишком тяжёлый для курьера: убери пару картинок.",
    "The report is too heavy for the courier: remove a couple of pictures.",
  ],
  tester_image_too_large: [
    "Картинка слишком большая даже после сжатия.",
    "A picture is too large even after compression.",
  ],
  tester_image_invalid: [
    "Эту картинку не получилось прочитать. Подойдут PNG, JPEG или WebP.",
    "That picture could not be read. PNG, JPEG or WebP work.",
  ],
  auth_session_expired: [
    "Вход устарел. Войди заново, черновик сохранится.",
    "Your sign-in expired. Sign in again; the draft is kept.",
  ],
};

const clock = (iso: string) =>
  new Date(iso).toLocaleTimeString([], {
    hour: "2-digit",
    minute: "2-digit",
    second: "2-digit",
    hourCycle: "h23",
  });

function useEnrollment(session: AuthSession | null) {
  const demo = useDemoSettings();
  const [server, setServer] = useState(false);
  useEffect(() => {
    if (session?.source !== "server") return undefined;
    let active = true;
    setServer(Boolean(session.tester));
    // Sign-in and restore answers carry no tester mark; the profile does.
    refreshDesktopProfile()
      .then((profile) => {
        if (active && profile?.userId === session.userId) setServer(Boolean(profile.tester));
      })
      .catch(() => undefined);
    return () => {
      active = false;
    };
  }, [session?.source, session?.userId, session?.tester]);
  if (session?.source === "demo") return demo.tester;
  return session?.source === "server" && server;
}

function EventLine({ event, isRu }: { event: TesterEvent; isRu: boolean }) {
  const t = (ru: string, en: string) => (isRu ? ru : en);
  const screen = (id: string) => (screens[id] ? screens[id][isRu ? 0 : 1] : id);
  switch (event.type) {
    case "screen":
      return (
        <li className="t-line is-screen">
          <Monitor />
          <span>
            {t("Экран", "Screen")}: <b>{screen(event.screen)}</b>
          </span>
          <time>{clock(event.at)}</time>
        </li>
      );
    case "click":
      return (
        <li className="t-line is-click">
          <Crosshair />
          <span>
            {t("Клик", "Click")}: <b>{event.label}</b>
          </span>
          <time>{clock(event.at)}</time>
        </li>
      );
    case "engine":
      return (
        <li className={`t-line ${event.ok ? "is-engine" : "is-failure"}`}>
          <Settings2 />
          <span>
            {event.command} · {event.ms} {t("мс", "ms")}
            {event.code ? ` · ${event.code}` : ""}
          </span>
          <time>{clock(event.at)}</time>
        </li>
      );
    case "failure":
      return (
        <li className="t-line is-failure">
          <CircleAlert />
          <span>
            {t("Сбой", "Failure")}: <b>{event.code}</b>
            {event.message ? ` · ${event.message}` : ""}
          </span>
          <time>{clock(event.at)}</time>
        </li>
      );
    case "journal":
      return (
        <li className={`t-line is-journal tone-${event.tone}`}>
          <NotebookPen />
          <span>{event.text}</span>
          <time>{clock(event.at)}</time>
        </li>
      );
    case "flag":
      return (
        <li className="t-line is-flag">
          <MapPin />
          <span>
            {t("Пинг: тут что-то не так", "Ping: something's off here")} · {screen(event.screen)}
          </span>
          <time>{clock(event.at)}</time>
        </li>
      );
  }
}

function Score({ value, label, tone }: { value: number; label: string; tone?: string }) {
  return (
    <div className={`t-score ${tone ?? ""}`}>
      <strong key={value}>{value}</strong>
      <span>{label}</span>
    </div>
  );
}

function ConsentDialog({
  onAnswer,
  isRu,
}: {
  onAnswer: (consent: "on" | "off") => void;
  isRu: boolean;
}) {
  return (
    <div className="t-consent-backdrop" data-tester-ui>
      <section
        className="t-consent"
        role="dialog"
        aria-modal="true"
        aria-labelledby="t-consent-title"
      >
        <span className="t-consent-ward" aria-hidden="true">
          <Eye />
        </span>
        <h2 id="t-consent-title">
          {isRu ? "Ты в программе тестирования BetterFy" : "You're in the BetterFy tester program"}
        </h2>
        <p>
          {isRu
            ? "Ты наш Observer Ward: BetterFy может вести журнал того, что ты делаешь, чтобы мы видели то же, что и ты. Журнал всегда открыт тебе, и отправляешь его только ты."
            : "You're our Observer Ward: BetterFy can keep a journal of what you do so we see what you see. The journal is always open to you, and only you send it."}
        </p>
        <div className="t-consent-columns">
          <div>
            <h3>{isRu ? "Записываем" : "Recorded"}</h3>
            <ul>
              <li>{isRu ? "экраны, по которым ты ходишь" : "the screens you visit"}</li>
              <li>
                {isRu ? "надписи кнопок, на которые нажимаешь" : "labels of the buttons you press"}
              </li>
              <li>
                {isRu
                  ? "установку, откат, проверку: результат и время"
                  : "install, restore, check: result and time"}
              </li>
              <li>{isRu ? "сбои и коды ошибок" : "failures and error codes"}</li>
              <li>{isRu ? "версии BetterFy и Windows" : "BetterFy and Windows versions"}</li>
            </ul>
          </div>
          <div>
            <h3>{isRu ? "Никогда" : "Never"}</h3>
            <ul>
              <li>{isRu ? "пароли и всё, что ты печатаешь" : "passwords or anything you type"}</li>
              <li>{isRu ? "твоё имя в путях Windows" : "your name in Windows paths"}</li>
              <li>{isRu ? "почту, Steam ID, длинные номера" : "email, Steam ID, long numbers"}</li>
              <li>{isRu ? "экран вне окна BetterFy" : "the screen outside BetterFy"}</li>
            </ul>
          </div>
        </div>
        <div className="t-consent-actions">
          <button className="s-btn" onClick={() => onAnswer("off")}>
            {isRu ? "Пока без записи" : "Not now"}
          </button>
          <button className="s-btn s-btn-primary" onClick={() => onAnswer("on")}>
            <Eye />
            {isRu ? "Включить запись" : "Start recording"}
          </button>
        </div>
      </section>
    </div>
  );
}

export function TesterProgram({ session }: { session: AuthSession | null }) {
  const { isRu } = useLocale();
  const enrolled = useEnrollment(session);
  const journal = useTesterJournal();
  const [consent, setConsent] = useState<Consent>(() => {
    const stored = getStorageItem(consentKey);
    return stored === "on" || stored === "off" ? stored : null;
  });
  const [open, setOpen] = useState(false);
  const [tab, setTab] = useState<"feed" | "notes">("feed");
  // A Roshan-style clock for the session, ticking only while the window is open.
  const [elapsed, setElapsed] = useState(0);
  useEffect(() => {
    if (!open) return undefined;
    const tick = () => setElapsed(Math.max(0, Math.round((Date.now() - Date.parse(journal.startedAt)) / 1000)));
    tick();
    const timer = window.setInterval(tick, 1000);
    return () => window.clearInterval(timer);
  }, [open, journal.startedAt]);
  const [sending, setSending] = useState<SendState>("idle");
  const [problem, setProblem] = useState("");
  const [bubble, setBubble] = useState<"failure" | "ping" | "crash" | null>(null);
  const [bounce, setBounce] = useState(0);
  const fileInput = useRef<HTMLInputElement>(null);
  const t = (ru: string, en: string) => (isRu ? ru : en);

  useEffect(() => {
    if (enrolled && consent === "on") testerRecorder.start();
    else testerRecorder.stop();
  }, [enrolled, consent]);
  useEffect(() => () => testerRecorder.stop(), []);

  // If the last run did not end normally, say so once and offer to send it.
  useEffect(() => {
    if (!enrolled || !("__TAURI_INTERNALS__" in window)) return undefined;
    let active = true;
    invoke<PreviousRun | null>("take_previous_run")
      .then((run) => {
        if (!active || !run) return;
        testerRecorder.setPreviousRun(run);
        if (run.uncleanExit || run.panic) setBubble("crash");
      })
      .catch(() => undefined);
    return () => {
      active = false;
    };
  }, [enrolled]);

  // A failure makes the bug jump and, with the window closed, offers to send.
  const lastPulse = useRef(journal.failurePulse);
  useEffect(() => {
    if (journal.failurePulse === lastPulse.current) return undefined;
    lastPulse.current = journal.failurePulse;
    setBounce((value) => value + 1);
    if (open) return undefined;
    setBubble("failure");
    const timer = window.setTimeout(() => setBubble(null), 9000);
    return () => window.clearTimeout(timer);
  }, [journal.failurePulse, open]);

  // Ctrl+Shift+F: a ping on the current screen, like Alt+click in Dota.
  useEffect(() => {
    if (!enrolled || !journal.recording) return undefined;
    const onKey = (event: KeyboardEvent) => {
      if (!(event.ctrlKey && event.shiftKey && event.code === "KeyF")) return;
      event.preventDefault();
      testerRecorder.flag();
      setBubble("ping");
      window.setTimeout(() => setBubble((current) => (current === "ping" ? null : current)), 1800);
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [enrolled, journal.recording]);

  if (!enrolled) return null;
  if (consent === null) {
    return (
      <ConsentDialog
        isRu={isRu}
        onAnswer={(answer) => {
          setStorageItem(consentKey, answer);
          setConsent(answer);
        }}
      />
    );
  }

  const setRecording = (on: boolean) => {
    const answer = on ? "on" : "off";
    setStorageItem(consentKey, answer);
    setConsent(answer);
  };

  const takeSnapshot = async () => {
    setProblem("");
    try {
      const dataUrl = await snapshotWindow();
      if (!testerRecorder.addImage("snapshot", dataUrl))
        setProblem(t("Не больше 6 картинок в отчёте.", "At most 6 pictures per report."));
    } catch (cause) {
      const code = cause instanceof Error ? cause.message : "";
      setProblem(
        sendErrors[code]?.[isRu ? 0 : 1] ?? t("Снимок не получился.", "The snapshot failed."),
      );
    }
  };

  const pickPicture = async (event: ChangeEvent<HTMLInputElement>) => {
    const file = event.target.files?.[0];
    event.target.value = "";
    if (!file) return;
    setProblem("");
    try {
      const dataUrl = await attachPicture(file);
      if (!testerRecorder.addImage("attachment", dataUrl))
        setProblem(t("Не больше 6 картинок в отчёте.", "At most 6 pictures per report."));
    } catch (cause) {
      const code = cause instanceof Error ? cause.message : "";
      setProblem(sendErrors[code]?.[isRu ? 0 : 1] ?? sendErrors.tester_image_invalid[isRu ? 0 : 1]);
    }
  };

  const send = async () => {
    if (sending === "packing" || sending === "flying") return;
    setProblem("");
    setSending("packing");
    const flying = window.setTimeout(() => setSending("flying"), 600);
    try {
      await sendTesterReport();
      window.clearTimeout(flying);
      testerRecorder.reset();
      setSending("done");
      window.setTimeout(() => setSending("idle"), 3200);
    } catch (cause) {
      window.clearTimeout(flying);
      const code = typeof cause === "string" ? cause : cause instanceof Error ? cause.message : "";
      setProblem(
        sendErrors[code]?.[isRu ? 0 : 1] ??
          t(
            "Курьер не долетел. Черновик на месте, попробуй ещё раз.",
            "The courier didn't make it. The draft is kept; try again.",
          ),
      );
      setSending("error");
    }
  };

  const pending = journal.images.filter((image) => image.note === null);
  const feed = journal.events.slice(-80).reverse();
  const busy = sending === "packing" || sending === "flying";
  const sendLabel = {
    idle: t("Отправить Zori", "Send to Zori"),
    error: t("Отправить ещё раз", "Send again"),
    packing: t("Пакуем отчёт…", "Packing the report…"),
    flying: t("Курьер летит…", "The courier is flying…"),
    done: t("Курьер доставил!", "Delivered by courier!"),
  }[sending];

  const actions = journal.counts.clicks + journal.counts.screens;
  const hp = Math.max(
    0,
    Math.min(100, Math.round(100 - (journal.counts.failures / Math.max(actions, 5)) * 400)),
  );

  return (
    <div className="t-root" data-tester-ui>
      {bubble && !open && (
        <div className={`t-bubble is-${bubble}`} role="status">
          {bubble === "ping" ? (
            <>
              <MapPin />
              {t("Пинг поставлен", "Ping placed")}
            </>
          ) : bubble === "crash" ? (
            <>
              <CircleAlert />
              <span>
                {t(
                  "В прошлый раз приложение закрылось само. Опиши, что делал, и отправь.",
                  "The app closed by itself last time. Say what you were doing and send it.",
                )}
              </span>
              <button
                onClick={() => {
                  setBubble(null);
                  setOpen(true);
                  setTab("notes");
                }}
              >
                {t("Описать и отправить", "Describe and send")}
              </button>
              <button
                className="t-bubble-close"
                aria-label={t("Позже", "Later")}
                onClick={() => setBubble(null)}
              >
                <X />
              </button>
            </>
          ) : (
            <>
              <CircleAlert />
              <span>{t("Сбой записан в журнал.", "A failure was recorded.")}</span>
              <button
                onClick={() => {
                  setBubble(null);
                  setOpen(true);
                  setTab("notes");
                }}
              >
                {t("Описать и отправить", "Describe and send")}
              </button>
              <button
                className="t-bubble-close"
                aria-label={t("Позже", "Later")}
                onClick={() => setBubble(null)}
              >
                <X />
              </button>
            </>
          )}
        </div>
      )}

      {open && (
        <section className="t-panel" aria-label={t("Тестовый журнал", "Tester journal")}>
          <header className="t-head">
            <div>
              <span className={`t-rec ${journal.recording ? "is-on" : ""}`}>
                {journal.recording ? "REC" : t("ПАУЗА", "PAUSED")}
              </span>
              <h2>{t("Тестовый журнал", "Tester journal")}</h2>
            </div>
            <button
              className="t-icon"
              onClick={() => setRecording(!journal.recording)}
              title={
                journal.recording
                  ? t("Поставить запись на паузу", "Pause recording")
                  : t("Включить запись", "Start recording")
              }
              aria-label={
                journal.recording
                  ? t("Поставить запись на паузу", "Pause recording")
                  : t("Включить запись", "Start recording")
              }
            >
              {journal.recording ? <EyeOff /> : <Eye />}
            </button>
            <button
              className="t-icon"
              onClick={() => setOpen(false)}
              aria-label={t("Свернуть", "Close")}
            >
              <X />
            </button>
          </header>

          <div
            className="t-hp"
            role="img"
            aria-label={t("Прочность сессии", "Session health")}
            title={t(
              "Доля действий без сбоя: чем больше сбоев на клики и экраны, тем ниже.",
              "Share of actions without a failure: the more failures per click and screen, the lower.",
            )}
          >
            <i
              style={{
                width: `${hp}%`,
                background: `hsl(${Math.round(hp * 1.2)} 62% 46%)`,
              }}
            />
            <span>
              {t("Прочность", "Health")} {hp}%
            </span>
            <time>
              {String(Math.floor(elapsed / 60)).padStart(2, "0")}:{String(elapsed % 60).padStart(2, "0")}
            </time>
          </div>
          {(journal.previousRun?.uncleanExit || journal.previousRun?.panic) && (
            <div className="t-grave" role="status">
              <Skull />
              <div>
                <strong>{t("Прошлый запуск оборвался", "The last run ended abruptly")}</strong>
                <small>
                  {journal.previousRun?.panic
                    ? `${journal.previousRun.panic.message} · ${journal.previousRun.panic.location}`
                    : t(
                        "Приложение закрылось без обычного выхода. Опиши, что делал, и отправь.",
                        "The app closed without a normal exit. Say what you were doing and send it.",
                      )}
                </small>
              </div>
              <button onClick={() => setTab("notes")}>{t("Описать", "Describe")}</button>
            </div>
          )}

          <div className="t-scoreboard">
            <Score value={journal.counts.screens} label={t("экраны", "screens")} />
            <Score value={journal.counts.clicks} label={t("клики", "clicks")} />
            <Score
              value={journal.counts.failures}
              label={t("сбои", "failures")}
              tone={journal.counts.failures ? "is-bad" : ""}
            />
            <Score value={journal.counts.flags} label={t("пинги", "pings")} tone="is-ping" />
          </div>

          <div className="t-tabs" role="tablist">
            <button role="tab" aria-selected={tab === "feed"} onClick={() => setTab("feed")}>
              {t("Лента", "Feed")}
            </button>
            <button role="tab" aria-selected={tab === "notes"} onClick={() => setTab("notes")}>
              {t("Заметки", "Notes")}
              {journal.notes.length > 0 && <i>{journal.notes.length}</i>}
            </button>
          </div>

          {tab === "feed" ? (
            <div className="t-feed-wrap">
              {feed.length ? (
                <ol className="t-feed">
                  {feed.map((event, index) => (
                    <EventLine
                      key={`${event.at}-${feed.length - index}`}
                      event={event}
                      isRu={isRu}
                    />
                  ))}
                </ol>
              ) : (
                <p className="t-empty">
                  {journal.recording
                    ? t(
                        "Пока тихо. Походи по BetterFy, и здесь появится всё, что попадёт в отчёт.",
                        "Quiet so far. Walk around BetterFy and everything that goes into the report appears here.",
                      )
                    : t(
                        "Запись на паузе: в журнал ничего не попадает.",
                        "Recording is paused: nothing goes into the journal.",
                      )}
                </p>
              )}
              <p className="t-promise">
                <Eye />
                {t(
                  "Всё, что видно здесь, уйдёт в отчёт. Больше ничего. Ctrl+Shift+F ставит пинг.",
                  "Everything shown here goes into the report, nothing else. Ctrl+Shift+F places a ping.",
                )}
              </p>
            </div>
          ) : (
            <div className="t-notes">
              <label className="t-draft">
                <span>{t("Черновик", "Draft")}</span>
                <textarea
                  aria-label={t("Черновик заметки", "Note draft")}
                  value={journal.draft}
                  maxLength={4000}
                  rows={4}
                  placeholder={t(
                    "Что делал → что ожидал → что увидел",
                    "What I did → what I expected → what I saw",
                  )}
                  onChange={(event) => testerRecorder.setDraft(event.target.value)}
                />
              </label>
              {pending.length > 0 && (
                <div className="t-thumbs">
                  {pending.map((image) => (
                    <figure key={image.id}>
                      <img
                        src={image.dataUrl}
                        alt={
                          image.kind === "snapshot"
                            ? t("Снимок окна", "Window snapshot")
                            : t("Картинка", "Picture")
                        }
                      />
                      <button
                        aria-label={t("Убрать картинку", "Remove picture")}
                        onClick={() => testerRecorder.removeImage(image.id)}
                      >
                        <X />
                      </button>
                    </figure>
                  ))}
                </div>
              )}
              <div className="t-note-actions">
                <button className="s-btn" onClick={takeSnapshot} disabled={busy}>
                  <Camera />
                  {t("Снимок окна", "Snapshot")}
                </button>
                <button
                  className="s-btn"
                  onClick={() => fileInput.current?.click()}
                  disabled={busy}
                >
                  <ImagePlus />
                  {t("Картинка", "Picture")}
                </button>
                <input
                  ref={fileInput}
                  type="file"
                  accept="image/png,image/jpeg,image/webp"
                  hidden
                  onChange={pickPicture}
                />
                <button
                  className="s-btn"
                  onClick={() => testerRecorder.commitDraft()}
                  disabled={!journal.draft.trim()}
                >
                  <Plus />
                  {t("В заметки", "Add note")}
                </button>
              </div>
              {journal.notes.length > 0 && (
                <ol className="t-note-list">
                  {journal.notes.map((note, index) => (
                    <li key={`${note.at}-${index}`}>
                      <time>{clock(note.at)}</time>
                      <p>{note.text}</p>
                      {journal.images.some((image) => image.note === index) && (
                        <div className="t-thumbs is-small">
                          {journal.images
                            .filter((image) => image.note === index)
                            .map((image) => (
                              <figure key={image.id}>
                                <img src={image.dataUrl} alt="" />
                              </figure>
                            ))}
                        </div>
                      )}
                      <button
                        className="t-icon"
                        aria-label={t("Удалить заметку", "Delete note")}
                        onClick={() => testerRecorder.removeNote(index)}
                      >
                        <Trash2 />
                      </button>
                    </li>
                  ))}
                </ol>
              )}
            </div>
          )}

          {problem && (
            <p className="t-problem" role="alert">
              {problem}
            </p>
          )}
          <footer className="t-foot">
            <button
              className={`s-btn s-btn-primary t-send is-${sending}`}
              onClick={send}
              disabled={busy}
            >
              {sending === "done" ? (
                <Check />
              ) : sending === "flying" ? (
                <Package className="t-courier" />
              ) : (
                <Send />
              )}
              {sendLabel}
            </button>
          </footer>
        </section>
      )}

      <button
        className={`t-fab ${journal.recording ? "is-recording" : ""}`}
        key={bounce}
        data-bounce={bounce > 0 ? "" : undefined}
        onClick={() => setOpen((value) => !value)}
        aria-expanded={open}
        aria-label={t("Тестовый журнал", "Tester journal")}
        title={t("Тестовый журнал", "Tester journal")}
      >
        <Bug />
        {journal.recording && <span className="t-fab-rec" aria-hidden="true" />}
        {journal.counts.failures > 0 && (
          <span className="t-fab-count">{journal.counts.failures}</span>
        )}
      </button>
    </div>
  );
}
