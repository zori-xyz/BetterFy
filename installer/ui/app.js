const tauri = window.__TAURI__;
const hasTauri = Boolean(tauri && tauri.core);
const WEBVIEW2_DOWNLOAD_URL =
  "https://developer.microsoft.com/microsoft-edge/webview2#download-the-webview2-runtime";
const HELP_URL = "https://t.me/BeterFyBot";
const FALLBACK_PATH = "%LOCALAPPDATA%\\BetterFy";

const STRINGS = {
  ru: {
    "welcome.eyebrow": "BETTERFY · WINDOWS",
    "welcome.title": "Установка BetterFy",
    "welcome.titleUpdate": "Обновление BetterFy",
    "welcome.body1": "Мастер установит приложение для текущего пользователя. Права администратора не понадобятся.",
    "welcome.body1Update": "BetterFy уже установлен. Обновление заменит файлы приложения; аккаунт, сессии и наборы хранятся отдельно и не затрагиваются.",
    "welcome.pathLabel": "Путь установки: ",
    "welcome.optDesktop": "Создать ярлык на рабочем столе",
    "welcome.optLaunch": "Запустить BetterFy после установки",
    "welcome.cancel": "Отмена",
    "welcome.install": "Установить",
    "welcome.update": "Обновить",
    "welcome.existingNotice": "Уже установлена версия {version}.",
    "welcome.uninstallLink": "Удалить BetterFy",
    "welcome.confirmUninstall": "Удалить BetterFy? Приложение и ярлыки будут удалены. Аккаунт и настройки, сохранённые отдельно, не пострадают.",
    "webview2.body": "Не найден Microsoft Edge WebView2 Runtime — без него BetterFy не запустится. Установите его с официальной страницы Microsoft и повторите установку.",
    "webview2.button": "Скачать WebView2",
    "installing.eyebrow": "INSTALL",
    "installing.title": "Устанавливаем BetterFy",
    "installing.body": "Это займёт несколько секунд.",
    "installing.stepFiles": "Копируем файлы приложения",
    "installing.stepShortcuts": "Создаём ярлыки",
    "installing.stepRegistry": "Регистрируем в Windows",
    "finish.eyebrow": "READY",
    "finish.title": "BetterFy установлен",
    "finish.body": "Приложение готово к запуску.",
    "finish.pathLabel": "Путь: ",
    "finish.done": "Готово",
    "uninstalling.eyebrow": "УДАЛЕНИЕ",
    "uninstalling.title": "Удаляем BetterFy",
    "uninstalling.body": "Убираем ярлыки, запись в реестре и файлы приложения.",
    "uninstalled.eyebrow": "ГОТОВО",
    "uninstalled.title": "BetterFy удалён",
    "uninstalled.body": "Приложение, ярлыки и запись в реестре удалены.",
    "uninstalled.done": "Закрыть",
    "error.eyebrow": "ОШИБКА",
    "error.title": "Не получилось",
    "error.body": "Что-то помешало завершить операцию. Можно попробовать ещё раз или написать в поддержку — код ниже пригодится.",
    "error.body.app_running": "BetterFy сейчас запущен. Закройте приложение и нажмите «Повторить».",
    "error.body.payload_not_embedded": "В этой сборке установщика отсутствует само приложение. Нужна другая сборка — сообщите об этом в поддержку.",
    "error.body.missing_desktop_dir": "Не удалось найти папку рабочего стола Windows. Установка приложения могла пройти успешно — проверьте меню «Пуск».",
    "error.body.payload_verify_failed": "Файлы приложения записались повреждёнными. Ничего не было заменено — можно спокойно попробовать снова.",
    "error.body.uninstall_incomplete": "Удаление не завершилось полностью. Часть файлов, ярлыков или запись в реестре могли остаться — попробуйте ещё раз.",
    "error.codeLabel": "Код: ",
    "error.close": "Закрыть",
    "error.retry": "Повторить",
    "error.help": "Написать в поддержку",
  },
  en: {
    "welcome.eyebrow": "BETTERFY · WINDOWS",
    "welcome.title": "Install BetterFy",
    "welcome.titleUpdate": "Update BetterFy",
    "welcome.body1": "Setup installs the app for your Windows account. Administrator access is not required.",
    "welcome.body1Update": "BetterFy is already installed. Updating replaces the app files; your account, sessions and presets are stored separately and are not touched.",
    "welcome.pathLabel": "Install path: ",
    "welcome.optDesktop": "Create a desktop shortcut",
    "welcome.optLaunch": "Launch BetterFy after installing",
    "welcome.cancel": "Cancel",
    "welcome.install": "Install",
    "welcome.update": "Update",
    "welcome.existingNotice": "Version {version} is already installed.",
    "welcome.uninstallLink": "Uninstall BetterFy",
    "welcome.confirmUninstall": "Uninstall BetterFy? The app and its shortcuts will be removed. Your account and settings, stored separately, are not affected.",
    "webview2.body": "Microsoft Edge WebView2 Runtime was not found — BetterFy will not start without it. Install it from Microsoft's official page and run this installer again.",
    "webview2.button": "Download WebView2",
    "installing.eyebrow": "INSTALL",
    "installing.title": "Installing BetterFy",
    "installing.body": "This takes a few seconds.",
    "installing.stepFiles": "Copying app files",
    "installing.stepShortcuts": "Creating shortcuts",
    "installing.stepRegistry": "Registering with Windows",
    "finish.eyebrow": "READY",
    "finish.title": "BetterFy is installed",
    "finish.body": "The app is ready to open.",
    "finish.pathLabel": "Path: ",
    "finish.done": "Finish",
    "uninstalling.eyebrow": "UNINSTALLING",
    "uninstalling.title": "Uninstalling BetterFy",
    "uninstalling.body": "Removing shortcuts, the registry entry, and app files.",
    "uninstalled.eyebrow": "DONE",
    "uninstalled.title": "BetterFy is uninstalled",
    "uninstalled.body": "The app, its shortcuts, and the registry entry have been removed.",
    "uninstalled.done": "Close",
    "error.eyebrow": "ERROR",
    "error.title": "That didn't work",
    "error.body": "Something stopped this from finishing. You can try again, or contact support — the code below will help.",
    "error.body.app_running": "BetterFy is currently running. Close it and press Retry.",
    "error.body.payload_not_embedded": "This installer build has no app embedded in it. You need a different build — please report this.",
    "error.body.missing_desktop_dir": "Windows' Desktop folder could not be found. The app itself may have installed fine — check the Start menu.",
    "error.body.payload_verify_failed": "The app files were written but came out corrupted. Nothing was replaced, so it's safe to try again.",
    "error.body.uninstall_incomplete": "Uninstalling did not fully finish. Some files, shortcuts, or the registry entry may remain — try again.",
    "error.codeLabel": "Code: ",
    "error.close": "Close",
    "error.retry": "Retry",
    "error.help": "Contact support",
  },
};

let lang = localStorage.getItem("betterfy-installer-lang") || "ru";
let existing = null;
let lastAction = "install";

function t(key, vars) {
  let value = STRINGS[lang][key] ?? STRINGS.ru[key] ?? key;
  if (vars) {
    for (const [name, val] of Object.entries(vars)) {
      value = value.replace(`{${name}}`, val);
    }
  }
  return value;
}

function applyTranslations() {
  document.documentElement.lang = lang;
  document.querySelectorAll("[data-i18n]").forEach((el) => {
    const key = el.dataset.i18n;
    const children = Array.from(el.childNodes).filter((n) => n.nodeType !== Node.TEXT_NODE);
    el.textContent = t(key);
    children.forEach((child) => el.appendChild(child));
  });
  document.querySelectorAll(".lang-switch button").forEach((btn) => {
    btn.classList.toggle("is-active", btn.dataset.lang === lang);
  });
}

function setLang(next) {
  lang = next;
  localStorage.setItem("betterfy-installer-lang", lang);
  applyTranslations();
  updateWelcomeForExistingInstall();
}

function showScreen(name) {
  document.querySelectorAll(".screen").forEach((el) => {
    el.hidden = el.dataset.screen !== name;
  });
}

async function invoke(command, args) {
  if (!hasTauri) {
    throw new Error("tauri_bridge_unavailable");
  }
  return tauri.core.invoke(command, args);
}

function updateWelcomeForExistingInstall() {
  const notice = document.getElementById("existing-notice");
  const installBtn = document.getElementById("btn-install");
  if (existing) {
    notice.hidden = false;
    document.getElementById("existing-notice-text").textContent = t("welcome.existingNotice", {
      version: existing.version,
    });
    document.querySelector('[data-i18n="welcome.title"]').textContent = t("welcome.titleUpdate");
    document.querySelector('[data-i18n="welcome.body1"]').textContent = t("welcome.body1Update");
    installBtn.textContent = t("welcome.update");
  } else {
    notice.hidden = true;
    document.querySelector('[data-i18n="welcome.title"]').textContent = t("welcome.title");
    document.querySelector('[data-i18n="welcome.body1"]').textContent = t("welcome.body1");
    installBtn.textContent = t("welcome.install");
  }
}

async function init() {
  applyTranslations();

  const pathField = document.getElementById("install-path");
  try {
    pathField.textContent = hasTauri ? await invoke("default_install_dir") : FALLBACK_PATH;
  } catch (err) {
    pathField.textContent = FALLBACK_PATH;
  }

  try {
    existing = hasTauri ? await invoke("existing_install") : null;
  } catch (err) {
    existing = null;
  }
  updateWelcomeForExistingInstall();

  try {
    const present = hasTauri ? await invoke("check_webview2") : true;
    document.getElementById("webview2-warning").hidden = present;
  } catch (err) {
    // If the check itself fails, don't block the install screen on it.
  }
}

async function closeWindow() {
  if (!hasTauri || !tauri.window) {
    return;
  }
  try {
    await tauri.window.getCurrentWindow().close();
  } catch (err) {
    // Permission or IPC failure: surface it instead of a button that looks
    // broken with no explanation.
    showInstallError(err);
  }
}

async function openWebview2Download() {
  try {
    await invoke("open_url", { url: WEBVIEW2_DOWNLOAD_URL });
  } catch (err) {
    // No bridge (browser preview) or the OS call failed; nothing more to do.
  }
}

async function openHelp() {
  try {
    await invoke("open_url", { url: HELP_URL });
  } catch (err) {
    // Nothing more to do without a working bridge.
  }
}

function showInstallError(err) {
  const code = String(err);
  const specificKey = `error.body.${code}`;
  const bodyEl = document.getElementById("error-body");
  bodyEl.textContent = t(specificKey) !== specificKey ? t(specificKey) : t("error.body");
  document.getElementById("error-code").textContent = code;
  showScreen("error");
}

function markSteps(done) {
  document.querySelectorAll("#install-steps li").forEach((li) => {
    li.classList.toggle("done", done);
  });
}

async function runInstall() {
  lastAction = "install";
  const options = {
    createDesktopShortcut: document.getElementById("opt-desktop").checked,
    launchAfter: document.getElementById("opt-launch").checked,
  };

  markSteps(false);
  showScreen("installing");

  try {
    const report = await invoke("run_install", { options });
    markSteps(true);
    document.getElementById("finish-path").textContent = report.installDir;
    showScreen("finish");
  } catch (err) {
    showInstallError(err);
  }
}

async function runUninstall() {
  if (!window.confirm(t("welcome.confirmUninstall"))) {
    return;
  }
  lastAction = "uninstall";
  showScreen("uninstalling");
  try {
    await invoke("run_uninstall");
    showScreen("uninstalled");
  } catch (err) {
    showInstallError(err);
  }
}

function retryLastAction() {
  if (lastAction === "uninstall") {
    runUninstall();
  } else {
    runInstall();
  }
}

document.querySelectorAll(".lang-switch button").forEach((btn) => {
  btn.addEventListener("click", () => setLang(btn.dataset.lang));
});
document.getElementById("btn-cancel").addEventListener("click", closeWindow);
document.getElementById("btn-install").addEventListener("click", runInstall);
document.getElementById("btn-done").addEventListener("click", closeWindow);
document.getElementById("btn-uninstalled-done").addEventListener("click", closeWindow);
document.getElementById("btn-close-error").addEventListener("click", closeWindow);
document.getElementById("btn-error-retry").addEventListener("click", retryLastAction);
document.getElementById("btn-error-help").addEventListener("click", openHelp);
document.getElementById("btn-webview2").addEventListener("click", openWebview2Download);
document.getElementById("btn-uninstall").addEventListener("click", runUninstall);

init();
