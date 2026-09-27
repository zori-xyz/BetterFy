import { lazy, Suspense, useEffect, useLayoutEffect, useRef, useState } from "react";
import {
  ArrowLeft,
  ArrowRight,
  ArrowUpRight,
  Check,
  ChevronDown,
  ChevronRight,
  CircleUserRound,
  Compass,
  FolderHeart,
  Gamepad2,
  Layers3,
  LoaderCircle,
  Plus,
  Search,
  Send,
  Settings,
  SlidersHorizontal,
  Sparkles,
  TriangleAlert,
  X,
} from "lucide-react";
import AuthFlow from "../AuthFlow";
import OnboardingFlow from "../OnboardingFlow";
import AppUpdater from "../AppUpdater";
import BetterFyWordmark from "../BetterFyWordmark";
import { restoreDesktopSession, revokeAuthSession, type AuthSession } from "../auth";
import type { GameInstallation } from "../engine";
import { useLocale, modCount } from "../i18n";
import { getStorageItem, getStoredStringArray, setStorageItem } from "../storage";

type Route = "home" | "catalog" | "build" | "library" | "settings" | "profile";
type Stage = "loading" | "auth" | "setup" | "workspace";
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
  useEffect(() => {
    let active = true;
    const start = performance.now();
    restoreDesktopSession()
      .catch(() => null)
      .then((restored) => {
        const timer = window.setTimeout(
          () => {
            if (!active) return;
            if (restored) {
              setSession(restored);
              const game = storedInstallation();
              setInstallation(game);
              setStage(game ? "workspace" : "setup");
            } else setStage("auth");
          },
          Math.max(0, 700 - (performance.now() - start)),
        );
        if (!active) window.clearTimeout(timer);
      });
    return () => {
      active = false;
    };
  }, []);
  const signout = async () => {
    try {
      await revokeAuthSession(session);
      setSession(null);
      setStage("auth");
    } catch {
      window.alert(
        isRu
          ? "Не удалось завершить вход. Попробуй ещё раз."
          : "Could not sign out. Please try again.",
      );
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
      {stage === "auth" && (
        <div className="s-entry">
          <AuthFlow
            onComplete={(next) => {
              setSession(next);
              const game = storedInstallation();
              setInstallation(game);
              setStage(game ? "workspace" : "setup");
            }}
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
            onSignOut={() => void signout()}
          />
        </Suspense>
      )}
      <AppUpdater />
    </div>
  );
}
