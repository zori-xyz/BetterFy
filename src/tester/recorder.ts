import { useSyncExternalStore } from "react";
import { getStorageItem, setStorageItem } from "../storage";
import { redactText } from "./redact";

// The tester journal: what a tester did and what failed, for the report the
// tester sends from the tester window. It records nothing until the account
// is in the tester program and the tester has switched recording on, and the
// tester sees every entry in the window. Typed text is never recorded: a
// click keeps only the label of what was clicked.

export type TesterEvent =
  | { type: "screen"; at: string; screen: string }
  | { type: "click"; at: string; screen: string; label: string }
  | { type: "engine"; at: string; command: string; ok: boolean; code?: string; ms: number }
  | { type: "failure"; at: string; source: "interface" | "engine"; code: string; message?: string }
  | { type: "journal"; at: string; tone: string; text: string }
  | { type: "flag"; at: string; screen: string };

export type TesterNote = { at: string; text: string };
export type TesterImage = {
  id: number;
  kind: "snapshot" | "attachment";
  /** The note it belongs to; null while it belongs to the draft. */
  note: number | null;
  dataUrl: string;
};
export type TesterCounts = { screens: number; clicks: number; failures: number; flags: number };

type State = {
  recording: boolean;
  startedAt: string;
  events: TesterEvent[];
  counts: TesterCounts;
  notes: TesterNote[];
  draft: string;
  images: TesterImage[];
  /** Bumped on every failure, so the window can react to one. */
  failurePulse: number;
};

const LIMIT = 2000;
const key = "betterfy:tester-journal";
const emptyCounts = (): TesterCounts => ({ screens: 0, clicks: 0, failures: 0, flags: 0 });
const now = () => new Date().toISOString();

function restored(): State {
  const fresh: State = {
    recording: false,
    startedAt: now(),
    events: [],
    counts: emptyCounts(),
    notes: [],
    draft: "",
    images: [],
    failurePulse: 0,
  };
  try {
    const stored = JSON.parse(getStorageItem(key) ?? "null");
    if (!stored || typeof stored !== "object") return fresh;
    return {
      ...fresh,
      startedAt: typeof stored.startedAt === "string" ? stored.startedAt : fresh.startedAt,
      events: Array.isArray(stored.events) ? stored.events.slice(-LIMIT) : [],
      counts: { ...emptyCounts(), ...(stored.counts ?? {}) },
      notes: Array.isArray(stored.notes) ? stored.notes : [],
      draft: typeof stored.draft === "string" ? stored.draft : "",
    };
  } catch {
    return fresh;
  }
}

let state = restored();
let screen = "home";
const listeners = new Set<() => void>();
let saveTimer = 0;

// Pictures stay in memory only: they are large, and a restart is a natural
// point to take a fresh one.
function save() {
  window.clearTimeout(saveTimer);
  saveTimer = window.setTimeout(() => {
    const { startedAt, events, counts, notes, draft } = state;
    setStorageItem(key, JSON.stringify({ startedAt, events, counts, notes, draft }));
  }, 800);
}

function update(next: Partial<State>, persist = true) {
  state = { ...state, ...next };
  listeners.forEach((listener) => listener());
  if (persist) save();
}

function push(event: TesterEvent, counts: Partial<TesterCounts> = {}) {
  if (!state.recording) return;
  const total = { ...state.counts };
  for (const [name, value] of Object.entries(counts) as [keyof TesterCounts, number][]) {
    total[name] += value;
  }
  update({
    events: [...state.events, event].slice(-LIMIT),
    counts: total,
    failurePulse: event.type === "failure" ? state.failurePulse + 1 : state.failurePulse,
  });
}

/** The label a person reads on what they clicked, never a field's value. */
function clickLabel(element: Element) {
  const own =
    element.getAttribute("aria-label") ??
    element.getAttribute("title") ??
    (element instanceof HTMLElement ? element.innerText : element.textContent) ??
    "";
  const label = own.replace(/\s+/g, " ").trim();
  if (
    element instanceof HTMLInputElement &&
    (element.type === "checkbox" || element.type === "radio")
  ) {
    const text = element.closest("label")?.textContent?.replace(/\s+/g, " ").trim() ?? label;
    return `${text.slice(0, 56)} ${element.checked ? "☑" : "☐"}`;
  }
  return label.slice(0, 60) || element.tagName.toLowerCase();
}

const clickable =
  'button, a, summary, [role="button"], [role="tab"], [role="menuitem"], [role="switch"], input[type="checkbox"], input[type="radio"]';

function onClick(event: MouseEvent) {
  const target = event.target instanceof Element ? event.target : null;
  if (!target || target.closest("[data-tester-ui]")) return;
  const element = target.closest(clickable);
  if (!element) return;
  push({ type: "click", at: now(), screen, label: redactText(clickLabel(element)) }, { clicks: 1 });
}

function failureMessage(value: unknown) {
  const text = value instanceof Error ? value.message : typeof value === "string" ? value : "";
  return redactText(text).slice(0, 300);
}

function onError(event: ErrorEvent) {
  testerRecorder.failure("interface", "script_error", failureMessage(event.error ?? event.message));
}

function onRejection(event: PromiseRejectionEvent) {
  testerRecorder.failure("interface", "unhandled_rejection", failureMessage(event.reason));
}

export const testerRecorder = {
  get recording() {
    return state.recording;
  },
  start() {
    if (state.recording) return;
    document.addEventListener("click", onClick, true);
    window.addEventListener("error", onError);
    window.addEventListener("unhandledrejection", onRejection);
    update({ recording: true }, false);
    // Resuming on the screen last recorded is not a new visit.
    const last = [...state.events].reverse().find((event) => event.type === "screen");
    if (!last || last.type !== "screen" || last.screen !== screen) {
      push({ type: "screen", at: now(), screen }, { screens: 1 });
    }
  },
  stop() {
    document.removeEventListener("click", onClick, true);
    window.removeEventListener("error", onError);
    window.removeEventListener("unhandledrejection", onRejection);
    update({ recording: false }, false);
  },
  screen(next: string) {
    if (next === screen) return;
    screen = next;
    push({ type: "screen", at: now(), screen: next }, { screens: 1 });
  },
  engine(command: string, ok: boolean, ms: number, code?: string) {
    push({ type: "engine", at: now(), command, ok, ms: Math.round(ms), ...(code ? { code } : {}) });
    if (!ok) this.failure("engine", code ?? "engine_error", command);
  },
  failure(source: "interface" | "engine", code: string, message?: string) {
    push(
      { type: "failure", at: now(), source, code, ...(message ? { message } : {}) },
      { failures: 1 },
    );
  },
  journal(tone: string, text: string) {
    push({ type: "journal", at: now(), tone, text: redactText(text).slice(0, 200) });
  },
  flag() {
    push({ type: "flag", at: now(), screen }, { flags: 1 });
  },
  setDraft(draft: string) {
    update({ draft: draft.slice(0, 4000) });
  },
  /** The draft becomes a note, and pictures taken while writing it join it. */
  commitDraft() {
    const text = state.draft.trim();
    if (!text) return;
    const index = state.notes.length;
    update({
      notes: [...state.notes, { at: now(), text }],
      draft: "",
      images: state.images.map((image) =>
        image.note === null ? { ...image, note: index } : image,
      ),
    });
  },
  removeNote(index: number) {
    update({
      notes: state.notes.filter((_, position) => position !== index),
      images: state.images
        .filter((image) => image.note !== index)
        .map((image) =>
          image.note !== null && image.note > index ? { ...image, note: image.note - 1 } : image,
        ),
    });
  },
  addImage(kind: TesterImage["kind"], dataUrl: string) {
    if (state.images.length >= 6) return false;
    update(
      { images: [...state.images, { id: Date.now() + Math.random(), kind, note: null, dataUrl }] },
      false,
    );
    return true;
  },
  removeImage(id: number) {
    update({ images: state.images.filter((image) => image.id !== id) }, false);
  },
  /** After a report is sent: a fresh session starts. */
  reset() {
    update({
      startedAt: now(),
      events: [],
      counts: emptyCounts(),
      notes: [],
      draft: "",
      images: [],
    });
    if (state.recording) push({ type: "screen", at: now(), screen }, { screens: 1 });
  },
  snapshot() {
    return state;
  },
};

export function useTesterJournal() {
  return useSyncExternalStore(
    (listener) => {
      listeners.add(listener);
      return () => listeners.delete(listener);
    },
    () => state,
  );
}
