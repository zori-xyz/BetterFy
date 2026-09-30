// Builds and signs the remote package catalog from src-tauri/packages.
//
//   npm run catalog:publish   write index.json with the next sequence and sign it
//   npm run catalog:check     fail if the published index no longer matches the
//                             manifests in src-tauri/packages (no key needed)
//
// The signature uses the minisign format of `tauri signer`, with a key that is
// separate from the updater key. The private key never enters the repository:
// BETTERFY_CATALOG_KEY points to it (default ~/.betterfy/catalog-signing.key)
// and BETTERFY_CATALOG_KEY_PASSWORD holds its password, if it has one.

import { execFileSync } from "node:child_process";
import { existsSync, readFileSync, readdirSync, writeFileSync } from "node:fs";
import { homedir } from "node:os";
import path from "node:path";
import { fileURLToPath } from "node:url";

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const packagesDir = path.join(root, "src-tauri", "packages");
const outputDir = path.join(root, "website", "public", "bot", "catalog");
const indexPath = path.join(outputDir, "index.json");
const VALIDITY_DAYS = 180;

function manifests() {
  return readdirSync(packagesDir)
    .filter((name) => name.endsWith(".json"))
    .sort()
    .map((name) => JSON.parse(readFileSync(path.join(packagesDir, name), "utf8")));
}

function published() {
  return existsSync(indexPath) ? JSON.parse(readFileSync(indexPath, "utf8")) : null;
}

const mode = process.argv[2];
if (mode === "check") {
  const current = published();
  if (!current) throw new Error("website/public/bot/catalog/index.json is missing");
  if (JSON.stringify(current.packages) !== JSON.stringify(manifests())) {
    throw new Error("The published catalog is out of date. Run `npm run catalog:publish`.");
  }
  const remainingDays = (Date.parse(current.expiresAt) - Date.now()) / (24 * 60 * 60 * 1000);
  if (remainingDays < 7) {
    throw new Error(`The published catalog expires on ${current.expiresAt}. Run \`npm run catalog:publish\`.`);
  }
  if (remainingDays < 30) {
    console.warn(`Warning: the published catalog expires on ${current.expiresAt}; re-sign it soon.`);
  }
  if (!existsSync(`${indexPath}.sig`)) throw new Error("index.json.sig is missing");
  console.log(`Catalog sequence ${current.sequence} matches ${current.packages.length} manifests.`);
} else if (mode === "publish") {
  const previous = published();
  // Never reuse a sequence that is already live: clients reject a reused
  // sequence with different bytes. Publishing from two branches would do that.
  let live = 0;
  try {
    const response = await fetch("https://betterfy-auth.zori-xyz.workers.dev/catalog/index.json");
    if (response.ok) live = (await response.json()).sequence ?? 0;
  } catch {
    console.warn("Could not read the live catalog; using the committed sequence only.");
  }
  const now = new Date();
  const index = {
    schemaVersion: 1,
    sequence: Math.max(previous?.sequence ?? 0, live) + 1,
    issuedAt: now.toISOString().replace(/\.\d{3}Z$/, "Z"),
    expiresAt: new Date(now.getTime() + VALIDITY_DAYS * 24 * 60 * 60 * 1000)
      .toISOString()
      .replace(/\.\d{3}Z$/, "Z"),
    packages: manifests(),
  };
  writeFileSync(indexPath, `${JSON.stringify(index, null, 2)}\n`);
  const key = process.env.BETTERFY_CATALOG_KEY ?? path.join(homedir(), ".betterfy", "catalog-signing.key");
  execFileSync("npx", ["tauri", "signer", "sign", "-f", key, indexPath], {
    cwd: root,
    stdio: ["ignore", "ignore", "inherit"],
    env: {
      ...process.env,
      TAURI_SIGNING_PRIVATE_KEY_PASSWORD: process.env.BETTERFY_CATALOG_KEY_PASSWORD ?? "",
    },
  });
  console.log(`Signed catalog sequence ${index.sequence} with ${index.packages.length} packages.`);
} else {
  console.error("Usage: node scripts/catalog-index.mjs <publish|check>");
  process.exit(2);
}
