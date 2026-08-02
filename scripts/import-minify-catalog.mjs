import { copyFileSync, existsSync, mkdirSync, readdirSync, readFileSync, rmSync, statSync, writeFileSync } from "node:fs";
import { basename, join, relative } from "node:path";
import { execFileSync } from "node:child_process";

const sourceRoot = process.argv[2];
if (!sourceRoot) throw new Error("Pass the cloned dota2-minify repository path.");

const modsRoot = join(sourceRoot, "Minify", "mods");
const outputRoot = new URL("../src/assets/minify/", import.meta.url);
const outputJson = new URL("../src/minifyCatalog.json", import.meta.url);
const sourceUrl = "https://github.com/Egezenn/dota2-minify";
const commit = execFileSync("git", ["-C", sourceRoot, "rev-parse", "HEAD"], { encoding: "utf8" }).trim();
const snapshotDate = execFileSync("git", ["-C", sourceRoot, "show", "-s", "--format=%cs", "HEAD"], { encoding: "utf8" }).trim();

const metadata = {
  "Auto Accept Match": ["utility", "MeGaNeKoS", "Автопринятие матча"],
  "Custom Backgrounds": ["custom", "Egezenn", "Свои фоны"],
  "Custom Fonts": ["custom", "Egezenn", "Свои шрифты"],
  "Custom Hero Grids": ["custom", "Egezenn", "Свои сетки героев"],
  "Dark Terrain": ["world", "robbyz512", "Тёмный ландшафт"],
  "Minify Base Attacks": ["core", "robbyz512", "Облегчённые базовые атаки"],
  "Minify Spells & Items": ["core", "robbyz512", "Облегчённые способности и предметы"],
  "Misc Optimization": ["core", "robbyz512", "Общая оптимизация"],
  "Mute Ambient Sounds": ["audio", "robbyz512", "Убрать звуки окружения"],
  "Mute Default Announcer": ["audio", "Egezenn", "Убрать стандартного диктора"],
  "Mute Taunt Sounds": ["audio", "robbyz512", "Убрать звуки насмешек"],
  "Mute Voice Line Sounds": ["audio", "robbyz512", "Убрать реплики героев"],
  "OpenDotaGuides Guides": ["utility", "Egezenn", "Гайды OpenDotaGuides"],
  "Remove Foilage": ["world", "robbyz512", "Убрать растительность"],
  "Remove Hero Renders": ["interface", "Egezenn", "Убрать рендеры героев"],
  "Remove Main Menu Background": ["interface", "Egezenn", "Убрать фон главного меню"],
  "Remove Pings": ["audio", "robbyz512", "Убрать пинги"],
  "Remove River": ["world", "robbyz512", "Убрать реку"],
  "Remove Showcases": ["interface", "Egezenn", "Убрать витрины профиля"],
  "Remove Sprays": ["interface", "robbyz512", "Убрать граффити"],
  "Remove Weather Effects": ["world", "robbyz512", "Убрать погодные эффекты"],
  "Repopulate Unit Query HUD": ["interface", "MeGaNeKoS", "Вернуть расширенный HUD цели"],
  "Reposition & Rescale HUD": ["interface", "robbyz512", "Позиция и масштаб HUD"],
  "Revamp Hero Grid Layout": ["interface", "Egezenn", "Новая сетка выбора героев"],
  "Revert Ping Sounds": ["audio", "Egezenn", "Вернуть классические звуки пингов"],
  "Show NetWorth": ["interface", "MeGaNeKoS", "Показывать свой Net Worth"],
  "Stat Site Buttons": ["utility", "Egezenn", "Кнопки сервисов статистики"],
  "Transparent HUD": ["interface", "ZerdacK / Egezenn", "Прозрачный HUD"],
  "Tree Mod": ["world", "robbyz512", "Упрощённые деревья"],
  "User Styles": ["custom", "Egezenn", "Пользовательские стили"],
};

const slugify = (value) =>
  value.toLowerCase().replace(/&/g, "and").replace(/[^a-z0-9]+/g, "-").replace(/^-|-$/g, "");

const localizedNote = (markdown, language) => {
  const marker = `<!-- LANG:${language} -->`;
  const start = markdown.indexOf(marker);
  if (start < 0) return "";
  const bodyStart = start + marker.length;
  const next = markdown.indexOf("<!-- LANG:", bodyStart);
  return markdown
    .slice(bodyStart, next < 0 ? undefined : next)
    .trim()
    .replace(/\r/g, "")
    .replace(/`([^`]+)`/g, "$1")
    .replace(/^!!:\s*/gm, language === "RU" ? "Важно: " : "Note: ")
    .replace(/^-\s+/gm, "• ")
    .slice(0, 1800);
};

const countFiles = (root) => {
  if (!existsSync(root)) return 0;
  return readdirSync(root, { withFileTypes: true }).reduce(
    (count, entry) => count + (entry.isDirectory() ? countFiles(join(root, entry.name)) : 1),
    0,
  );
};

const lineCount = (file) =>
  existsSync(file)
    ? readFileSync(file, "utf8").split(/\r?\n/).filter((line) => line.trim() && !line.trim().startsWith("#")).length
    : 0;

rmSync(outputRoot, { recursive: true, force: true });
mkdirSync(outputRoot, { recursive: true });

const items = Object.entries(metadata).map(([sourceName, [category, author, russianName]]) => {
  const modRoot = join(modsRoot, sourceName);
  if (!statSync(modRoot).isDirectory()) throw new Error(`Missing Minify mod: ${sourceName}`);

  const notes = readFileSync(join(modRoot, "notes.md"), "utf8");
  const slug = slugify(sourceName);
  const jpg = join(modRoot, "preview.jpg");
  const png = join(modRoot, "preview.png");
  const previewSource = existsSync(jpg) ? jpg : existsSync(png) ? png : null;
  const preview = previewSource ? `${slug}.${basename(previewSource).split(".").at(-1)}` : null;
  if (previewSource) copyFileSync(previewSource, new URL(preview, outputRoot));

  const scripts = readdirSync(modRoot).filter((name) => name.endsWith(".py")).length;
  const includedFiles = countFiles(join(modRoot, "files")) + countFiles(join(modRoot, "files_uncompiled"));
  const stylingRules = lineCount(join(modRoot, "styling.css")) + lineCount(join(modRoot, "_styling.css"));
  const blacklistEntries = lineCount(join(modRoot, "blacklist.txt"));

  return {
    id: `minify-${slug}`,
    sourceName,
    name: { ru: russianName, en: sourceName },
    description: {
      ru: localizedNote(notes, "RU") || localizedNote(notes, "EN"),
      en: localizedNote(notes, "EN"),
    },
    category,
    author,
    preview,
    sourcePath: relative(sourceRoot, modRoot),
    evidence: { blacklistEntries, includedFiles, stylingRules, scripts },
  };
});

writeFileSync(
  outputJson,
  `${JSON.stringify({ source: sourceUrl, commit, snapshotDate, items }, null, 2)}\n`,
  "utf8",
);

console.log(`Imported ${items.length} Minify mods and ${items.filter((item) => item.preview).length} previews.`);
