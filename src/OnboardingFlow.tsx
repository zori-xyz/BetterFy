import { useEffect, useRef, useState, type FormEvent } from "react";
import {
  ArrowLeft,
  ArrowRight,
  Check,
  ExternalLink,
  FolderOpen,
  Gamepad2,
  LoaderCircle,
  MessageCircle,
  RefreshCcw,
  Search,
  ShieldCheck,
  TriangleAlert,
} from "lucide-react";
import BetterFyWordmark from "./BetterFyWordmark";
import AccentTitle from "./AccentTitle";
import { engineBridge, type GameInstallation } from "./engine";
import { useLocale } from "./i18n";
import dotaSpiritsSetup from "./assets/dota-spirits-setup.png";

type Stage = "intro" | "scanning" | "found" | "manual" | "not-found" | "error";

const copy = {
  ru: {
    kicker: "ПОДКЛЮЧЕНИЕ ИГРЫ",
    title: "Найдём твою Dota 2",
    text: "BetterFy проверит стандартные библиотеки Steam. До подтверждения пути приложение ничего не меняет.",
    scan: "Найти автоматически",
    manual: "Указать путь вручную",
    safe: "На этом шаге — только чтение",
    scanning: "Проверяем библиотеки Steam",
    scanningText: "Ищем установку и проверяем обязательные файлы.",
    steps: ["Библиотеки Steam", "Папка Dota 2", "Файлы клиента"],
    found: "Установка найдена",
    foundVerified: "Путь и маркерные файлы проверены локально. Запись в папку игры ещё не выполнялась.",
    foundDemo: "Это демонстрация интерфейса. Реальная установка будет подтверждена только в desktop-сборке.",
    path: "ПАПКА ИГРЫ",
    client: "ИСТОЧНИК",
    verification: "ПРОВЕРКА",
    verified: "локальная",
    preview: "демо",
    continue: "Перейти на главную",
    manualTitle: "Укажи папку dota 2 beta",
    manualText: "Путь проверяется внутри desktop-приложения. В браузерном превью результат остаётся демонстрационным.",
    pathPlaceholder: "C:\\Program Files (x86)\\Steam\\steamapps\\common\\dota 2 beta",
    choose: "Проверить путь",
    back: "Назад",
    notFound: "Dota 2 не найдена",
    notFoundText: "Мы не увидели игру в стандартных папках Steam. Укажи установку вручную.",
    chooseManual: "Указать папку",
    retry: "Повторить поиск",
    error: "Путь не подтверждён",
    errorText: "Нужна папка «dota 2 beta» с игровыми файлами. Проверь путь и попробуй снова.",
    community: "Нужна помощь с установкой?",
    communityAction: "Написать в @BetterFyBot",
    demoBadge: "PROTOTYPE",
    previewAction: "Продолжить в preview-режиме",
    artLabel: "DOTA COMPANIONS / CONNECTION",
  },
  en: {
    kicker: "GAME CONNECTION",
    title: "Let's find your Dota 2",
    text: "BetterFy checks standard Steam libraries. Nothing is changed before you confirm the location.",
    scan: "Find automatically",
    manual: "Enter path manually",
    safe: "Read-only at this step",
    scanning: "Checking Steam libraries",
    scanningText: "Looking for the installation and required marker files.",
    steps: ["Steam libraries", "Dota 2 folder", "Client files"],
    found: "Installation found",
    foundVerified: "The path and marker files were verified locally. Nothing has been written to the game folder.",
    foundDemo: "This is an interface demonstration. The real installation is confirmed only in the desktop build.",
    path: "GAME FOLDER",
    client: "SOURCE",
    verification: "VERIFICATION",
    verified: "local",
    preview: "demo",
    continue: "Continue to Home",
    manualTitle: "Enter the dota 2 beta folder",
    manualText: "The path is checked inside the desktop app. Browser preview results remain simulated.",
    pathPlaceholder: "C:\\Program Files (x86)\\Steam\\steamapps\\common\\dota 2 beta",
    choose: "Verify path",
    back: "Back",
    notFound: "Dota 2 wasn't found",
    notFoundText: "The game wasn't found in standard Steam locations. Enter the installation manually.",
    chooseManual: "Enter folder",
    retry: "Search again",
    error: "Path not verified",
    errorText: "Choose a “dota 2 beta” folder containing the game files, then try again.",
    community: "Need help with setup?",
    communityAction: "Message @BetterFyBot",
    demoBadge: "PROTOTYPE",
    previewAction: "Continue in preview mode",
    artLabel: "DOTA COMPANIONS / CONNECTION",
  },
};

export default function OnboardingFlow({
  onComplete,
}: {
  onComplete: (installation: GameInstallation) => void;
}) {
  const { language } = useLocale();
  const t = copy[language];
  const [stage, setStage] = useState<Stage>("intro");
  const [step, setStep] = useState(0);
  const [path, setPath] = useState("");
  const [game, setGame] = useState<GameInstallation | null>(null);
  const mounted = useRef(true);

  useEffect(() => {
    mounted.current = true;
    return () => {
      mounted.current = false;
    };
  }, []);

  const scan = async () => {
    setStage("scanning");
    setStep(0);
    const stepOne = window.setTimeout(() => mounted.current && setStep(1), 480);
    const stepTwo = window.setTimeout(() => mounted.current && setStep(2), 940);
    try {
      const installations = await engineBridge.discoverGame();
      if (!mounted.current) return;
      setGame(installations[0] ?? null);
      setStage(installations.length ? "found" : "not-found");
    } catch {
      if (mounted.current) setStage("error");
    } finally {
      window.clearTimeout(stepOne);
      window.clearTimeout(stepTwo);
    }
  };

  const verifyManual = async (event: FormEvent) => {
    event.preventDefault();
    setStage("scanning");
    setStep(1);
    try {
      const installation = await engineBridge.validateGamePath(path);
      if (!mounted.current) return;
      setGame(installation);
      setStage("found");
    } catch {
      if (mounted.current) setStage("error");
    }
  };

  const continueInPreview = () => {
    setGame({
      path: "Preview / Dota 2",
      executablePath: "Preview only",
      steamLibrary: "BetterFy interface preview",
      client: "Prototype preview",
      source: "demo",
      verified: false,
    });
    setStage("found");
  };

  return (
    <main className={`setup-shell setup-${stage} ${game?.verified ? "setup-verified" : "setup-unverified"}`}>
      <header className="setup-header" data-tauri-drag-region>
        <div className="setup-brand">
          <BetterFyWordmark compact />
          <small>GAME SETUP</small>
        </div>
        <span>SETUP · 01</span>
      </header>

      <section className="setup-scene" aria-hidden="true">
        <div className="setup-art">
          <span>{t.artLabel}</span>
          <img src={dotaSpiritsSetup} alt="" />
          <i className="hero-light-beam" />
        </div>
        <div className="setup-machine">
          <div className="machine-path">
            <span className="machine-node">
              {stage === "found" ? <Check /> : stage === "scanning" ? <Search /> : <FolderOpen />}
            </span>
            <span className="machine-trace"><i /><i /><i /></span>
          </div>
          <p>STEAM · APP 570</p>
        </div>
        <div className="setup-community">
          <MessageCircle />
          <span>{t.community}</span>
          <a href="https://t.me/BetterFyBot" target="_blank" rel="noreferrer">
            {t.communityAction}<ExternalLink />
          </a>
        </div>
      </section>

      <section className="setup-panel">
        {stage === "intro" && (
          <div className="setup-copy setup-enter">
            <span>{t.kicker}</span>
            <h1 className="accent-title"><AccentTitle text={t.title} /></h1>
            <p>{t.text}</p>
            <div className="setup-safety"><ShieldCheck />{t.safe}</div>
            <button className="setup-primary" onClick={scan}>
              <Search />{t.scan}<ArrowRight />
            </button>
            <button className="setup-secondary" onClick={() => setStage("manual")}>
              <FolderOpen />{t.manual}
            </button>
            <button className="setup-preview" onClick={continueInPreview}>{t.previewAction}<ArrowRight /></button>
          </div>
        )}

        {stage === "scanning" && (
          <div className="setup-copy setup-enter" role="status" aria-live="polite">
            <span>SCAN / 570</span>
            <h1 className="accent-title"><AccentTitle text={t.scanning} /></h1>
            <p>{t.scanningText}</p>
            <div className="scan-stack">
              {t.steps.map((item, index) => (
                <div className={index < step ? "done" : index === step ? "active" : ""} key={item}>
                  <span>{index < step ? <Check /> : <LoaderCircle />}</span>
                  <strong>{item}</strong>
                  <small>{index < step ? "OK" : index === step ? "…" : "WAIT"}</small>
                </div>
              ))}
            </div>
          </div>
        )}

        {stage === "manual" && (
          <form className="setup-copy setup-enter" onSubmit={verifyManual}>
            <button type="button" className="setup-back" onClick={() => setStage("intro")}>
              <ArrowLeft />{t.back}
            </button>
            <span>MANUAL / PATH</span>
            <h1 className="accent-title"><AccentTitle text={t.manualTitle} /></h1>
            <p>{t.manualText}</p>
            <label className="setup-path-field">
              <FolderOpen />
              <input
                value={path}
                onChange={(event) => setPath(event.target.value)}
                placeholder={t.pathPlaceholder}
                autoComplete="off"
                spellCheck={false}
              />
            </label>
            <button className="setup-primary" disabled={!path.trim()}>
              <FolderOpen />{t.choose}<ArrowRight />
            </button>
          </form>
        )}

        {stage === "found" && game && (
          <div className="setup-copy setup-enter">
            <span className={game.verified ? "success" : "prototype"}>
              {game.verified ? "READY / 570" : "PREVIEW / 570"}
            </span>
            <h1 className="accent-title"><AccentTitle text={t.found} /></h1>
            <p>{game.verified ? t.foundVerified : t.foundDemo}</p>
            <div className={`found-game ${game.verified ? "verified" : "preview"}`}>
              <span><Gamepad2 /></span>
              <div><strong>Dota 2</strong><small>{game.client}</small></div>
              <i>{game.verified ? <Check /> : t.demoBadge}</i>
            </div>
            <dl className="setup-facts">
              <div><dt>{t.path}</dt><dd title={game.path}>{game.path}</dd></div>
              <div><dt>{t.client}</dt><dd>{game.steamLibrary}</dd></div>
              <div><dt>{t.verification}</dt><dd>{game.verified ? t.verified : t.preview}</dd></div>
            </dl>
            <button className={`setup-primary ${game.verified ? "ready" : ""}`} onClick={() => onComplete(game)}>
              <Check />{t.continue}<ArrowRight />
            </button>
          </div>
        )}

        {(stage === "not-found" || stage === "error") && (
          <div className="setup-copy setup-enter">
            <span className="warning">CHECK / 570</span>
            <div className="setup-error-title">
              <h1 className="accent-title">
                <AccentTitle
                  text={stage === "not-found" ? t.notFound : t.error}
                  trailing={<TriangleAlert className="setup-alert-icon" />}
                />
              </h1>
            </div>
            <p>{stage === "not-found" ? t.notFoundText : t.errorText}</p>
            <button className="setup-primary" onClick={() => setStage("manual")}>
              <FolderOpen />{t.chooseManual}<ArrowRight />
            </button>
            <button className="setup-secondary" onClick={scan}>
              <RefreshCcw />{t.retry}
            </button>
            <button className="setup-preview" onClick={continueInPreview}>{t.previewAction}<ArrowRight /></button>
          </div>
        )}
      </section>
    </main>
  );
}
