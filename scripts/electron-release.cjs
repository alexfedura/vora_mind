const path = require("node:path");
const fs = require("node:fs");
const crypto = require("node:crypto");

const root = path.resolve(__dirname, "..");
const outDir = path.resolve(process.env.ELECTRON_OUTPUT_DIR || path.join(root, "artifacts/electron"));

function requireEnv(name) {
  const value = process.env[name]?.trim();
  if (!value) throw new Error(`Missing required environment variable: ${name}`);
  return value;
}

function validateHttpsUrl(value) {
  let url;
  try {
    url = new URL(value);
  } catch {
    throw new Error("VORA_UPDATE_URL is not a valid URL");
  }
  if (url.protocol !== "https:" || url.username || url.password) {
    throw new Error("VORA_UPDATE_URL must be HTTPS and must not contain credentials");
  }
  return url.toString().replace(/\/+$/, "");
}

async function main() {
  const signingRequired = process.env.ELECTRON_REQUIRE_SIGNING === "1";
  const updateUrl = validateHttpsUrl(requireEnv("VORA_UPDATE_URL"));
  let azureSignOptions;

  if (signingRequired) {
    azureSignOptions = {
      endpoint: requireEnv("AZURE_TRUSTED_SIGNING_ENDPOINT"),
      codeSigningAccountName: requireEnv("AZURE_TRUSTED_SIGNING_ACCOUNT"),
      certificateProfileName: requireEnv("AZURE_TRUSTED_SIGNING_PROFILE"),
      publisherName: requireEnv("AZURE_TRUSTED_SIGNING_PUBLISHER"),
      fileDigest: "SHA256",
      timestampDigest: "SHA256",
      timestampRfc3161: "http://timestamp.acs.microsoft.com"
    };
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
        provider: "generic",
        url: updateUrl,
        channel: "latest"
      },
      win: {
        target: ["nsis"],
        artifactName: "${productName}-${version}-${arch}-setup.${ext}",
        forceCodeSigning: signingRequired,
        ...(azureSignOptions ? { azureSignOptions } : {})
      },
      nsis: {
        oneClick: false,
        perMachine: false,
        allowToChangeInstallationDirectory: true,
        createDesktopShortcut: true,
        createStartMenuShortcut: true
      }
    }
  });

  validateLatestMetadata(outDir);
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
