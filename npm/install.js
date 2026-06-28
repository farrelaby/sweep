#!/usr/bin/env node
const { existsSync, createWriteStream, mkdirSync, chmodSync, writeFileSync, readFileSync, unlinkSync } = require("fs");
const { join } = require("path");
const { pipeline } = require("stream");
const { promisify } = require("util");
const https = require("https");
const http = require("http");
const { spawnSync } = require("child_process");

const pipe = promisify(pipeline);

const PKG = require("./package.json");
const VERSION = PKG.version;
const BIN_DIR = join(__dirname, "bin");
const BIN_PATH = join(BIN_DIR, process.platform === "win32" ? "dirsweep.exe" : "dirsweep");
const VERSION_FILE = join(BIN_DIR, "VERSION");
const TIMEOUT_MS = 30_000;

function platform() {
  const os = process.platform;
  const arch = process.arch;

  const map = {
    "win32-x64": "x86_64-pc-windows-msvc",
    "win32-arm64": "aarch64-pc-windows-msvc",
    "linux-x64": "x86_64-unknown-linux-gnu",
    "linux-arm64": "aarch64-unknown-linux-gnu",
    "darwin-x64": "x86_64-apple-darwin",
    "darwin-arm64": "aarch64-apple-darwin",
  };

  const key = `${os}-${arch}`;
  if (!map[key]) {
    console.error(`Unsupported platform: ${key}`);
    console.error(`dirsweep supports: ${Object.keys(map).join(", ")}`);
    process.exit(1);
  }
  return map[key];
}

function isWindows() {
  return process.platform === "win32";
}

function releaseUrl(target) {
  const ext = isWindows() ? "zip" : "tar.gz";
  return `https://github.com/farrelaby/dirsweep/releases/download/v${VERSION}/dirsweep-v${VERSION}-${target}.${ext}`;
}

function download(url, dest, redirects = 0) {
  const MAX_REDIRECTS = 10;
  const proto = url.startsWith("https") ? https : http;
  return new Promise((resolve, reject) => {
    const req = proto.get(url, { timeout: TIMEOUT_MS }, (res) => {
      if (res.statusCode >= 300 && res.statusCode < 400 && res.headers.location) {
        if (redirects >= MAX_REDIRECTS) {
          reject(new Error(`Too many redirects (${MAX_REDIRECTS}): ${url}`));
          return;
        }
        download(res.headers.location, dest, redirects + 1).then(resolve).catch(reject);
        return;
      }
      if (res.statusCode !== 200) {
        reject(new Error(`Download failed with status ${res.statusCode}: ${url}`));
        return;
      }
      const file = createWriteStream(dest);
      pipe(res, file).then(resolve).catch(reject);
    });
    req.on("timeout", () => {
      req.destroy();
      reject(new Error(`Download timed out after ${TIMEOUT_MS}ms: ${url}`));
    });
    req.on("error", reject);
  });
}

function isUpToDate() {
  if (!existsSync(VERSION_FILE)) return false;
  try {
    return readFileSync(VERSION_FILE, "utf8").trim() === VERSION;
  } catch {
    return false;
  }
}

async function install() {
  if (isUpToDate() && existsSync(BIN_PATH)) {
    return;
  }

  mkdirSync(BIN_DIR, { recursive: true });

  // Remove stale binary
  try { unlinkSync(BIN_PATH); } catch {}

  const target = platform();
  const url = releaseUrl(target);
  const archivePath = join(BIN_DIR, isWindows() ? "dirsweep.zip" : "dirsweep.tar.gz");

  console.log(`Downloading dirsweep v${VERSION} for ${target}...`);

  try {
    await download(url, archivePath);
  } catch (err) {
    console.error(`Failed to download from ${url}`);
    console.error(err.message);
    process.exit(1);
  }

  if (isWindows()) {
    const ps = spawnSync(
      "powershell",
      ["-NoProfile", "-Command", `Expand-Archive -Path '${archivePath}' -DestinationPath '${BIN_DIR}' -Force`],
      { stdio: "inherit" }
    );
    if (ps.status !== 0) {
      console.error("Failed to extract archive");
      process.exit(1);
    }
  } else {
    const tar = spawnSync("tar", ["-xzf", archivePath, "-C", BIN_DIR], {
      stdio: "inherit",
    });
    if (tar.status !== 0) {
      console.error("Failed to extract archive");
      process.exit(1);
    }
  }

  try { unlinkSync(archivePath); } catch {}

  chmodSync(BIN_PATH, 0o755);
  writeFileSync(VERSION_FILE, VERSION);
  console.log(`dirsweep v${VERSION} installed at ${BIN_PATH}`);
}

install().catch((err) => {
  console.error(err);
  process.exit(1);
});
