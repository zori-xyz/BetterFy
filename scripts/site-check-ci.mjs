import { spawn } from "node:child_process";

const env = { ...process.env, BETTERFY_SITE_URL: "http://127.0.0.1:4174/BetterFy/" };
const preview = spawn(
  process.platform === "win32" ? "npm.cmd" : "npm",
  ["run", "site:preview", "--", "--host", "127.0.0.1", "--port", "4174"],
  { env, stdio: ["ignore", "pipe", "pipe"] },
);

let previewOutput = "";
preview.stdout.on("data", (chunk) => { previewOutput += chunk; });
preview.stderr.on("data", (chunk) => { previewOutput += chunk; });

async function waitForPreview() {
  for (let attempt = 0; attempt < 80; attempt += 1) {
    if (preview.exitCode !== null) throw new Error(`Website preview exited early.\n${previewOutput}`);
    try {
      const response = await fetch(env.BETTERFY_SITE_URL);
      if (response.ok) return;
    } catch {
      // The preview server is still starting.
    }
    await new Promise((resolve) => setTimeout(resolve, 250));
  }
  throw new Error(`Website preview did not become ready.\n${previewOutput}`);
}

try {
  await waitForPreview();
  const check = spawn(
    process.platform === "win32" ? "npm.cmd" : "npm",
    ["run", "site:check"],
    { env, stdio: "inherit" },
  );
  const exitCode = await new Promise((resolve, reject) => {
    check.once("error", reject);
    check.once("exit", (code) => resolve(code ?? 1));
  });
  if (exitCode !== 0) process.exitCode = exitCode;
} finally {
  preview.kill("SIGTERM");
}
