import { useEffect, useState } from "react";
import {
  ArrowUpRight,
  Check,
  CircleUserRound,
  ExternalLink,
  FolderOpen,
  Gamepad2,
  Globe2,
  HelpCircle,
  Laptop,
  LoaderCircle,
  LogOut,
  Moon,
  Paintbrush,
  Send,
  Shield,
  Sun,
} from "lucide-react";
import { useLocale } from "../i18n";
import { engineBridge, type GameInstallation, type SystemDiagnosticReport } from "../engine";
import {
  fetchDeviceSessions,
  revokeDeviceSession,
  type AuthSession,
  type DeviceSession,
} from "../auth";
import { PageHead } from "./ui";
import EmailIdentity from "./EmailIdentity";
import BetterFyWordmark from "../BetterFyWordmark";
import { revealTheme } from "./delight";

export function Preferences({
  theme,
  setTheme,
  motion,
  setMotion,
  installation,
  onReconnect,
}: {
  theme: "dark" | "light";
  setTheme: (theme: "dark" | "light") => void;
  motion: boolean;
  setMotion: (motion: boolean) => void;
  installation: GameInstallation;
  onReconnect: () => void;
}) {
  const { language, setLanguage, isRu } = useLocale();
  const [tab, setTab] = useState("appearance");
  const [diagnostics, setDiagnostics] = useState<SystemDiagnosticReport | null>(null);
  const [checking, setChecking] = useState(false);
  const [error, setError] = useState(false);
  const run = async () => {
    setChecking(true);
    setError(false);
    try {
      setDiagnostics(await engineBridge.collectSystemDiagnostics(installation.path));
    } catch {
      setError(true);
    } finally {
      setChecking(false);
    }
  };
  const tabs = [
    { id: "appearance", icon: Paintbrush, label: isRu ? "Внешний вид" : "Appearance" },
    { id: "game", icon: Gamepad2, label: isRu ? "Игра и файлы" : "Game & files" },
    { id: "about", icon: HelpCircle, label: isRu ? "О приложении" : "About" },
  ];
  return (
    <>
      <PageHead
        title={isRu ? "Настройки" : "Settings"}
        description={isRu ? "Сделай BetterFy удобным для себя." : "Make BetterFy work for you."}
      />
      <div className="s-settings-layout">
        <nav
          className="s-settings-nav"
          aria-label={isRu ? "Разделы настроек" : "Settings sections"}
        >
          {tabs.map(({ id, icon: Icon, label }) => (
            <button
              key={id}
              aria-current={tab === id ? "page" : undefined}
              onClick={() => setTab(id)}
            >
              <Icon />
              {label}
            </button>
          ))}
        </nav>
        <section className="s-settings-content">
          {tab === "appearance" && (
            <>
              <h2>{isRu ? "Внешний вид" : "Appearance"}</h2>
              <p>
                {isRu
                  ? "Выбери тему, язык и характер движения."
                  : "Choose your theme, language and motion."}
              </p>
              <div className="s-theme-grid">
                {(["dark", "light"] as const).map((value) => (
                  <button
                    key={value}
                    className={`s-theme-option is-${value}`}
                    aria-pressed={theme === value}
                    onClick={(event) =>
                      revealTheme(event, () => {
                        document.documentElement.dataset.theme = value;
                        setTheme(value);
                      })
                    }
                  >
                    <span className="s-theme-mini">
                      <BetterFyWordmark />
                      <i />
                    </span>
                    <strong>
                      {value === "dark" ? <Moon /> : <Sun />}
                      {value === "dark" ? (isRu ? "Графит" : "Graphite") : isRu ? "Мел" : "Chalk"}
                      {theme === value && <Check />}
                    </strong>
                  </button>
                ))}
              </div>
              <div className="s-setting-row">
                <span className="s-setting-icon">
                  <Globe2 />
                </span>
                <div>
                  <strong>{isRu ? "Язык" : "Language"}</strong>
                  <p>
                    {isRu
                      ? "Изменится сразу во всём приложении"
                      : "Changes instantly across the application"}
                  </p>
                </div>
                <div className="s-segment">
                  <button aria-pressed={language === "ru"} onClick={() => setLanguage("ru")}>
                    Русский
                  </button>
                  <button aria-pressed={language === "en"} onClick={() => setLanguage("en")}>
                    English
                  </button>
                </div>
              </div>
              <div className="s-setting-row">
                <span className="s-setting-icon">
                  <Paintbrush />
                </span>
                <div>
                  <strong>{isRu ? "Анимации интерфейса" : "Interface motion"}</strong>
                  <p>
                    {isRu
                      ? "Плавные переходы и отклики на действия"
                      : "Smooth transitions and interaction feedback"}
                  </p>
                </div>
                <button
                  className="s-switch"
                  role="switch"
                  aria-checked={motion}
                  aria-label={isRu ? "Анимации интерфейса" : "Interface motion"}
                  onClick={() => setMotion(!motion)}
                >
                  <span />
                </button>
              </div>
            </>
          )}
          {tab === "game" && (
            <>
              <h2>Dota 2</h2>
              <p>
                {isRu
                  ? "Подключение игры и диагностика установки."
                  : "Game connection and installation diagnostics."}
              </p>
              <div className="s-connected-game">
                <span>
                  <Gamepad2 />
                </span>
                <div>
                  <strong>Dota 2</strong>
                  <small>
                    {installation.verified
                      ? isRu
                        ? "Установка найдена"
                        : "Installation found"
                      : isRu
                        ? "Демонстрационный профиль"
                        : "Preview profile"}
                  </small>
                </div>
                <span className="s-chip">Steam</span>
              </div>
              <label className="s-field">
                <span>{isRu ? "Папка игры" : "Game folder"}</span>
                <div className="s-readonly-path">
                  <FolderOpen />
                  <span title={installation.path}>{installation.path}</span>
                </div>
              </label>
              <button className="s-btn" onClick={onReconnect}>
                {isRu ? "Выбрать другую папку" : "Choose another folder"}
                <ArrowUpRight />
              </button>
              <div className="s-settings-divider" />
              <div className="s-setting-row">
                <span className="s-setting-icon">
                  <Shield />
                </span>
                <div>
                  <strong>{isRu ? "Диагностика" : "Diagnostics"}</strong>
                  <p>
                    {isRu
                      ? "Проверить подключение и состояние приложения"
                      : "Check your connection and app state"}
                  </p>
                </div>
                <button className="s-btn" disabled={checking} onClick={() => void run()}>
                  {checking ? <LoaderCircle className="s-spin" /> : <Check />}
                  {isRu ? "Проверить" : "Check"}
                </button>
              </div>
              {error && (
                <p className="s-form-error" role="alert">
                  {isRu
                    ? "Проверка не завершилась. Попробуй ещё раз."
                    : "Check did not complete. Try again."}
                </p>
              )}
              {diagnostics && (
                <div className="s-diagnostics" role="status">
                  <strong>{isRu ? "Проверка завершена" : "Check complete"}</strong>
                  <p>
                    {diagnostics.platform} · BetterFy {diagnostics.appVersion}
                  </p>
                  <p>
                    {installation.verified
                      ? isRu
                        ? "Результаты диагностики доступны ниже."
                        : "Diagnostic results are available below."
                      : isRu
                        ? "В браузере показан демонстрационный результат."
                        : "The browser shows a demonstration result."}
                  </p>
                  <details>
                    <summary>{isRu ? "Технические сведения" : "Technical details"}</summary>
                    <pre>{JSON.stringify(diagnostics, null, 2)}</pre>
                  </details>
                </div>
              )}
            </>
          )}
          {tab === "about" && (
            <>
              <h2>BetterFy</h2>
              <p>
                {isRu
                  ? "Менеджер модов и обликов для Dota 2."
                  : "A mod and cosmetic manager for Dota 2."}
              </p>
              <div className="s-about-version">
                <span>{__APP_VERSION__}</span>
                <small>{isRu ? "Ранний доступ" : "Early access"}</small>
              </div>
              <div className="s-about-note">
                {isRu
                  ? "Собирай и сохраняй наборы модов и обликов. На Windows BetterFy уже ставит проверенные моды настройки игры одним файлом, с резервной копией и откатом в один клик. Облики пока только для просмотра."
                  : "Build and save sets of mods and looks. On Windows, BetterFy installs audited game-tuning mods as one file, with a backup and one-click restore. Looks are preview-only for now."}
              </div>
              <a
                className="s-settings-link"
                href="https://t.me/BeterHelp"
                target="_blank"
                rel="noreferrer"
              >
                <Send />
                <span>{isRu ? "Помощь и обратная связь" : "Help & feedback"}</span>
                <ArrowUpRight />
              </a>
              <a
                className="s-settings-link"
                href="https://github.com/zori-xyz/BetterFy"
                target="_blank"
                rel="noreferrer"
              >
                <ExternalLink />
                <span>{isRu ? "Исходный код" : "Source code"}</span>
                <ArrowUpRight />
              </a>
            </>
          )}
        </section>
      </div>
    </>
  );
}

export function Profile({
  session,
  avatarUrl,
  onSignOut,
}: {
  session: AuthSession | null;
  avatarUrl: string | null;
  onSignOut: () => Promise<boolean>;
}) {
  const { isRu, language } = useLocale();
  const [devices, setDevices] = useState<DeviceSession[]>([]);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState(false);
  const [revoking, setRevoking] = useState<string | null>(null);
  const [revokeError, setRevokeError] = useState(false);
  const [signOutError, setSignOutError] = useState(false);
  const preview = !session || session.source === "demo";
  const offline = session?.source === "offline";
  // Premium is bought in the Telegram bot and attaches to the Telegram
  // account; a standalone BetterFy ID would pay for a different account.
  const noTelegram = session?.telegramLinked === false;
  const signOut = async () => {
    setSignOutError(false);
    if (!(await onSignOut())) setSignOutError(true);
  };
  useEffect(() => {
    setDevices([]);
    setError(false);
    if (preview || offline || !session) return;
    let active = true;
    setLoading(true);
    fetchDeviceSessions(session)
      .then((items) => {
        if (active) setDevices(items);
      })
      .catch(() => {
        if (active) setError(true);
      })
      .finally(() => {
        if (active) setLoading(false);
      });
    return () => {
      active = false;
    };
  }, [session, preview, offline]);
  return (
    <div className="s-profile">
      <PageHead
        title={isRu ? "Профиль" : "Profile"}
        description={
          isRu ? "Твой аккаунт и доступ к BetterFy." : "Your account and BetterFy access."
        }
      />
      <section className="s-profile-card">
        <div className="s-profile-cover" />
        <div className="s-profile-person">
          <span className="s-avatar">
            {avatarUrl ? <img src={avatarUrl} alt="" /> : <CircleUserRound />}
          </span>
          <div>
            <h2>{preview ? (isRu ? "Гость BetterFy" : "BetterFy guest") : session.displayName}</h2>
            <p>
              {preview
                ? isRu
                  ? "Знакомство с приложением"
                  : "Exploring the application"
                : session.username
                  ? `@${session.username}`
                  : isRu
                    ? "Аккаунт BetterFy"
                    : "BetterFy account"}
            </p>
          </div>
          <span className="s-chip">
            {preview
              ? isRu
                ? "Без входа"
                : "Not signed in"
              : offline
                ? isRu
                  ? "Нет связи"
                  : "Offline"
                : isRu
                  ? "Подключён"
                  : "Connected"}
          </span>
        </div>
        <div className="s-profile-access">
          <div>
            <span className="s-eyebrow">{isRu ? "ДОСТУП" : "ACCESS"}</span>
            <h3>
              {preview
                ? isRu
                  ? "Продолжи со своим аккаунтом"
                  : "Continue with your account"
                : session.accessTier === "premium"
                  ? "BetterFy Premium"
                  : isRu
                    ? "Ранний доступ"
                    : "Early access"}
            </h3>
            <p>
              {preview
                ? isRu
                  ? "Сейчас ты смотришь интерфейс без входа. Выбери BetterFy ID или Telegram, чтобы войти."
                  : "You are exploring without signing in. Choose BetterFy ID or Telegram to continue."
                : offline
                  ? isRu
                    ? "Сервер BetterFy сейчас недоступен. Вход сохранён; статус доступа обновится после перезапуска с сетью."
                    : "The BetterFy server is unreachable. Your sign-in is saved; access status updates after a restart with a connection."
                  : noTelegram
                    ? isRu
                      ? "Premium оформляется в Telegram-боте и пока привязывается к Telegram-аккаунту, а не к BetterFy ID. Связка аккаунтов появится позже."
                      : "Premium is purchased in the Telegram bot and currently attaches to a Telegram account, not to a BetterFy ID. Account linking is coming later."
                    : session.accessExpiresAt
                      ? `${isRu ? "Действует до" : "Active until"} ${new Intl.DateTimeFormat(language === "ru" ? "ru-RU" : "en-GB", { dateStyle: "medium" }).format(new Date(session.accessExpiresAt * 1000))}`
                      : isRu
                        ? "Управляй доступом через BetterFy Bot."
                        : "Manage access through BetterFy Bot."}
            </p>
          </div>
          {preview ? (
            <button className="s-btn s-btn-primary" onClick={() => void signOut()}>
              <CircleUserRound />
              {isRu ? "Выбрать способ входа" : "Choose sign-in"}
            </button>
          ) : offline ? (
            <button className="s-btn" onClick={() => window.location.reload()}>
              {isRu ? "Повторить подключение" : "Reconnect"}
            </button>
          ) : noTelegram ? null : (
            <a className="s-btn" href="https://t.me/BeterFyBot" target="_blank" rel="noreferrer">
              {isRu ? "Управлять доступом" : "Manage access"}
              <ArrowUpRight />
            </a>
          )}
        </div>
      </section>
      {!preview && !offline && (
        <section className="s-profile-devices">
          <h2>{isRu ? "Устройства" : "Devices"}</h2>
          {loading ? (
            <LoaderCircle className="s-spin" />
          ) : error ? (
            <p className="s-form-error">
              {isRu ? "Не удалось загрузить список входов" : "Could not load sessions"}
            </p>
          ) : devices.length ? (
            devices.map((device) => (
              <div className="s-setting-row" key={device.sessionId}>
                <span className="s-setting-icon">
                  <Laptop />
                </span>
                <div>
                  <strong>
                    {device.current
                      ? isRu
                        ? "Это устройство"
                        : "This device"
                      : device.clientKind === "web"
                        ? isRu
                          ? "Браузер"
                          : "Browser"
                        : "BetterFy Desktop"}
                  </strong>
                  <p>
                    {new Intl.DateTimeFormat(language === "ru" ? "ru-RU" : "en-GB", {
                      dateStyle: "medium",
                    }).format(new Date(device.lastUsedAt * 1000))}
                  </p>
                </div>
                {device.current ? (
                  <span className="s-chip">{isRu ? "Текущий" : "Current"}</span>
                ) : (
                  <button
                    className="s-btn"
                    disabled={revoking === device.sessionId}
                    onClick={async () => {
                      setRevoking(device.sessionId);
                      setRevokeError(false);
                      try {
                        // ok:false means the session was not ended; keep it listed.
                        if (!(await revokeDeviceSession(session, device.sessionId)))
                          throw new Error("auth_session_revoke_failed");
                        setDevices((items) =>
                          items.filter((item) => item.sessionId !== device.sessionId),
                        );
                      } catch {
                        setRevokeError(true);
                      } finally {
                        setRevoking(null);
                      }
                    }}
                  >
                    {isRu ? "Завершить вход" : "End session"}
                  </button>
                )}
              </div>
            ))
          ) : (
            <p>{isRu ? "Других активных входов нет" : "No other active sessions"}</p>
          )}
          {revokeError && (
            <p className="s-form-error" role="alert">
              {isRu
                ? "Не удалось завершить этот вход. Попробуй ещё раз."
                : "Could not end that session. Please try again."}
            </p>
          )}
          <button className="s-text-button s-signout" onClick={() => void signOut()}>
            <LogOut />
            {isRu ? "Выйти из аккаунта" : "Sign out"}
          </button>
          {signOutError && (
            <p className="s-form-error" role="alert">
              {isRu
                ? "Не удалось выйти. Проверь подключение и попробуй ещё раз."
                : "Could not sign out. Check your connection and try again."}
            </p>
          )}
        </section>
      )}
      {!preview && !offline && session && <EmailIdentity session={session} />}
    </div>
  );
}
