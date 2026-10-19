// Tester program: reports a tester sends from the desktop app.
//
// A report is a multipart upload: one `report` part (JSON, the recorded events
// and the tester's notes) and up to MAX_TESTER_IMAGES `image` parts (a
// snapshot of BetterFy's own window or a picture the tester attached, such as
// a Steam screenshot of Dota). The Worker checks the shape and sizes here,
// keeps only counts in D1 and forwards the content to the founder's own chat.

export const MAX_TESTER_ACCOUNTS = 3;
export const MAX_TESTER_IMAGES = 6;
export const MAX_TESTER_IMAGE_BYTES = 5 * 1024 * 1024;
export const MAX_TESTER_REPORT_BYTES = 512 * 1024;
// The parts above plus multipart framing.
export const MAX_TESTER_BODY_BYTES = MAX_TESTER_IMAGES * MAX_TESTER_IMAGE_BYTES + MAX_TESTER_REPORT_BYTES + 64 * 1024;
export const TESTER_REPORTS_PER_DAY = 30;
const MAX_EVENTS = 2000;
const MAX_NOTES = 50;
const MAX_NOTE_CHARS = 4000;
const MAX_NOTES_TOTAL_CHARS = 20000;
// Telegram allows 4096 characters per message; the summary stays below that
// with room for the markup. Longer notes are complete in the attached JSON.
const MAX_SUMMARY_CHARS = 3800;

/**
 * Tester logins from the BETTERFY_TESTER_LOGINS Worker setting. More than
 * MAX_TESTER_ACCOUNTS turns the program off for everyone: a list that grew by
 * mistake must not quietly enroll an extra account.
 */
export function testerLogins(env, normalize) {
  const raw = typeof env.BETTERFY_TESTER_LOGINS === "string" ? env.BETTERFY_TESTER_LOGINS : "";
  const logins = new Set(raw.split(",").map((value) => normalize(value.trim())).filter(Boolean));
  return logins.size > MAX_TESTER_ACCOUNTS ? new Set() : logins;
}

export function testerReportsChat(env) {
  const value = typeof env.TESTER_REPORTS_CHAT_ID === "string" ? env.TESTER_REPORTS_CHAT_ID.trim() : "";
  return /^-?\d{1,20}$/.test(value) ? value : null;
}

/** "png", "jpeg" or null, from the file's own bytes, never its name or type. */
export function imageKind(bytes) {
  if (bytes.length >= 8 && [0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a].every((byte, index) => bytes[index] === byte)) {
    return "png";
  }
  if (bytes.length >= 3 && bytes[0] === 0xff && bytes[1] === 0xd8 && bytes[2] === 0xff) return "jpeg";
  return null;
}

const isCount = (value) => Number.isSafeInteger(value) && value >= 0 && value <= 1_000_000;
const isIsoTime = (value) => typeof value === "string" && value.length <= 40 && !Number.isNaN(Date.parse(value));
const isShortText = (value, max) => typeof value === "string" && value.length <= max && !/[\u0000-\u0008\u000b\u000c\u000e-\u001f]/.test(value);

/** The report part, parsed and checked against the image parts that came with it. */
export function parseTesterReport(text, imageCount) {
  let report;
  try {
    report = JSON.parse(text);
  } catch {
    return null;
  }
  if (!report || typeof report !== "object" || report.schema !== 1) return null;
  if (!isShortText(report.appVersion, 32) || !isShortText(report.platform, 120)) return null;
  if (!Number.isSafeInteger(report.tzOffsetMinutes) || Math.abs(report.tzOffsetMinutes) > 14 * 60) return null;
  if (!isIsoTime(report.startedAt)) return null;
  const counts = report.counts;
  if (!counts || !["screens", "clicks", "failures", "flags"].every((key) => isCount(counts[key]))) return null;
  if (!Array.isArray(report.events) || report.events.length > MAX_EVENTS) return null;
  if (!report.events.every((event) => event && typeof event === "object" && !Array.isArray(event))) return null;
  if (!Array.isArray(report.notes) || report.notes.length > MAX_NOTES) return null;
  let noteChars = 0;
  for (const note of report.notes) {
    if (!note || !isIsoTime(note.at) || !isShortText(note.text, MAX_NOTE_CHARS)) return null;
    noteChars += note.text.length;
  }
  if (noteChars > MAX_NOTES_TOTAL_CHARS) return null;
  if (!Array.isArray(report.images) || report.images.length !== imageCount) return null;
  for (const image of report.images) {
    if (!image || !["snapshot", "attachment"].includes(image.kind)) return null;
    if (image.note !== null && !(Number.isSafeInteger(image.note) && image.note >= 0 && image.note < report.notes.length)) {
      return null;
    }
  }
  return report;
}

export function escapeHtml(value) {
  return String(value).replace(/&/g, "&amp;").replace(/</g, "&lt;").replace(/>/g, "&gt;");
}

function testerClock(iso, offsetMinutes) {
  const local = new Date(Date.parse(iso) + offsetMinutes * 60 * 1000);
  return `${String(local.getUTCHours()).padStart(2, "0")}:${String(local.getUTCMinutes()).padStart(2, "0")}`;
}

function sessionLength(startedAt, now) {
  const minutes = Math.max(0, Math.round((now * 1000 - Date.parse(startedAt)) / 60000));
  return minutes < 60 ? `${minutes} мин` : `${Math.floor(minutes / 60)} ч ${minutes % 60} мин`;
}

/** The message the founder reads first, in Telegram HTML. */
export function testerSummary(report, login, now) {
  const { counts } = report;
  const head = [
    `🐞 <b>Отчёт тестировщика</b> · ${escapeHtml(login)}`,
    `BetterFy ${escapeHtml(report.appVersion)} · ${escapeHtml(report.platform)}`,
    `Сессия ${sessionLength(report.startedAt, now)} · экраны ${counts.screens} · клики ${counts.clicks} · сбои ${counts.failures} · пинги ${counts.flags}`,
  ];
  const tail = [];
  const snapshots = report.images.filter((image) => image.kind === "snapshot").length;
  const pictures = report.images.length - snapshots;
  if (report.images.length) tail.push(`🖼 Снимки окна: ${snapshots} · картинки: ${pictures}`);
  tail.push(`📎 Полный журнал в файле ниже, событий: ${report.events.length}`);

  let budget = MAX_SUMMARY_CHARS - [...head, ...tail].join("\n").length - 40;
  const lines = [];
  if (report.notes.length === 0) {
    lines.push("📝 Заметок нет");
  } else {
    lines.push("📝 <b>Заметки</b>");
    for (const [index, note] of report.notes.entries()) {
      const prefix = `${index + 1}. ${testerClock(note.at, report.tzOffsetMinutes)} · `;
      const text = escapeHtml(note.text.trim());
      if (budget <= prefix.length + 20) {
        lines.push(`… ещё ${report.notes.length - index} в файле`);
        break;
      }
      const room = budget - prefix.length;
      const shown = text.length > room ? `${text.slice(0, Math.max(0, room - 1)).replace(/&[a-z]*$/, "")}…` : text;
      lines.push(prefix + shown);
      budget -= prefix.length + shown.length + 1;
    }
  }
  return [...head, "", ...lines, "", ...tail].join("\n");
}

export function testerImageCaption(image, index) {
  const kind = image.kind === "snapshot" ? "снимок окна BetterFy" : "картинка тестировщика";
  return image.note === null ? `${index + 1} · ${kind}` : `${index + 1} · ${kind} · к заметке ${image.note + 1}`;
}
