import { access, readFile } from "node:fs/promises";
import { constants } from "node:fs";
import process from "node:process";

const entryPath = new URL("../src/main.tsx", import.meta.url);
const legacyRoots = [
  "../src/App.tsx",
  "../src/ModCatalogRoute.tsx",
  "../src/MinifyModsRoute.tsx",
  "../src/PresetManager.tsx",
].map(path => new URL(path, import.meta.url));

const entry = await readFile(entryPath, "utf8");
const errors = [];

if (!/from\s+["']\.\/studio\/StudioApp["']/.test(entry)) {
  errors.push("src/main.tsx must use src/studio/StudioApp.tsx as the product entry.");
}

if (/from\s+["']\.\/App["']/.test(entry)) {
  errors.push("src/main.tsx must not restore the retired parallel App.tsx entry.");
}

for (const path of legacyRoots) {
  try {
    await access(path, constants.F_OK);
    errors.push(`${path.pathname.split("/").slice(-2).join("/")} is retired and must not be restored.`);
  } catch {
    // Missing is the expected state.
  }
}

if (errors.length > 0) {
  console.error(errors.join("\n"));
  process.exit(1);
}

console.log("Frontend entry verified: src/studio/StudioApp.tsx");
