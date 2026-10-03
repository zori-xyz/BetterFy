#!/usr/bin/env node
// Renders the README "Project activity" card from local git history.
//
// Output: .github/assets/project-activity-{dark,light}.svg
//
// Counting rules:
// - Non-merge commits reachable from the chosen ref (default: HEAD). Merge
//   commits are excluded because they record an integration, not new work.
// - Days are calendar days in UTC, keyed by author date.
// - A commit is "from CI" when its author is an automation account: a name
//   or email containing "[bot]" (GitHub App and Actions identities such as
//   github-actions[bot]) or the legacy Actions address action@github.com.
//   Everything else counts as "by hand". The committer is ignored on purpose:
//   pull requests merged in the GitHub web interface are committed by
//   "GitHub <noreply@github.com>" but were still written by a person.
//
// Usage:
//   node .github/scripts/project-activity.mjs [--ref <rev>] [--days <n>]
//        [--today YYYY-MM-DD] [--out <dir>]
//
// Plain Node and git only; no npm dependencies.

import { execFileSync } from "node:child_process";
import { mkdirSync, writeFileSync } from "node:fs";
import { resolve } from "node:path";

const DAY_MS = 86_400_000;

function parseArgs(argv) {
  const options = { ref: "HEAD", days: 60, today: null, out: null };
  for (let i = 0; i < argv.length; i += 1) {
    const key = argv[i];
    const value = argv[i + 1];
    if (key === "--ref") options.ref = value;
    else if (key === "--days") options.days = Number.parseInt(value, 10);
    else if (key === "--today") options.today = value;
    else if (key === "--out") options.out = value;
    else throw new Error(`Unknown argument: ${key}`);
    i += 1;
  }
  if (!Number.isInteger(options.days) || options.days < 7 || options.days > 365) {
    throw new Error("--days must be an integer between 7 and 365");
  }
  if (options.today !== null && !/^\d{4}-\d{2}-\d{2}$/.test(options.today)) {
    throw new Error("--today must be YYYY-MM-DD");
  }
  return options;
}

function isAutomationAuthor(name, email) {
  const n = name.toLowerCase();
  const e = email.toLowerCase();
  return n.includes("[bot]") || e.includes("[bot]") || e === "action@github.com";
}

function readCommits(ref) {
  const output = execFileSync(
    "git",
    ["log", "--no-merges", "--format=%at%x09%an%x09%ae", ref, "--"],
    { encoding: "utf8", maxBuffer: 64 * 1024 * 1024 },
  );
  return output
    .split("\n")
    .filter(Boolean)
    .map((line) => {
      const [seconds, name, email] = line.split("\t");
      return {
        day: new Date(Number(seconds) * 1000).toISOString().slice(0, 10),
        ci: isAutomationAuthor(name ?? "", email ?? ""),
      };
    });
}

function summarize(commits, days, today) {
  const end = Date.parse(`${today}T00:00:00Z`);
  const buckets = [];
  const index = new Map();
  for (let i = days - 1; i >= 0; i -= 1) {
    const day = new Date(end - i * DAY_MS).toISOString().slice(0, 10);
    index.set(day, buckets.length);
    buckets.push({ day, hand: 0, ci: 0 });
  }
  let hand = 0;
  let ci = 0;
  let first = null;
  for (const commit of commits) {
    if (commit.ci) ci += 1;
    else hand += 1;
    if (first === null || commit.day < first) first = commit.day;
    const slot = index.get(commit.day);
    if (slot !== undefined) buckets[slot][commit.ci ? "ci" : "hand"] += 1;
  }
  return { buckets, total: hand + ci, hand, ci, first, today };
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
    hand: "#D9C6FF",
    ci: "#A84DFF",
  },
  light: {
    background: "#FFFFFF",
    border: "#D8D1DF",
    title: "#211C27",
    muted: "#665F6D",
    quiet: "#8A8391",
    grid: "#EEEAF3",
    empty: "#E5E0E9",
    hand: "#A98AE6",
    ci: "#5F1CB3",
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

// Rectangle with only the top corners rounded, so stacked bars sit flat on the
// baseline and flat against each other.
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
  const peak = Math.max(0, ...buckets.map((b) => b.hand + b.ci));
  const scaleMax = niceMax(peak);
  const slot = plotWidth / buckets.length;
  const barWidth = Math.max(3, slot * 0.66);
  const unit = plotHeight / scaleMax;
  const windowTotal = buckets.reduce((sum, b) => sum + b.hand + b.ci, 0);

  const bars = [];
  buckets.forEach((bucket, i) => {
    const x = plotLeft + i * slot + (slot - barWidth) / 2;
    const count = bucket.hand + bucket.ci;
    if (count === 0) {
      bars.push(
        `<rect x="${round(x)}" y="${plotBottom - 2}" width="${round(barWidth)}" height="2" rx="1" fill="${t.empty}"/>`,
      );
      return;
    }
    const handHeight = bucket.hand * unit;
    const ciHeight = bucket.ci * unit;
    const label = `${bucket.day}: ${bucket.hand} by hand, ${bucket.ci} from CI`;
    const parts = [];
    if (bucket.ci > 0 && bucket.hand > 0) {
      // Hand-written work sits on the baseline; CI work stacks on top with a
      // 1px seam so the two segments stay distinguishable in both themes.
      parts.push(
        `<rect x="${round(x)}" y="${round(plotBottom - handHeight)}" width="${round(barWidth)}" height="${round(handHeight)}" fill="${t.hand}"/>`,
      );
      const ciTop = plotBottom - handHeight - 1 - ciHeight;
      parts.push(
        `<path d="${topRoundedBar(x, ciTop, barWidth, ciHeight, 2)}" fill="${t.ci}"/>`,
      );
    } else {
      const h = bucket.ci > 0 ? ciHeight : handHeight;
      parts.push(
        `<path d="${topRoundedBar(x, plotBottom - h, barWidth, h, 2)}" fill="${bucket.ci > 0 ? t.ci : t.hand}"/>`,
      );
    }
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
    { label: "by hand", color: t.hand },
    { label: "from CI", color: t.ci },
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
  const since = summary.first ?? summary.today;
  const footer = `${summary.total} commits since ${since}  /  ${summary.hand} by hand  /  ${summary.ci} from CI`;
  const desc =
    `Stacked bar chart of commits per day on BetterFy from ${first} to ${last}: ` +
    `${windowTotal} commits in this window, peak ${peak} in one day. ` +
    `${footer.replaceAll("  /  ", ", ")}.`;

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
<text x="${padX}" y="74" class="subtitle">Commits per day over the last ${buckets.length} days, updated ${escapeXml(summary.today)}</text>
${legend.join("\n")}
${grid}
<line x1="${plotLeft}" y1="${plotBottom + 0.5}" x2="${plotRight}" y2="${plotBottom + 0.5}" stroke="${t.border}" stroke-width="1"/>
<text x="${plotLeft - 10}" y="${plotBottom + 4}" text-anchor="end" class="axis">0</text>
${bars.join("\n")}
<text x="${plotLeft}" y="${plotBottom + 22}" class="axis">${first}</text>
<text x="${plotRight}" y="${plotBottom + 22}" text-anchor="end" class="axis">${last}</text>
<line x1="${padX}" y1="${height - 54.5}" x2="${width - padX}" y2="${height - 54.5}" stroke="${t.grid}" stroke-width="1"/>
<text x="${padX}" y="${height - 24}" class="footer"><tspan class="strong">${summary.total}</tspan> commits since ${escapeXml(since)}<tspan dx="10">/</tspan><tspan dx="10" class="strong">${summary.hand}</tspan> by hand<tspan dx="10">/</tspan><tspan dx="10" class="strong">${summary.ci}</tspan> from CI</text>
</svg>
`;
}

function main() {
  const options = parseArgs(process.argv.slice(2));
  const today = options.today ?? new Date().toISOString().slice(0, 10);
  const out = resolve(options.out ?? resolve(import.meta.dirname, "../assets"));
  const summary = summarize(readCommits(options.ref), options.days, today);
  mkdirSync(out, { recursive: true });
  for (const theme of Object.keys(THEMES)) {
    const file = resolve(out, `project-activity-${theme}.svg`);
    writeFileSync(file, renderSvg(summary, theme));
    console.log(file);
  }
  console.log(
    `${summary.total} commits since ${summary.first} / ${summary.hand} by hand / ${summary.ci} from CI`,
  );
}

main();
