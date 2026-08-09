import { readFileSync, statSync } from "node:fs";
import { fileURLToPath } from "node:url";
import path from "node:path";

const projectRoot = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const assetDir = path.join(projectRoot, "src-tauri", "windows", "installer");
const expected = [
  ["sidebar.bmp", 164, 314],
  ["header.bmp", 150, 57],
];

for (const [name, width, height] of expected) {
  const file = path.join(assetDir, name);
  const bytes = readFileSync(file);
  if (bytes.length < 54 || bytes.toString("ascii", 0, 2) !== "BM") {
    throw new Error(`${name} is not a Windows BMP file`);
  }
  const actualWidth = bytes.readInt32LE(18);
  const actualHeight = Math.abs(bytes.readInt32LE(22));
  const bitsPerPixel = bytes.readUInt16LE(28);
  if (actualWidth !== width || actualHeight !== height) {
    throw new Error(`${name} is ${actualWidth}x${actualHeight}; expected ${width}x${height}`);
  }
  if (![24, 32].includes(bitsPerPixel)) {
    throw new Error(`${name} uses unsupported ${bitsPerPixel}-bit pixels`);
  }
  if (statSync(file).size > 1024 * 1024) {
    throw new Error(`${name} is unexpectedly large`);
  }
}

const template = readFileSync(path.join(projectRoot, "src-tauri", "windows", "installer.nsi"), "utf8");
const requiredTemplateContracts = [
  "MUI_FINISHPAGE_SHOWREADME_FUNCTION CreateOrUpdateDesktopShortcut",
  "MUI_FINISHPAGE_RUN_FUNCTION RunMainBinary",
  "https://zori-xyz.github.io/BetterFy/",
  "https://github.com/zori-xyz",
  "Function BetterFyInstFilesShow",
  "Function BetterFyFinishShow",
];

for (const contract of requiredTemplateContracts) {
  if (!template.includes(contract)) {
    throw new Error(`installer.nsi is missing required contract: ${contract}`);
  }
}

console.log("BetterFy installer artwork and finish actions are valid.");
