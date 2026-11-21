const fs = require("node:fs");
const path = require("node:path");
const crypto = require("node:crypto");

const outputDir = path.resolve(process.argv[2] || "artifacts/electron");
const latestPath = path.join(outputDir, "latest.yml");

if (!fs.existsSync(latestPath)) {
  console.error(`Missing ${latestPath}`);
  process.exit(1);
}

const text = fs.readFileSync(latestPath, "utf8");
const exeNames = fs.readdirSync(outputDir).filter((name) => name.endsWith(".exe") && name.includes("-setup."));
if (exeNames.length !== 1) {
  console.error(`Expected exactly one NSIS setup exe, found ${exeNames.length}`);
  process.exit(1);
}

const exeName = exeNames[0];
const exePath = path.join(outputDir, exeName);
const lines = text.split(/\r?\n/);
let sha512 = null;
for (let index = 0; index < lines.length; index += 1) {
  const urlMatch = lines[index].match(/^\s*-\s+url:\s*(.+)\s*$/);
  if (!urlMatch || urlMatch[1].trim() !== exeName) continue;
  for (let child = index + 1; child < Math.min(index + 8, lines.length); child += 1) {
    const shaMatch = lines[child].match(/^\s+sha512:\s*(.+)\s*$/);
    if (shaMatch) {
      sha512 = shaMatch[1].trim();
      break;
    }
  }
  break;
}
if (!sha512) {
  console.error(`latest.yml has no sha512 entry for ${exeName}`);
  process.exit(1);
}

const actual = crypto.createHash("sha512").update(fs.readFileSync(exePath)).digest("base64");
if (actual !== sha512) {
  console.error(`SHA-512 mismatch for ${exeName}`);
  process.exit(1);
}

console.log(JSON.stringify({
  installer: exePath,
  sha512: actual,
  latestYml: latestPath
}, null, 2));
