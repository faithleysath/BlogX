#!/usr/bin/env node

const { createWriteStream, existsSync, mkdirSync, rmSync, chmodSync } = require("node:fs");
const { mkdtemp } = require("node:fs/promises");
const https = require("node:https");
const os = require("node:os");
const path = require("node:path");
const { pipeline } = require("node:stream/promises");
const { spawnSync } = require("node:child_process");

const version = require("../package.json").version;
const repo = "faithleysath/BlogX";
const checkOnly = process.argv.includes("--check");

function targetTriple() {
  const platform = process.platform;
  const arch = process.arch;

  if (platform === "linux" && arch === "x64") {
    return "x86_64-unknown-linux-gnu";
  }
  if (platform === "linux" && arch === "arm64") {
    return "aarch64-unknown-linux-gnu";
  }
  if (platform === "darwin" && arch === "x64") {
    return "x86_64-apple-darwin";
  }
  if (platform === "darwin" && arch === "arm64") {
    return "aarch64-apple-darwin";
  }

  throw new Error(`Unsupported platform: ${platform} ${arch}. BlogX prebuilt npm installs currently support Linux glibc and macOS.`);
}

function download(url, destination) {
  return new Promise((resolve, reject) => {
    const request = https.get(
      url,
      {
        headers: {
          "User-Agent": "@faithleysath/blogx installer"
        }
      },
      (response) => {
        if ([301, 302, 303, 307, 308].includes(response.statusCode)) {
          response.resume();
          download(response.headers.location, destination).then(resolve, reject);
          return;
        }
        if (response.statusCode !== 200) {
          response.resume();
          reject(new Error(`Download failed: ${response.statusCode} ${response.statusMessage}`));
          return;
        }
        pipeline(response, createWriteStream(destination)).then(resolve, reject);
      }
    );
    request.on("error", reject);
  });
}

function run(command, args, options = {}) {
  const result = spawnSync(command, args, {
    stdio: "inherit",
    ...options
  });
  if (result.error) {
    throw result.error;
  }
  if (result.status !== 0) {
    throw new Error(`${command} ${args.join(" ")} failed with exit code ${result.status}`);
  }
}

async function main() {
  const triple = targetTriple();
  const vendor = path.join(__dirname, "..", "vendor");
  const binary = path.join(vendor, "blogx");

  if (checkOnly) {
    if (!existsSync(binary)) {
      throw new Error(`Missing installed binary at ${binary}`);
    }
    run(binary, ["--version"]);
    return;
  }

  mkdirSync(vendor, { recursive: true });
  rmSync(binary, { force: true });

  const archive = `blogx-${triple}.tar.gz`;
  const url = `https://github.com/${repo}/releases/download/v${version}/${archive}`;
  const tempDir = await mkdtemp(path.join(os.tmpdir(), "blogx-npm-"));
  const archivePath = path.join(tempDir, archive);

  await download(url, archivePath);
  run("tar", ["-xzf", archivePath, "-C", vendor, "blogx"]);
  chmodSync(binary, 0o755);
}

main().catch((error) => {
  console.error(`@faithleysath/blogx install failed: ${error.message}`);
  process.exit(1);
});
