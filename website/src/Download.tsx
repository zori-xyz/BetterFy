import { ArrowDownToLine, ArrowRight, Check, CircleUserRound, ExternalLink, Monitor, ShieldAlert, WifiOff, X } from "lucide-react";
import type { Copy, Language } from "./copy";
import type { WebSession } from "./Account";

export type ReleaseMeta = { version: string; fileName: string; size: number; publishedAt: string; prerelease: boolean };
export type ReleaseState = "idle" | "loading" | "ready" | "missing" | "unavailable";
type GithubRelease = {
  tag_name?: string;
  published_at?: string;
  draft?: boolean;
  prerelease?: boolean;
  assets?: Array<{ name?: string; size?: number }>;
};

export const repo = "https://github.com/zori-xyz/BetterFy";
export const releasesUrl = "https://api.github.com/repos/zori-xyz/BetterFy/releases?per_page=20";
// Same choice as the auth worker's resolver: a stable release first, else the
// newest Early Access pre-release, BetterFy Setup before the older NSIS name.
const installerNames = ["BetterFy-Setup.exe", "BetterFy-Windows-x64-setup.exe"];

export function pickRelease(releases: GithubRelease[]): ReleaseMeta | null {
  const available = releases.filter((release) => release && release.draft !== true);
  const installer = (release: GithubRelease) => {
    for (const name of installerNames) {
      const found = release.assets?.find((item) => item?.name === name && item.size);
      if (found) return found;
    }
    return null;
  };
  const release =
    available.find((item) => item.prerelease !== true && installer(item)) ??
    available.find((item) => item.prerelease === true && installer(item));
  const file = release && installer(release);
  if (!release || !file?.name || !file.size || !release.tag_name || !release.published_at) return null;
  return {
    version: release.tag_name,
    fileName: file.name,
    size: file.size,
    publishedAt: release.published_at,
    prerelease: release.prerelease === true,
  };
}

export function safeReleaseUrl(value?: string) {
  try {
    if (!value) return null;
    const url = new URL(value);
    return url.protocol === "https:" && url.hostname === "github.com" && url.pathname.startsWith("/zori-xyz/BetterFy/releases/")
      ? url.toString()
      : null;
  } catch {
    return null;
  }
}

export function Download({
  language,
  t,
  close,
  session,
  release,
  releaseState,
  retry,
  download,
  error,
}: {
  language: Language;
  t: Copy;
  close: () => void;
  session: WebSession | null;
  release: ReleaseMeta | null;
  releaseState: ReleaseState;
  retry: () => void;
  download: () => void;
  error: string;
}) {
  const d = t.download;
  const loading = releaseState === "loading" || releaseState === "idle";
  const date = release
    ? new Intl.DateTimeFormat(language === "ru" ? "ru-RU" : "en-GB", { day: "2-digit", month: "short", year: "numeric" }).format(
        new Date(release.publishedAt),
      )
    : "—";
  const lead = loading
    ? d.checking
    : release
      ? session
        ? d.readySigned
        : d.ready
      : releaseState === "unavailable"
        ? d.offlineLead
        : d.missingLead;
  const steps: Array<{ state: "done" | "error" | "todo"; text: string; icon: JSX.Element | string }> = [
    { state: "done", text: d.system, icon: <Monitor size={14} /> },
    {
      state: release ? "done" : releaseState === "unavailable" ? "error" : "todo",
      text: loading ? d.locating : release ? d.found : releaseState === "unavailable" ? d.network : d.unavailable,
      icon: release ? <Check size={14} /> : releaseState === "unavailable" ? <WifiOff size={14} /> : "2",
    },
    {
      state: session ? "done" : "todo",
      text: session ? session.displayName : d.login,
      icon: session ? <Check size={14} /> : <CircleUserRound size={14} />,
    },
  ];

  return (
    <div className="modal-backdrop" onMouseDown={close}>
      <section
        className="w-dialog w-download"
        role="dialog"
        aria-modal="true"
        aria-labelledby="w-download-title"
        onMouseDown={(event) => event.stopPropagation()}
      >
        <button className="modal-close" onClick={close} aria-label={t.close}>
          <X size={17} />
        </button>
        <p className="w-micro">{d.dialogTag}</p>
        <h2 id="w-download-title">{d.title}</h2>
        <p className="w-lead">{lead}</p>
        <ol className="w-steps">
          {steps.map((step, index) => (
            <li key={index} className={`is-${step.state}`}>
              <i>{step.icon}</i>
              {step.text}
            </li>
          ))}
        </ol>
        {release && (
          <dl className="w-release">
            <div>
              <dt>{d.version}</dt>
              <dd>{release.version}</dd>
            </div>
            <div>
              <dt>{d.size}</dt>
              <dd title={release.fileName}>{(release.size / 1048576).toFixed(1)} MB</dd>
            </div>
            <div>
              <dt>{d.published}</dt>
              <dd>{date}</dd>
            </div>
          </dl>
        )}
        {release?.prerelease && <p className="w-hint">{d.prerelease}</p>}
        {releaseState === "unavailable" && (
          <button className="w-secondary" onClick={retry}>
            {d.retry}
          </button>
        )}
        <button className="w-primary" onClick={download} disabled={loading || !release}>
          <ArrowDownToLine size={17} />
          {d.cta}
          <ArrowRight size={16} />
        </button>
        {error && (
          <p className="w-form-error" role="alert">
            {error}
          </p>
        )}
        <p className="w-note">
          <ShieldAlert size={15} />
          {d.unsigned}
        </p>
        <a className="w-quiet-link" href={`${repo}/releases`}>
          {d.versions}
          <ExternalLink size={12} />
        </a>
      </section>
    </div>
  );
}
