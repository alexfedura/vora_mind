const path = require("node:path");
const fs = require("node:fs");
const crypto = require("node:crypto");
const { spawnSync } = require("node:child_process");

const root = path.resolve(__dirname, "..");
const outDir = path.resolve(process.env.ELECTRON_OUTPUT_DIR || path.join(root, "artifacts/electron"));
const publish = process.env.ELECTRON_PUBLISH === "1";

function requireEnv(name) {
  const value = process.env[name]?.trim();
  if (!value) throw new Error(`Missing required environment variable: ${name}`);
  return value;
}

async function main() {
  const signingRequired = process.env.ELECTRON_REQUIRE_SIGNING === "1";
  if (signingRequired) {
    const certificate = process.env.CSC_LINK || process.env.WIN_CSC_LINK;
    if (!certificate) throw new Error("Missing Windows signing certificate: CSC_LINK or WIN_CSC_LINK");
    if (!process.env.CSC_KEY_PASSWORD && !process.env.WIN_CSC_KEY_PASSWORD) {
      throw new Error("Missing Windows signing certificate password");
    }
  }

  const build = spawnSync("pnpm", ["electron:build"], {
    cwd: root,
    stdio: "inherit",
    shell: true
  });
  if (build.status !== 0) {
    throw new Error(`Electron build failed with exit code ${build.status ?? "unknown"}`);
  }

  const electronBuilder = require("electron-builder");
  await electronBuilder.build({
    targets: electronBuilder.Platform.WINDOWS.createTarget(["nsis"], electronBuilder.Arch.x64),
    config: {
      appId: "com.alexfedura.voramind",
      productName: "Vora Mind",
      directories: { output: outDir },
      files: ["dist/**", "dist-electron/**", "package.json"],
      asar: true,
      publish: {
        provider: "github",
        owner: "alexfedura",
        repo: "vora_mind",
        channel: "latest",
        private: false
      },
      win: {
        target: ["nsis"],
        artifactName: "Vora-Mind-${version}-${arch}-setup.${ext}",
        forceCodeSigning: signingRequired
      },
      nsis: {
        oneClick: false,
        perMachine: false,
        allowToChangeInstallationDirectory: true,
        allowElevation: false,
        deleteAppDataOnUninstall: false,
        createDesktopShortcut: true,
        createStartMenuShortcut: true
      }
    },
    publish: publish ? "always" : "never"
  });

  validateLatestMetadata(outDir);
  if (publish) requireEnv("GITHUB_TOKEN");
}

function parseYamlScalar(text, key) {
  const match = text.match(new RegExp(`^${key}:\\s*(.+)$`, "m"));
  return match ? match[1].trim() : null;
}

function validateLatestMetadata(outputDirectory) {
  const latestPath = path.join(outputDirectory, "latest.yml");
  if (!fs.existsSync(latestPath)) {
    throw new Error(`latest.yml was not generated at ${latestPath}`);
  }

  const text = fs.readFileSync(latestPath, "utf8");
  const version = parseYamlScalar(text, "version");
  if (!version) throw new Error("latest.yml has no version");

  const lines = text.split(/\r?\n/);
  const entries = [];
  for (let index = 0; index < lines.length; index += 1) {
    const urlMatch = lines[index].match(/^\s*-\s+url:\s*(.+)\s*$/);
    if (!urlMatch) continue;
    const url = urlMatch[1].trim();
    let sha512 = null;
    for (let child = index + 1; child < Math.min(index + 8, lines.length); child += 1) {
      const shaMatch = lines[child].match(/^\s+sha512:\s*(.+)\s*$/);
      if (shaMatch) {
        sha512 = shaMatch[1].trim();
        break;
      }
    }
    entries.push({ url, sha512 });
  }

  if (entries.length === 0) throw new Error("latest.yml has no installer file entry");
  for (const entry of entries) {
    const filePath = path.join(outputDirectory, entry.url);
    if (!fs.existsSync(filePath)) continue;
    if (!entry.sha512) throw new Error(`latest.yml has no sha512 entry for ${entry.url}`);
    const actual = crypto.createHash("sha512").update(fs.readFileSync(filePath)).digest("base64");
    if (actual !== entry.sha512) throw new Error(`SHA-512 mismatch for ${entry.url}`);
  }

  fs.writeFileSync(path.join(outputDirectory, "latest-validation.json"), JSON.stringify({
    validatedAt: new Date().toISOString(),
    latestYml: latestPath,
    version
  }, null, 2));
}

main().catch((error) => {
  console.error(error instanceof Error ? error.message : String(error));
  process.exitCode = 1;
});
