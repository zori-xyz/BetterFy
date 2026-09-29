import { lazy, Suspense, useEffect, useLayoutEffect, useState } from "react";
import { ArrowLeft, RefreshCcw, WifiOff } from "lucide-react";
import AuthFlow from "../AuthFlow";
import OnboardingFlow from "../OnboardingFlow";
import AppUpdater from "../AppUpdater";
import BetterFyWordmark from "../BetterFyWordmark";
import {
  authErrorCode,
  refreshDesktopProfile,
  restoreDesktopSession,
  revokeAuthSession,
  type AuthSession,
} from "../auth";
import type { GameInstallation } from "../engine";
import { useLocale } from "../i18n";
import { getStorageItem, setStorageItem } from "../storage";

type Stage = "loading" | "offline" | "auth" | "setup" | "workspace";
type Theme = "dark" | "light";
const installationKey = "betterfy:game-installation";
const previewSession: AuthSession = {
  userId: "betterfy-preview",
  displayName: "Preview",
  accessTier: "preview",
  source: "demo",
};
const previewInstallation: GameInstallation = {
  path: "Preview / Dota 2",
  executablePath: "Preview only",
  steamLibrary: "BetterFy interface preview",
  client: "Prototype preview",
  source: "demo",
  verified: false,
};
function storedInstallation(): GameInstallation | null {
  try {
    const item = JSON.parse(getStorageItem(installationKey) ?? "null");
    return item && typeof item.path === "string" && typeof item.verified === "boolean"
      ? item
      : null;
  } catch {
    return null;
  }
}
const Workspace = lazy(() => import("./Workspace"));

export default function StudioApp() {
  const { isRu } = useLocale();
  const [stage, setStage] = useState<Stage>("loading");
  const [session, setSession] = useState<AuthSession | null>(null);
  const [installation, setInstallation] = useState<GameInstallation | null>(null);
  const [theme, setTheme] = useState<Theme>(() =>
    getStorageItem("betterfy:theme") === "light" ? "light" : "dark",
  );
  const [motion, setMotion] = useState(() => getStorageItem("betterfy:motion") !== "off");
  useLayoutEffect(() => {
    document.documentElement.dataset.theme = theme;
    document.documentElement.style.colorScheme = theme;
    setStorageItem("betterfy:theme", theme);
  }, [theme]);
  useEffect(() => {
    document.documentElement.classList.toggle("motion-disabled", !motion);
    setStorageItem("betterfy:motion", motion ? "on" : "off");
    return () => document.documentElement.classList.remove("motion-disabled");
  }, [motion]);
  const enter = (next: AuthSession) => {
    setSession(next);
    const game = storedInstallation();
    setInstallation(game);
    setStage(game ? "workspace" : "setup");
  };
  const restore = () => {
    let active = true;
    const start = performance.now();
    setStage("loading");
    restoreDesktopSession()
      .then(
        (restored) => ({ restored, offline: false }),
        // A network failure is not a sign-out: the refresh credential is
        // still in the vault. Without this the user was sent to sign-in and
        // could not even restore an installed mod while offline.
        (cause) => ({
          restored: null,
          offline: authErrorCode(cause) === "auth_service_unavailable",
        }),
      )
      .then(({ restored, offline }) => {
        const timer = window.setTimeout(
          () => {
            if (!active) return;
            if (restored) enter(restored);
            else setStage(offline ? "offline" : "auth");
          },
          Math.max(0, 700 - (performance.now() - start)),
        );
        if (!active) window.clearTimeout(timer);
      });
    return () => {
      active = false;
    };
  };
  useEffect(restore, []);
  // Premium bought in the bot (or expired) shows up when the user returns to
  // the window, not only after a restart.
  useEffect(() => {
    if (session?.source !== "server") return undefined;
    let last = Date.now();
    const onFocus = () => {
      if (Date.now() - last < 60_000) return;
      last = Date.now();
      refreshDesktopProfile()
        .then((profile) => {
          if (profile)
            setSession((current) => (current?.userId === profile.userId ? profile : current));
        })
        .catch(() => undefined);
    };
    window.addEventListener("focus", onFocus);
    return () => window.removeEventListener("focus", onFocus);
  }, [session?.source]);
  const signout = async () => {
    try {
      await revokeAuthSession(session);
      setSession(null);
      setStage("auth");
      return true;
    } catch {
      return false;
    }
  };
  return (
    <div className="s-root">
      {stage === "loading" && (
        <main className="s-boot">
          <BetterFyWordmark />
          <div className="s-boot-progress" />
          <p role="status">{isRu ? "Открываем BetterFy" : "Opening BetterFy"}</p>
        </main>
      )}
      {stage === "offline" && (
        <main className="s-boot s-offline">
          <BetterFyWordmark />
          <WifiOff />
          <h1>{isRu ? "Нет связи с BetterFy" : "Can't reach BetterFy"}</h1>
          <p>
            {isRu
              ? "Вход сохранён на этом устройстве. Можно повторить подключение или открыть приложение без сети: установленные моды и откат работают, данные аккаунта обновятся позже."
              : "Your sign-in is saved on this device. Retry, or open the app offline: installed mods and restore keep working, and account details update later."}
          </p>
          <div>
            <button className="s-btn s-btn-primary" onClick={restore}>
              <RefreshCcw />
              {isRu ? "Повторить" : "Retry"}
            </button>
            <button
              className="s-btn"
              onClick={() =>
                enter({
                  userId: "offline",
                  displayName: isRu ? "Без сети" : "Offline",
                  accessTier: "offline",
                  source: "offline",
                })
              }
            >
              {isRu ? "Открыть без сети" : "Open offline"}
            </button>
          </div>
        </main>
      )}
      {stage === "auth" && (
        <div className="s-entry">
          <AuthFlow
            onComplete={enter}
            onPreview={() => {
              setSession(previewSession);
              setInstallation(previewInstallation);
              setStage("workspace");
            }}
          />
        </div>
      )}
      {stage === "setup" && (
        <div className="s-entry">
          <OnboardingFlow
            onComplete={(game) => {
              setInstallation(game);
              setStorageItem(installationKey, JSON.stringify(game));
              setStage("workspace");
            }}
          />
          {installation && (
            <button className="s-setup-cancel s-btn" onClick={() => setStage("workspace")}>
              <ArrowLeft />
              {isRu ? "Вернуться в приложение" : "Back to application"}
            </button>
          )}
        </div>
      )}
      {stage === "workspace" && installation && (
        <Suspense
          fallback={
            <main className="s-boot">
              <BetterFyWordmark />
              <p role="status">{isRu ? "Открываем каталог" : "Opening catalog"}</p>
            </main>
          }
        >
          <Workspace
            session={session}
            installation={installation}
            theme={theme}
            setTheme={setTheme}
            motion={motion}
            setMotion={setMotion}
            onReconnect={() => setStage("setup")}
            onSignOut={signout}
          />
        </Suspense>
      )}
      <AppUpdater />
    </div>
  );
}
