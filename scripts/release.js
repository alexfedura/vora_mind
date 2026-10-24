import { execFileSync } from "node:child_process";
import { existsSync, readFileSync, writeFileSync } from "node:fs";
import { resolve } from "node:path";
import { fileURLToPath } from "node:url";

const root = resolve(fileURLToPath(new URL("..", import.meta.url)));
const command = process.argv[2];
const argument = process.argv[3];
const files = {
  package: resolve(root, "package.json"),
  tauri: resolve(root, "src-tauri/tauri.conf.json"),
  cargo: resolve(root, "src-tauri/Cargo.toml"),
  lock: resolve(root, "src-tauri/Cargo.lock"),
};

function fail(message) {
  console.error(message);
  process.exit(1);
}

function runGit(args, options = {}) {
  const output = execFileSync("git", args, {
    cwd: root,
    encoding: "utf8",
    stdio: options.silent ? ["ignore", "pipe", "inherit"] : "inherit",
  });
  return (output ?? "").trim();
}

function readVersions() {
  const packageJson = JSON.parse(readFileSync(files.package, "utf8"));
  const tauriJson = JSON.parse(readFileSync(files.tauri, "utf8"));
  const cargoToml = readFileSync(files.cargo, "utf8");
  const cargoVersion = /^\[package\][\s\S]*?^version = "([^"]+)"/m.exec(cargoToml)?.[1];
  if (!cargoVersion) fail("Could not read package.version from src-tauri/Cargo.toml");
  return {
    package: packageJson.version,
    tauri: tauriJson.version,
    cargo: cargoVersion,
  };
}

function assertVersionsMatch(versions) {
  if (new Set(Object.values(versions)).size !== 1) {
    fail(`Version mismatch: package.json=${versions.package}, tauri.conf.json=${versions.tauri}, Cargo.toml=${versions.cargo}`);
  }
  if (!/^(0|[1-9]\d*)\.(0|[1-9]\d*)\.(0|[1-9]\d*)$/.test(versions.package)) {
    fail(`Expected a stable semantic version, got ${versions.package}`);
  }
}

function increment(version, releaseType) {
  const parts = version.split(".").map(Number);
  if (releaseType === "major") return `${parts[0] + 1}.0.0`;
  if (releaseType === "minor") return `${parts[0]}.${parts[1] + 1}.0`;
  return `${parts[0]}.${parts[1]}.${parts[2] + 1}`;
}

function setCargoLockVersion(contents, oldVersion, newVersion) {
  const escapedVersion = oldVersion.replaceAll(".", "\\.");
  const pattern = new RegExp(`(^\\[\\[package\\]\\]\\r?\\nname = "vora_mind"\\r?\\nversion = ")${escapedVersion}(")`, "m");
  if (!pattern.test(contents)) fail("Could not find the vora_mind package version in src-tauri/Cargo.lock");
  return contents.replace(pattern, `$1${newVersion}$2`);
}

function syncCargoLock(oldVersion, newVersion) {
  if (!existsSync(files.lock)) {
    console.warn("Cargo.lock not found, skipping version sync for it.");
    return;
  }
  const cargoLock = readFileSync(files.lock, "utf8");
  writeFileSync(files.lock, setCargoLockVersion(cargoLock, oldVersion, newVersion));
}

function isTracked(file) {
  return runGit(["ls-files", "--", file], { silent: true }).length > 0;
}

if (command === "sync") {
  const versions = readVersions();
  if (!/^(0|[1-9]\d*)\.(0|[1-9]\d*)\.(0|[1-9]\d*)$/.test(versions.package)) {
    fail(`Expected a stable semantic version, got ${versions.package}`);
  }

  const tauriJson = JSON.parse(readFileSync(files.tauri, "utf8"));
  tauriJson.version = versions.package;
  writeFileSync(files.tauri, `${JSON.stringify(tauriJson, null, 2)}\n`);

  const cargoToml = readFileSync(files.cargo, "utf8");
  const cargoPattern = /(^\[package\][\s\S]*?^version = ")[^"]+(")/m;
  if (!cargoPattern.test(cargoToml)) fail("Could not update package.version in src-tauri/Cargo.toml");
  writeFileSync(files.cargo, cargoToml.replace(cargoPattern, `$1${versions.package}$2`));

  syncCargoLock(versions.cargo, versions.package);
  assertVersionsMatch(readVersions());
  console.log(`Synchronized release version: ${versions.package}`);
  process.exit(0);
}

if (command === "verify") {
  const versions = readVersions();
  assertVersionsMatch(versions);
  const expectedTag = `v${versions.package}`;
  const tag = process.argv.slice(3).find((value) => value !== "--");
  if (tag && tag !== expectedTag) fail(`Release tag ${tag} does not match synchronized app version ${expectedTag}`);
  console.log(`Release version verified: ${expectedTag}`);
  process.exit(0);
}

if (!["patch", "minor", "major"].includes(command)) {
  fail("Usage: node scripts/release.js <patch|minor|major> [--dry-run]");
}

const dryRun = argument === "--dry-run";
const versions = readVersions();
assertVersionsMatch(versions);
const nextVersion = increment(versions.package, command);
const tag = `v${nextVersion}`;
console.log(`Release ${versions.package} -> ${nextVersion} (${tag})`);
if (dryRun) process.exit(0);

if (runGit(["status", "--porcelain"], { silent: true })) {
  fail("Release requires a clean working tree. Commit or stash all changes first.");
}
const branch = runGit(["branch", "--show-current"], { silent: true });
if (!branch) fail("Release must run from a named branch, not detached HEAD.");
if (runGit(["tag", "--list", tag], { silent: true })) fail(`Tag ${tag} already exists.`);
runGit(["remote", "get-url", "origin"], { silent: true });

const packageJson = JSON.parse(readFileSync(files.package, "utf8"));
packageJson.version = nextVersion;
writeFileSync(files.package, `${JSON.stringify(packageJson, null, 2)}\n`);

const tauriJson = JSON.parse(readFileSync(files.tauri, "utf8"));
tauriJson.version = nextVersion;
writeFileSync(files.tauri, `${JSON.stringify(tauriJson, null, 2)}\n`);

const cargoToml = readFileSync(files.cargo, "utf8");
const cargoPattern = /(^\[package\][\s\S]*?^version = ")[^"]+(")/m;
if (!cargoPattern.test(cargoToml)) fail("Could not update package.version in src-tauri/Cargo.toml");
writeFileSync(files.cargo, cargoToml.replace(cargoPattern, `$1${nextVersion}$2`));

syncCargoLock(versions.cargo, nextVersion);

const releaseFiles = ["package.json", "src-tauri/tauri.conf.json", "src-tauri/Cargo.toml"];
if (existsSync(files.lock) && isTracked("src-tauri/Cargo.lock")) releaseFiles.push("src-tauri/Cargo.lock");
runGit(["add", ...releaseFiles]);
runGit([
  "commit",
  "-m",
  `chore(config): release ${tag}`,
  "-m",
  "Co-authored-by: Copilot <223556219+Copilot@users.noreply.github.com>",
]);
runGit(["tag", "-a", tag, "-m", `Release ${tag}`]);
runGit(["push", "origin", `HEAD:refs/heads/${branch}`, `refs/tags/${tag}`]);
console.log(`Pushed ${tag}; GitHub Actions will build and publish the release.`);
