#!/usr/bin/env node

const { spawnSync } = require("node:child_process");
const { existsSync } = require("node:fs");
const path = require("node:path");

const executable = process.platform === "win32" ? "blogx.exe" : "blogx";
const binary = path.join(__dirname, "..", "vendor", executable);

if (!existsSync(binary)) {
  console.error("BlogX binary is missing. Reinstall @faithleysath/blogx.");
  process.exit(1);
}

const result = spawnSync(binary, process.argv.slice(2), {
  stdio: "inherit"
});

if (result.error) {
  console.error(result.error.message);
  process.exit(1);
}

process.exit(result.status === null ? 1 : result.status);
