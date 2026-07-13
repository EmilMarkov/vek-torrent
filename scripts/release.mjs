#!/usr/bin/env node
// Скрипт релиза: двигает версию во всех манифестах, обновляет lock-файлы,
// коммитит, тегает и пушит — пуш тега запускает workflow сборки релиза.
//
// Использование:
//   npm run release              # patch: 0.1.6 -> 0.1.7
//   npm run release -- minor     # 0.1.6 -> 0.2.0
//   npm run release -- major    # 0.1.6 -> 1.0.0
//   npm run release -- 1.2.3    # явная версия

import { execSync } from "node:child_process";
import { readFileSync, writeFileSync } from "node:fs";

const run = (cmd, options = {}) => {
  console.log(`$ ${cmd}`);
  return execSync(cmd, { stdio: "inherit", ...options });
};
const capture = (cmd) => execSync(cmd, { encoding: "utf8" }).trim();

const fail = (message) => {
  console.error(`✗ ${message}`);
  process.exit(1);
};

// ── Проверки состояния репозитория ─────────────────────────────────────────

const branch = capture("git rev-parse --abbrev-ref HEAD");
if (branch !== "main") fail(`релиз выполняется только с main (сейчас: ${branch})`);

if (capture("git status --porcelain") !== "") {
  fail("рабочее дерево не чистое — закоммитьте или отложите изменения");
}

run("git fetch origin main --tags");
const local = capture("git rev-parse HEAD");
const remote = capture("git rev-parse origin/main");
if (local !== remote) fail("main расходится с origin/main — сначала синхронизируйтесь");

// ── Вычисление новой версии ─────────────────────────────────────────────────

const pkg = JSON.parse(readFileSync("package.json", "utf8"));
const current = pkg.version;
const bump = process.argv[2] ?? "patch";

function nextVersion(version, kind) {
  const explicit = /^\d+\.\d+\.\d+$/;
  if (explicit.test(kind)) return kind;
  const [major, minor, patch] = version.split(".").map(Number);
  switch (kind) {
    case "major":
      return `${major + 1}.0.0`;
    case "minor":
      return `${major}.${minor + 1}.0`;
    case "patch":
      return `${major}.${minor}.${patch + 1}`;
    default:
      return fail(`непонятный аргумент «${kind}» (patch|minor|major|x.y.z)`);
  }
}

const version = nextVersion(current, bump);
const tag = `v${version}`;

if (capture(`git tag -l ${tag}`) !== "") fail(`тег ${tag} уже существует`);
console.log(`\nРелиз: ${current} → ${version}\n`);

// ── Обновление манифестов ───────────────────────────────────────────────────

// package.json
pkg.version = version;
writeFileSync("package.json", JSON.stringify(pkg, null, 2) + "\n");

// Cargo.toml (workspace.package.version)
const cargo = readFileSync("Cargo.toml", "utf8");
const cargoUpdated = cargo.replace(/^version = "\d+\.\d+\.\d+"$/m, `version = "${version}"`);
if (cargoUpdated === cargo) fail("не нашёл workspace-версию в Cargo.toml");
writeFileSync("Cargo.toml", cargoUpdated);

// src-tauri/tauri.conf.json
const confPath = "src-tauri/tauri.conf.json";
const conf = JSON.parse(readFileSync(confPath, "utf8"));
conf.version = version;
writeFileSync(confPath, JSON.stringify(conf, null, 2) + "\n");

// ── Lock-файлы ──────────────────────────────────────────────────────────────

run("npm install");
run("cargo generate-lockfile");

// ── Коммит, тег, пуш ────────────────────────────────────────────────────────

run("git add package.json package-lock.json Cargo.toml Cargo.lock src-tauri/tauri.conf.json");
run(`git commit -m "build: v${version}"`);
run(`git tag -a ${tag} -m "${tag}"`);
run("git push origin main");
run(`git push origin ${tag}`);

console.log(`\n✓ ${tag} отправлен — workflow Release собирает и публикует артефакты:`);
console.log("  https://github.com/EmilMarkov/vek-torrent/actions");
