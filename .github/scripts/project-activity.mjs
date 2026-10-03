#!/usr/bin/env node
// Renders the README "Project activity" card: commits and GitHub Actions
// workflow runs per day, as dark and light SVG variants.
//
// Series:
// - Commits: non-merge commits reachable from any remote-tracking branch
//   (`git log --remotes`), each counted once, keyed by author date in UTC.
//   The branch that stores this chart is excluded. Merge commits are left
//   out because they record an integration, not new work.
// - CI runs: GitHub Actions workflow runs, keyed by creation time in UTC.
//   Runs of this chart's own workflow and runs that concluded as "skipped"
//   are left out.
// The two series are different units. They share a count axis but are drawn
// side by side, never stacked or summed.
//
// Usage:
//   GITHUB_TOKEN=... node .github/scripts/project-activity.mjs
//     [--repo owner/name] [--days <n>] [--today YYYY-MM-DD] [--out <dir>]
//
// The token needs read access to Actions. Plain Node and git only; no npm
// dependencies.

import { execFileSync } from "node:child_process";
import { mkdirSync, writeFileSync } from "node:fs";
import { resolve } from "node:path";

const DAY_MS = 86_400_000;
const CHART_BRANCH = "project-activity";
const CHART_WORKFLOW = ".github/workflows/project-activity.yml";

function parseArgs(argv) {
  const options = {
    repo: process.env.GITHUB_REPOSITORY ?? "zori-xyz/BetterFy",
    days: 60,
    today: null,
    out: null,
  };
  for (let i = 0; i < argv.length; i += 1) {
    const key = argv[i];
    const value = argv[i + 1];
    if (key === "--repo") options.repo = value;
    else if (key === "--days") options.days = Number.parseInt(value, 10);
    else if (key === "--today") options.today = value;
    else if (key === "--out") options.out = value;
    else throw new Error(`Unknown argument: ${key}`);
    i += 1;
  }
  if (!/^[\w.-]+\/[\w.-]+$/.test(options.repo)) throw new Error("--repo must be owner/name");
  if (!Number.isInteger(options.days) || options.days < 7 || options.days > 365) {
    throw new Error("--days must be an integer between 7 and 365");
  }
  if (options.today !== null && !/^\d{4}-\d{2}-\d{2}$/.test(options.today)) {
    throw new Error("--today must be YYYY-MM-DD");
  }
  return options;
}

function utcDay(date) {
  return new Date(date).toISOString().slice(0, 10);
}

function readCommitDays() {
  const output = execFileSync(
    "git",
    [
      "log",
      `--exclude=refs/remotes/*/${CHART_BRANCH}`,
      "--remotes",
      "--no-merges",
      "--format=%H %at",
    ],
    { encoding: "utf8", maxBuffer: 64 * 1024 * 1024 },
  );
  const seen = new Map();
  for (const line of output.split("\n")) {
    if (!line) continue;
    const [hash, seconds] = line.split(" ");
    if (!seen.has(hash)) seen.set(hash, utcDay(Number(seconds) * 1000));
  }
  if (seen.size === 0) {
    throw new Error("No commits found on remote-tracking branches; fetch with full history first.");
  }
  return [...seen.values()];
}

async function readRunDays(repo) {
  const token = process.env.GITHUB_TOKEN ?? process.env.GH_TOKEN;
  if (!token) throw new Error("Set GITHUB_TOKEN or GH_TOKEN to read workflow runs.");
  const days = [];
  for (let page = 1; ; page += 1) {
    const response = await fetch(
      `https://api.github.com/repos/${repo}/actions/runs?per_page=100&page=${page}`,
      {
        headers: {
          Accept: "application/vnd.github+json",
          Authorization: `Bearer ${token}`,
          "X-GitHub-Api-Version": "2022-11-28",
          "User-Agent": "betterfy-project-activity",
        },
      },
    );
    if (!response.ok) {
      throw new Error(`Workflow runs request failed: ${response.status} ${await response.text()}`);
    }
    const body = await response.json();
    for (const run of body.workflow_runs) {
      if (run.conclusion === "skipped") continue;
      if ((run.path ?? "").split("@")[0] === CHART_WORKFLOW) continue;
      days.push(utcDay(run.created_at));
    }
    if (body.workflow_runs.length < 100) break;
  }
  return days;
}

function summarize(commitDays, runDays, days, today) {
  const end = Date.parse(`${today}T00:00:00Z`);
  const buckets = [];
  const index = new Map();
  for (let i = days - 1; i >= 0; i -= 1) {
    const day = utcDay(end - i * DAY_MS);
    index.set(day, buckets.length);
    buckets.push({ day, commits: 0, runs: 0 });
  }
  const since = commitDays.reduce((min, d) => (d < min ? d : min), commitDays[0]);
  for (const d of commitDays) {
    const slot = index.get(d);
    if (slot !== undefined) buckets[slot].commits += 1;
  }
  let runs = 0;
  for (const d of runDays) {
    if (d < since || d > today) continue;
    runs += 1;
    const slot = index.get(d);
    if (slot !== undefined) buckets[slot].runs += 1;
  }
  const commits = commitDays.filter((d) => d <= today).length;
  return { buckets, commits, runs, since, today };
}

// Brand tokens from DESIGN.md and src/studio/studio-tokens.css.
const THEMES = {
  dark: {
    background: "#0D0D13",
    border: "#26252F",
    title: "#F7F5FB",
    muted: "#8F8C9B",
    quiet: "#5D5A67",
    grid: "#1E1D27",
    empty: "#24232D",
    commits: "#D9C6FF",
    runs: "#A84DFF",
  },
  light: {
    background: "#FFFFFF",
    border: "#D8D1DF",
    title: "#211C27",
    muted: "#665F6D",
    quiet: "#8A8391",
    grid: "#EEEAF3",
    empty: "#E5E0E9",
    commits: "#A98AE6",
    runs: "#5F1CB3",
  },
};

const FONT =
  "-apple-system, BlinkMacSystemFont, 'Segoe UI', 'Noto Sans', Helvetica, Arial, sans-serif";

function escapeXml(value) {
  return String(value)
    .replaceAll("&", "&amp;")
    .replaceAll("<", "&lt;")
    .replaceAll(">", "&gt;")
    .replaceAll('"', "&quot;");
}

function round(value) {
  return Math.round(value * 100) / 100;
}

// Rectangle with only the top corners rounded, so bars sit flat on the baseline.
function topRoundedBar(x, y, width, height, radius) {
  const r = Math.min(radius, width / 2, height);
  return (
    `M${round(x)} ${round(y + height)}` +
    `V${round(y + r)}` +
    `Q${round(x)} ${round(y)} ${round(x + r)} ${round(y)}` +
    `H${round(x + width - r)}` +
    `Q${round(x + width)} ${round(y)} ${round(x + width)} ${round(y + r)}` +
    `V${round(y + height)}Z`
  );
}

function niceMax(value) {
  if (value <= 4) return 4;
  const steps = [5, 10, 15, 20, 25, 30, 40, 50, 60, 80, 100, 150, 200, 300, 500];
  for (const step of steps) if (value <= step) return step;
  return Math.ceil(value / 100) * 100;
}

function renderSvg(summary, themeName) {
  const t = THEMES[themeName];
  const width = 880;
  const height = 336;
  const padX = 32;
  const gutter = 28; // room for y-axis labels
  const plotLeft = padX + gutter;
  const plotRight = width - padX;
  const plotTop = 108;
  const plotBottom = 242;
  const plotHeight = plotBottom - plotTop;
  const plotWidth = plotRight - plotLeft;

  const { buckets } = summary;
  const peak = Math.max(0, ...buckets.flatMap((b) => [b.commits, b.runs]));
  const scaleMax = niceMax(peak);
  const slot = plotWidth / buckets.length;
  const pairWidth = Math.max(5, slot * 0.72);
  const seam = 1;
  const barWidth = (pairWidth - seam) / 2;
  const unit = plotHeight / scaleMax;

  const bars = [];
  buckets.forEach((bucket, i) => {
    const x0 = plotLeft + i * slot + (slot - pairWidth) / 2;
    const series = [
      { value: bucket.commits, color: t.commits, x: x0 },
      { value: bucket.runs, color: t.runs, x: x0 + barWidth + seam },
    ];
    const parts = series.map(({ value, color, x }) => {
      if (value === 0) {
        return `<rect x="${round(x)}" y="${plotBottom - 2}" width="${round(barWidth)}" height="2" fill="${t.empty}"/>`;
      }
      const h = value * unit;
      return `<path d="${topRoundedBar(x, plotBottom - h, barWidth, h, 1.5)}" fill="${color}"/>`;
    });
    const label = `${bucket.day}: ${bucket.commits} commits, ${bucket.runs} CI runs`;
    bars.push(`<g><title>${escapeXml(label)}</title>${parts.join("")}</g>`);
  });

  const grid = [scaleMax, scaleMax / 2]
    .map((value) => {
      const y = round(plotBottom - value * unit);
      return (
        `<line x1="${plotLeft}" y1="${y}" x2="${plotRight}" y2="${y}" stroke="${t.grid}" stroke-width="1" stroke-dasharray="2 4"/>` +
        `<text x="${plotLeft - 10}" y="${y + 4}" text-anchor="end" class="axis">${Number.isInteger(value) ? value : value.toFixed(1)}</text>`
      );
    })
    .join("");

  // Legend, right-aligned. Widths are estimated for a 12px system sans.
  const charWidth = 6.7;
  const legendItems = [
    { label: "commits", color: t.commits },
    { label: "CI runs", color: t.runs },
  ];
  let cursor = plotRight;
  const legend = [];
  for (const item of [...legendItems].reverse()) {
    const textWidth = item.label.length * charWidth;
    legend.unshift(
      `<rect x="${round(cursor - textWidth - 16)}" y="35" width="10" height="10" rx="2.5" fill="${item.color}"/>` +
        `<text x="${round(cursor)}" y="44.5" text-anchor="end" class="legend">${item.label}</text>`,
    );
    cursor -= textWidth + 16 + 22;
  }

  const first = buckets[0].day;
  const last = buckets[buckets.length - 1].day;
  const windowCommits = buckets.reduce((sum, b) => sum + b.commits, 0);
  const windowRuns = buckets.reduce((sum, b) => sum + b.runs, 0);
  const desc =
    `Commits and GitHub Actions workflow runs per day on BetterFy from ${first} to ${last}, ` +
    `drawn as two separate bars per day: ${windowCommits} commits and ${windowRuns} CI runs in this window. ` +
    `${summary.commits} commits and ${summary.runs} CI runs since ${summary.since}.`;

  return `<svg xmlns="http://www.w3.org/2000/svg" width="${width}" height="${height}" viewBox="0 0 ${width} ${height}" role="img" aria-labelledby="title desc">
<title id="title">Project activity</title>
<desc id="desc">${escapeXml(desc)}</desc>
<style>
text{font-family:${FONT};font-variant-numeric:tabular-nums}
.title{font-size:20px;font-weight:600;fill:${t.title};letter-spacing:-.01em}
.subtitle{font-size:13px;fill:${t.muted}}
.legend{font-size:12px;fill:${t.muted}}
.axis{font-size:11px;fill:${t.quiet}}
.footer{font-size:13px;fill:${t.muted}}
.footer tspan.strong{fill:${t.title};font-weight:600}
</style>
<rect x="0.5" y="0.5" width="${width - 1}" height="${height - 1}" rx="20" fill="${t.background}" stroke="${t.border}"/>
<text x="${padX}" y="50" class="title">Project activity</text>
<text x="${padX}" y="74" class="subtitle">Commits and CI runs per day over the last ${buckets.length} days, updated ${escapeXml(summary.today)}</text>
${legend.join("\n")}
${grid}
<line x1="${plotLeft}" y1="${plotBottom + 0.5}" x2="${plotRight}" y2="${plotBottom + 0.5}" stroke="${t.border}" stroke-width="1"/>
<text x="${plotLeft - 10}" y="${plotBottom + 4}" text-anchor="end" class="axis">0</text>
${bars.join("\n")}
<text x="${plotLeft}" y="${plotBottom + 22}" class="axis">${first}</text>
<text x="${plotRight}" y="${plotBottom + 22}" text-anchor="end" class="axis">${last}</text>
<line x1="${padX}" y1="${height - 54.5}" x2="${width - padX}" y2="${height - 54.5}" stroke="${t.grid}" stroke-width="1"/>
<text x="${padX}" y="${height - 24}" class="footer"><tspan class="strong">${summary.commits}</tspan> commits<tspan dx="10">/</tspan><tspan dx="10" class="strong">${summary.runs}</tspan> CI runs since ${escapeXml(summary.since)}</text>
</svg>
`;
}

async function main() {
  const options = parseArgs(process.argv.slice(2));
  const today = options.today ?? utcDay(Date.now());
  const out = resolve(options.out ?? "project-activity");
  const commitDays = readCommitDays();
  const runDays = await readRunDays(options.repo);
  const summary = summarize(commitDays, runDays, options.days, today);
  mkdirSync(out, { recursive: true });
  for (const theme of Object.keys(THEMES)) {
    const file = resolve(out, `project-activity-${theme}.svg`);
    writeFileSync(file, renderSvg(summary, theme));
    console.log(file);
  }
  console.log(`${summary.commits} commits / ${summary.runs} CI runs since ${summary.since}`);
}

await main();
