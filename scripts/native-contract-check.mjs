import { chromium } from "playwright";

const base = process.env.BETTERFY_APP_URL ?? "http://127.0.0.1:1420/";
const browser = await chromium.launch();

async function nativePage(withInstallation = false) {
  const page = await browser.newPage({ viewport: { width: 1180, height: 820 }, reducedMotion: "reduce" });
  await page.addInitScript(({ withInstallation }) => {
    window.__avatarFetches = 0;
    if (withInstallation) localStorage.setItem("betterfy:game-installation", JSON.stringify({
      path: "Preview / Dota 2", executablePath: "Preview only", steamLibrary: "Test fixture",
      client: "Prototype preview", source: "demo", verified: false,
    }));
    const localPreset = (id, updatedAt) => ({
      schemaVersion: 1, id, name: id, description: "", author: "Local profile",
      version: "1.0.0", modIds: ["minify-misc-optimization"], wardrobeIds: [],
      source: "local", readOnly: false, createdAt: updatedAt, updatedAt,
    });
    window.__TAURI_INTERNALS__ = {
      invoke: async (command) => {
        if (command === "auth_restore_session") return null;
        if (command === "list_presets") return [
          localPreset("local.native-date", "unix:1789905600000"),
          localPreset("local.invalid-date", "not-a-date"),
        ];
        if (command === "auth_begin_device_challenge") return {
          deepLink: `https://t.me/BeterFyBot?start=auth_${"A".repeat(43)}`,
          expiresAt: Math.floor(Date.now() / 1000) + 600,
          pollAfterSeconds: 1,
        };
        if (command === "auth_poll_device_challenge") return {
          state: "confirmed", profile: {
            userId: "test-player", displayName: "Test Player", username: null,
            accessTier: "early-access", accessExpiresAt: null, accessPlan: null,
            accessRecurring: null, sessionId: null, avatarAvailable: true,
          },
        };
        if (command === "auth_fetch_avatar") {
          window.__avatarFetches += 1;
          return {
            contentType: "image/png",
            bytes: Array.from(Uint8Array.from(atob("iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVQIHWP4z8DwHwAFgAI/ScLytQAAAABJRU5ErkJggg=="), char => char.charCodeAt(0))),
          };
        }
        if (command === "auth_list_sessions") return [];
        return null;
      },
    };
  }, { withInstallation });
  return page;
}

try {
  const library = await nativePage();
  const libraryErrors = [];
  library.on("pageerror", error => libraryErrors.push(error.message));
  await library.goto(base);
  await library.getByRole("button", { name: "Посмотреть приложение" }).click();
  await library.locator(".s-look-carousel").waitFor();
  if (await library.locator(".s-emote-tools, .s-carousel-pause").count()) {
    throw new Error("Animation controls remained visible on Home");
  }
  await library.getByRole("navigation", { name: "Основная навигация" }).getByRole("button", { name: "Библиотека" }).click();
  await library.getByRole("heading", { name: "Твоя библиотека" }).waitFor();
  await library.getByRole("heading", { name: "local.native-date" }).waitFor();
  await library.getByText("Дата неизвестна").waitFor();
  if (libraryErrors.length) throw new Error(`Library render failed: ${libraryErrors.join("; ")}`);
  await library.close();

  const auth = await nativePage(true);
  const authErrors = [];
  auth.on("pageerror", error => authErrors.push(error.message));
  await auth.goto(base);
  await auth.getByRole("button", { name: /Войти через Telegram/ }).click();
  await auth.getByRole("navigation", { name: "Основная навигация" }).waitFor({ timeout: 10000 });
  await auth.locator(".s-sidebar-profile .s-mini-avatar img").waitFor();
  const avatarFetchesBeforeProfile = await auth.evaluate(() => window.__avatarFetches);
  await auth.locator(".s-sidebar-profile").click();
  await auth.locator(".s-profile-person .s-avatar img").waitFor();
  if (await auth.evaluate(() => window.__avatarFetches) !== avatarFetchesBeforeProfile) {
    throw new Error("Opening Profile triggered another avatar request");
  }
  if (authErrors.length) throw new Error(`Native auth failed: ${authErrors.join("; ")}`);
  await auth.close();

  console.log("Native contracts: saved unix dates render; nullable Telegram profile opens workspace; avatar is shared by sidebar and profile; animation controls are absent.");
} finally {
  await browser.close();
}
