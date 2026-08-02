import { useEffect, useRef, useState } from "react";
import { ArrowDownToLine, Check, RefreshCw, ShieldCheck, X } from "lucide-react";
import { check, type Update } from "@tauri-apps/plugin-updater";
import { relaunch } from "@tauri-apps/plugin-process";
import { useLocale } from "./i18n";

type UpdateState = "idle" | "available" | "downloading" | "installed";

const copy = {
  ru: {
    eyebrow: "ПОДПИСАННОЕ ОБНОВЛЕНИЕ",
    available: "Доступна новая версия BetterFy",
    ready: "Обновление установлено",
    detail: (version: string) => `Версия ${version} готова к безопасной установке.`,
    progress: (percent: number) => `Загружаем и проверяем подпись · ${percent}%`,
    install: "Обновить сейчас",
    restart: "Перезапустить BetterFy",
    later: "Позже",
  },
  en: {
    eyebrow: "SIGNED UPDATE",
    available: "A new BetterFy version is available",
    ready: "Update installed",
    detail: (version: string) => `Version ${version} is ready for a verified install.`,
    progress: (percent: number) => `Downloading and verifying signature · ${percent}%`,
    install: "Update now",
    restart: "Restart BetterFy",
    later: "Later",
  },
} as const;

function isTauriRuntime() {
  return "__TAURI_INTERNALS__" in window;
}

export default function AppUpdater() {
  const { language } = useLocale();
  const t = copy[language];
  const pending = useRef<Update | null>(null);
  const [state, setState] = useState<UpdateState>("idle");
  const [version, setVersion] = useState("");
  const [progress, setProgress] = useState(0);

  useEffect(() => {
    if (!isTauriRuntime()) return;
    let active = true;
    const timer = window.setTimeout(async () => {
      try {
        const update = await check();
        if (!active || !update) return;
        pending.current = update;
        setVersion(update.version);
        setState("available");
      } catch {
        // Startup must remain usable offline. A later launch checks again.
      }
    }, 2200);
    return () => {
      active = false;
      window.clearTimeout(timer);
      pending.current?.close().catch(() => undefined);
    };
  }, []);

  const install = async () => {
    const update = pending.current;
    if (!update || state === "downloading") return;
    setState("downloading");
    setProgress(0);
    let downloaded = 0;
    let total = 0;
    try {
      await update.downloadAndInstall((event) => {
        if (event.event === "Started") {
          total = event.data.contentLength ?? 0;
          return;
        }
        if (event.event === "Progress") {
          downloaded += event.data.chunkLength;
          if (total > 0) setProgress(Math.min(99, Math.round((downloaded / total) * 100)));
          return;
        }
        setProgress(100);
      });
      setState("installed");
    } catch {
      setState("available");
      setProgress(0);
    }
  };

  if (state === "idle") return null;

  return (
    <aside className={`app-update-card is-${state}`} role="status" aria-live="polite">
      <div className="app-update-icon">
        {state === "installed" ? <Check /> : state === "downloading" ? <RefreshCw /> : <ArrowDownToLine />}
      </div>
      <div className="app-update-copy">
        <span><ShieldCheck />{t.eyebrow}</span>
        <strong>{state === "installed" ? t.ready : t.available}</strong>
        <small>{state === "downloading" ? t.progress(progress) : t.detail(version)}</small>
        {state === "downloading" && <div className="app-update-progress"><i style={{ width: `${progress}%` }} /></div>}
      </div>
      <button className="app-update-action" onClick={state === "installed" ? () => void relaunch() : install} disabled={state === "downloading"}>
        {state === "installed" ? t.restart : state === "downloading" ? `${progress}%` : t.install}
      </button>
      {state === "available" && (
        <button className="app-update-dismiss" onClick={() => setState("idle")} aria-label={t.later} title={t.later}><X /></button>
      )}
    </aside>
  );
}
