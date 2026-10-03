import { useSyncExternalStore } from "react";

// What BetterFy is doing, in order, for the activity journal under the
// builder. Lives in memory for the session: it explains the current work and
// is not a log file. The Windows test report keeps the durable record.

export type JournalTone = "step" | "ok" | "warn" | "error" | "dev";

export type JournalEntry = {
  id: number;
  at: number;
  tone: JournalTone;
  text: [ru: string, en: string];
  /** Error code, file or count; shown under "Details" or in developer mode. */
  detail?: string;
};

const LIMIT = 120;
let entries: JournalEntry[] = [];
let sequence = 0;
const listeners = new Set<() => void>();

function emit() {
  listeners.forEach((listener) => listener());
}

export const journal = {
  log(tone: JournalTone, ru: string, en: string, detail?: string) {
    sequence += 1;
    entries = [...entries, { id: sequence, at: Date.now(), tone, text: [ru, en] as [string, string], detail }].slice(
      -LIMIT,
    );
    emit();
  },
  clear() {
    entries = [];
    emit();
  },
};

export function useJournal() {
  return useSyncExternalStore(
    (listener) => {
      listeners.add(listener);
      return () => listeners.delete(listener);
    },
    () => entries,
  );
}

export function journalText(list: JournalEntry[], ru: boolean) {
  return list
    .map((entry) => {
      const time = new Date(entry.at).toLocaleTimeString(ru ? "ru-RU" : "en-GB", {
        hour12: false,
      });
      return `${time}  ${entry.text[ru ? 0 : 1]}${entry.detail ? `  [${entry.detail}]` : ""}`;
    })
    .join("\n");
}
