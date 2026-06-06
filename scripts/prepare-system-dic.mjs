import { createHash } from "node:crypto";
import { createReadStream, createWriteStream, existsSync, readFileSync, renameSync, statSync } from "node:fs";
import { mkdir } from "node:fs/promises";
import { dirname, resolve } from "node:path";
import { pipeline } from "node:stream/promises";
import { fileURLToPath } from "node:url";

const __filename = fileURLToPath(import.meta.url);
const __dirname = dirname(__filename);
const repoRoot = resolve(__dirname, "..");
const partsDir = resolve(repoRoot, "resources/system.dic.parts");
const manifestPath = resolve(partsDir, "manifest.json");
const manifest = JSON.parse(readFileSync(manifestPath, "utf8"));
const outputPath = resolve(repoRoot, manifest.file);
const tempPath = `${outputPath}.tmp`;

function sha256(path) {
  return new Promise((resolveHash, reject) => {
    const hash = createHash("sha256");
    const stream = createReadStream(path);
    stream.on("data", (chunk) => hash.update(chunk));
    stream.on("error", reject);
    stream.on("end", () => resolveHash(hash.digest("hex")));
  });
}

async function outputIsCurrent() {
  if (!existsSync(outputPath)) {
    return false;
  }

  const stats = statSync(outputPath);
  if (stats.size !== manifest.size) {
    return false;
  }

  return (await sha256(outputPath)) === manifest.sha256;
}

async function appendPart(partName, output) {
  const partPath = resolve(partsDir, partName);
  if (!existsSync(partPath)) {
    throw new Error(`Missing dictionary part: ${partPath}`);
  }

  await pipeline(createReadStream(partPath), output, { end: false });
}

async function assemble() {
  await mkdir(dirname(outputPath), { recursive: true });

  const output = createWriteStream(tempPath);
  try {
    for (const partName of manifest.parts) {
      await appendPart(partName, output);
    }
  } finally {
    output.end();
  }

  await new Promise((resolveFinished, reject) => {
    output.on("finish", resolveFinished);
    output.on("error", reject);
  });

  const stats = statSync(tempPath);
  if (stats.size !== manifest.size) {
    throw new Error(`Assembled dictionary has size ${stats.size}; expected ${manifest.size}`);
  }

  const actualHash = await sha256(tempPath);
  if (actualHash !== manifest.sha256) {
    throw new Error(`Assembled dictionary sha256 ${actualHash}; expected ${manifest.sha256}`);
  }

  renameSync(tempPath, outputPath);
}

if (await outputIsCurrent()) {
  console.log("resources/system.dic is current");
} else {
  console.log("assembling resources/system.dic from parts");
  await assemble();
  console.log("resources/system.dic is ready");
}
