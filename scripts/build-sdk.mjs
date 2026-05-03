#!/usr/bin/env node
import { execSync } from "node:child_process";
import {
  existsSync,
  readFileSync,
  writeFileSync,
  copyFileSync,
  mkdirSync,
} from "node:fs";
import { join, dirname } from "node:path";
import { fileURLToPath } from "node:url";

const root = join(dirname(fileURLToPath(import.meta.url)), "..");

const idlSrc = join(root, "target/idl/enclz.json");
const typesSrc = join(root, "target/types/enclz.ts");
const sdkSrcDir = join(root, "sdk/src");
const sdkDir = join(root, "sdk");

// Step 1: Run anchor build if artifacts are missing
if (!existsSync(idlSrc) || !existsSync(typesSrc)) {
  console.log("Artifacts missing — running anchor build...");
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
