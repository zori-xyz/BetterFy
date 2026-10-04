import { useEffect, useRef, useState } from "react";
import { ChevronDown, Copy, Check, Eraser } from "lucide-react";
import { useLocale } from "../../i18n";
import { journal, journalText, useJournal, type JournalEntry } from "./journal";

const toneLabel: Record<JournalEntry["tone"], [string, string]> = {
  step: ["шаг", "step"],
  ok: ["готово", "done"],
  warn: ["внимание", "note"],
  error: ["стоп", "stop"],
  dev: ["dev", "dev"],
};

// What BetterFy is doing, one line per step. Collapsed it shows only the
// latest line; open it to read the whole session.
export function BuildJournal({ busy, developer }: { busy: boolean; developer: boolean }) {
  const { isRu } = useLocale();
  const entries = useJournal();
  const [open, setOpen] = useState(false);
  const [copied, setCopied] = useState(false);
  const listRef = useRef<HTMLOListElement>(null);
  const last = entries[entries.length - 1];
  useEffect(() => {
    const list = listRef.current;
    if (open && list) list.scrollTop = list.scrollHeight;
  }, [open, entries.length]);
  const time = (at: number) =>
    new Date(at).toLocaleTimeString(isRu ? "ru-RU" : "en-GB", { hour12: false });
  const visible = developer ? entries : entries.filter((entry) => entry.tone !== "dev");
  return (
    <section className={`b-journal ${open ? "is-open" : ""} ${busy ? "is-live" : ""}`}>
      <button
        className="b-journal-bar"
        type="button"
        aria-expanded={open}
        aria-controls="build-journal-list"
        onClick={() => setOpen((value) => !value)}
      >
        <span className="b-journal-dot" aria-hidden="true" />
        <b>{isRu ? "Журнал BetterFy" : "BetterFy journal"}</b>
        <span className="b-journal-last" aria-live="polite">
          {last
            ? last.text[isRu ? 0 : 1]
            : isRu
              ? "Пока тихо. Здесь будет видно каждый шаг."
              : "Quiet so far. Every step shows up here."}
        </span>
        <small>{visible.length}</small>
        <ChevronDown className="b-journal-chevron" />
      </button>
      {open && (
        <div className="b-journal-body">
          {visible.length ? (
            <ol id="build-journal-list" ref={listRef}>
              {visible.map((entry) => (
                <li key={entry.id} className={`is-${entry.tone}`}>
                  <time>{time(entry.at)}</time>
                  <span className="b-journal-tone">{toneLabel[entry.tone][isRu ? 0 : 1]}</span>
                  <span>
                    {entry.text[isRu ? 0 : 1]}
                    {entry.detail && (developer || entry.tone === "error") && (
                      <code>{entry.detail}</code>
                    )}
                  </span>
                </li>
              ))}
            </ol>
          ) : (
            <p id="build-journal-list" className="b-journal-empty">
              {isRu
                ? "Подготовка, установка и откат пишут сюда, что именно происходит."
                : "Preparing, installing and restoring write here what exactly happens."}
            </p>
          )}
          <div className="b-journal-tools">
            <button
              className="s-text-button"
              type="button"
              disabled={!visible.length}
              onClick={() => {
                navigator.clipboard
                  .writeText(journalText(visible, isRu))
                  .then(() => {
                    setCopied(true);
                    window.setTimeout(() => setCopied(false), 1800);
                  })
                  .catch(() => undefined);
              }}
            >
              {copied ? <Check /> : <Copy />}
              {copied ? (isRu ? "Скопировано" : "Copied") : isRu ? "Копировать" : "Copy"}
            </button>
            <button
              className="s-text-button"
              type="button"
              disabled={!entries.length || busy}
              onClick={() => journal.clear()}
            >
              <Eraser />
              {isRu ? "Очистить" : "Clear"}
            </button>
          </div>
        </div>
      )}
    </section>
  );
}
