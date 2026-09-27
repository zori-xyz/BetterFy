const tauri = window.__TAURI__;
const hasTauri = Boolean(tauri && tauri.core);
const WEBVIEW2_DOWNLOAD_URL =
  "https://developer.microsoft.com/microsoft-edge/webview2#download-the-webview2-runtime";

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

async function init() {
  const pathField = document.getElementById("install-path");
  try {
    pathField.textContent = hasTauri ? await invoke("default_install_dir") : "%LOCALAPPDATA%\\BetterFy";
  } catch (err) {
    pathField.textContent = "%LOCALAPPDATA%\\BetterFy";
  }

  try {
    const present = hasTauri ? await invoke("check_webview2") : true;
    document.getElementById("webview2-warning").hidden = present;
  } catch (err) {
    // If the check itself fails, don't block the install screen on it.
  }
}

function closeWindow() {
  if (hasTauri && tauri.window) {
    tauri.window.getCurrentWindow().close();
  }
}

async function openWebview2Download() {
  try {
    await invoke("open_url", { url: WEBVIEW2_DOWNLOAD_URL });
  } catch (err) {
    // No bridge (browser preview) or the OS call failed; nothing more to do.
  }
}

async function runInstall() {
  const options = {
    createDesktopShortcut: document.getElementById("opt-desktop").checked,
    launchAfter: document.getElementById("opt-launch").checked,
  };

  showScreen("installing");

  try {
    const report = await invoke("run_install", { options });
    document.getElementById("finish-path").textContent = report.installDir;
    showScreen("finish");
  } catch (err) {
    document.getElementById("error-code").textContent = String(err);
    showScreen("error");
  }
}

document.getElementById("btn-cancel").addEventListener("click", closeWindow);
document.getElementById("btn-install").addEventListener("click", runInstall);
document.getElementById("btn-done").addEventListener("click", closeWindow);
document.getElementById("btn-close-error").addEventListener("click", closeWindow);
document.getElementById("btn-webview2").addEventListener("click", openWebview2Download);

init();
