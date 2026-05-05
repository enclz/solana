#!/usr/bin/env node
import { execSync } from "node:child_process";
import {
  existsSync,
  readdirSync,
  readFileSync,
  statSync,
  writeFileSync,
  copyFileSync,
  mkdirSync,
} from "node:fs";
import { join, dirname } from "node:path";
import { fileURLToPath } from "node:url";

const root = join(dirname(fileURLToPath(import.meta.url)), "..");

const idlSrc = join(root, "target/idl/enclz.json");
const typesSrc = join(root, "target/types/enclz.ts");
const programSrcDir = join(root, "programs/enclz/src");
const programManifest = join(root, "programs/enclz/Cargo.toml");
const sdkSrcDir = join(root, "sdk/src");
const sdkDir = join(root, "sdk");

// Step 1: Run anchor build when artifacts are missing OR stale.
// Staleness is "any program source is newer than target/idl/enclz.json" —
// existsSync alone shipped a stale IDL once already (execute_swap +
// execute_lending_op were absent for two commits because the cached file
// passed the existence check).
function newestMtime(path) {
  const stat = statSync(path);
  if (!stat.isDirectory()) return stat.mtimeMs;
  let max = stat.mtimeMs;
  for (const entry of readdirSync(path, { withFileTypes: true })) {
    const child = join(path, entry.name);
    const childMax = newestMtime(child);
    if (childMax > max) max = childMax;
  }
  return max;
}

let needsBuild = !existsSync(idlSrc) || !existsSync(typesSrc);
let staleReason = null;
if (!needsBuild) {
  const idlMtime = statSync(idlSrc).mtimeMs;
  const srcMtime = newestMtime(programSrcDir);
  const manifestMtime = statSync(programManifest).mtimeMs;
  const newestSource = Math.max(srcMtime, manifestMtime);
  if (newestSource > idlMtime) {
    needsBuild = true;
    staleReason = "program source newer than target/idl/enclz.json";
  }
}

if (needsBuild) {
  console.log(
    staleReason
      ? `Artifacts stale (${staleReason}) — running anchor build...`
      : "Artifacts missing — running anchor build...",
  );
  execSync("anchor build", { cwd: root, stdio: "inherit" });
}

// Step 2: Copy artifacts into sdk/src/
mkdirSync(sdkSrcDir, { recursive: true });
copyFileSync(idlSrc, join(sdkSrcDir, "enclz.json"));
copyFileSync(typesSrc, join(sdkSrcDir, "enclz.ts"));

// Step 3: Sync sdk/package.json version from IDL metadata.version
const idl = JSON.parse(readFileSync(join(sdkSrcDir, "enclz.json"), "utf8"));
const idlVersion = idl.metadata?.version;
if (!idlVersion) {
  throw new Error("IDL is missing metadata.version — rebuild the program");
}
const sdkPkgPath = join(sdkDir, "package.json");
const sdkPkg = JSON.parse(readFileSync(sdkPkgPath, "utf8"));
if (sdkPkg.version !== idlVersion) {
  sdkPkg.version = idlVersion;
  writeFileSync(sdkPkgPath, JSON.stringify(sdkPkg, null, 2) + "\n");
  console.log(`Updated sdk/package.json version → ${idlVersion}`);
}

// Step 4: Compile sdk/src/ → sdk/dist/
const tsc = join(root, "node_modules/.bin/tsc");
execSync(`${tsc} -p tsconfig.json`, { cwd: sdkDir, stdio: "inherit" });

console.log("sdk/dist/ built successfully.");
