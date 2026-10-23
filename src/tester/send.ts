import { invoke } from "@tauri-apps/api/core";
import { testerRecorder } from "./recorder";
import { dataUrlBase64 } from "./pictures";

const isDesktop = () => typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;

type HighEntropyNavigator = Navigator & {
  userAgentData?: {
    platform?: string;
    getHighEntropyValues?: (hints: string[]) => Promise<{ platformVersion?: string }>;
  };
};

// The user agent says "Windows NT 10.0" on Windows 11 as well; Microsoft's
// documented way to tell them apart is the platformVersion client hint
// (13 and above is Windows 11).
async function platformName() {
  const data = (navigator as HighEntropyNavigator).userAgentData;
  const webview = /Edg\/(\d+)/.exec(navigator.userAgent)?.[1];
  try {
    if (data?.platform === "Windows" && data.getHighEntropyValues) {
      const { platformVersion = "" } = await data.getHighEntropyValues(["platformVersion"]);
      const major = Number(platformVersion.split(".")[0]);
      const windows = major >= 13 ? "Windows 11" : major > 0 ? "Windows 10" : "Windows";
      return `${windows} (${platformVersion})${webview ? ` · WebView2 ${webview}` : ""}`;
    }
  } catch {
    // Falls back to the plain user agent below.
  }
  return (/\(([^)]+)\)/.exec(navigator.userAgent)?.[1] ?? "unknown").slice(0, 120);
}

/** Sends the current journal, notes and pictures; returns the report ID. */
export async function sendTesterReport() {
  testerRecorder.commitDraft();
  const state = testerRecorder.snapshot();
  const report = {
    schema: 1,
    appVersion: __APP_VERSION__,
    platform: await platformName(),
    tzOffsetMinutes: -new Date().getTimezoneOffset(),
    startedAt: state.startedAt,
    counts: state.counts,
    notes: state.notes,
    images: state.images.map(({ kind, note }) => ({ kind, note })),
    events: state.events,
    previousRun: state.previousRun,
  };
  if (!isDesktop()) {
    // The browser preview has no engine and no session: the report is
    // packed exactly as on Windows and then dropped.
    JSON.stringify(report);
    await new Promise((resolve) => window.setTimeout(resolve, 1400));
    return "preview";
  }
  const receipt = await invoke<{ reportId: string }>("send_tester_report", {
    report: JSON.stringify(report),
    images: state.images.map((image) => dataUrlBase64(image.dataUrl)),
  });
  return receipt.reportId;
}
