#!/usr/bin/env node
const { existsSync } = require("fs");
const { join } = require("path");
const { spawnSync } = require("child_process");

const binPath = join(__dirname, "bin", process.platform === "win32" ? "dirsweep.exe" : "dirsweep");

if (!existsSync(binPath)) {
  console.error("dirsweep binary not found. Run `npm install` or `npx dirsweep` again.");
  process.exit(1);
}

const result = spawnSync(binPath, process.argv.slice(2), {
  stdio: "inherit",
  windowsHide: true,
});

if (result.status !== null) {
  process.exit(result.status);
} else if (result.signal) {
  const signals = { SIGTERM: 143, SIGKILL: 137, SIGINT: 130 };
  process.exit(signals[result.signal] ?? 128);
} else {
  process.exit(1);
}
