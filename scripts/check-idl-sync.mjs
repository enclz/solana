#!/usr/bin/env node
/**
 * CI gate: fails if `target/idl/enclz.json` (just produced by `anchor build`)
 * has drifted from the committed `idl/enclz.json`, or if `idl/error-map.json`
 * is out of sync with the IDL's errors array.
 *
 * Used by `.github/workflows/program-ci.yml` after the build job artifact is
 * downloaded to `target/`.
 */

import { existsSync, readFileSync } from "node:fs";
import { resolve } from "node:path";

const REPO_ROOT = resolve(new URL(".", import.meta.url).pathname, "..");
const IDL_TARGET = resolve(REPO_ROOT, "target/idl/enclz.json");
const IDL_COMMITTED = resolve(REPO_ROOT, "idl/enclz.json");
const ERROR_MAP_COMMITTED = resolve(REPO_ROOT, "idl/error-map.json");

function camelCase(name) {
  if (!name) return name;
  return name[0].toLowerCase() + name.slice(1);
}

function expectedErrorMap(idl) {
  const errors = Array.isArray(idl?.errors) ? idl.errors : [];
  return errors.map((entry) => ({
    anchorCode: entry.code,
    name: entry.name,
    restErrorCode: camelCase(entry.name),
  }));
}

function readJson(p) {
  return JSON.parse(readFileSync(p, "utf8"));
}

function main() {
  if (!existsSync(IDL_TARGET)) {
    console.error(
      `missing ${IDL_TARGET} — run \`anchor build\` before checking IDL drift`
    );
    process.exit(1);
  }
  const builtIdl = readJson(IDL_TARGET);

  // The committed IDL is populated after the first successful devnet deploy
  // (see openspec/changes/add-devnet-deploy-pipeline tasks 6.1/6.2). Until
  // then, the IDL drift gate is dormant and only the error-map gate runs.
  if (existsSync(IDL_COMMITTED)) {
    const committedIdl = readJson(IDL_COMMITTED);
    if (JSON.stringify(builtIdl) !== JSON.stringify(committedIdl)) {
      console.error(
        `FAIL: target/idl/enclz.json differs from idl/enclz.json — run \`npm run idl:sync\` and commit the result`
      );
      process.exit(1);
    }
    console.log("✓ idl/enclz.json in sync with target");
  } else {
    console.log(
      `idl/enclz.json not yet committed — gate dormant until first deploy seeds it`
    );
  }

  if (!existsSync(ERROR_MAP_COMMITTED)) {
    console.error(
      `missing committed ${ERROR_MAP_COMMITTED} — run \`npm run idl:sync\` and commit`
    );
    process.exit(1);
  }

  const expectedMap = expectedErrorMap(builtIdl);
  const committedMap = readJson(ERROR_MAP_COMMITTED);
  if (JSON.stringify(expectedMap) !== JSON.stringify(committedMap)) {
    console.error(
      `FAIL: idl/error-map.json drift — run \`npm run idl:sync\` and commit the result`
    );
    process.exit(1);
  }

  console.log("✓ idl/error-map.json in sync with target");
}

main();
