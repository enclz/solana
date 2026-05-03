#!/usr/bin/env node
/**
 * Mirrors `target/idl/enclz.json` → `idl/enclz.json` and regenerates
 * `idl/error-map.json` from the IDL's `errors` array.
 *
 * Run after `anchor build` (or as the last step of `migrations/deploy.ts`)
 * to keep the committed artifacts current.
 */

import { mkdirSync, readFileSync, writeFileSync } from "node:fs";
import { dirname, resolve } from "node:path";

const REPO_ROOT = resolve(new URL(".", import.meta.url).pathname, "..");
const IDL_TARGET = resolve(REPO_ROOT, "target/idl/enclz.json");
const IDL_COMMITTED = resolve(REPO_ROOT, "idl/enclz.json");
const ERROR_MAP_COMMITTED = resolve(REPO_ROOT, "idl/error-map.json");

function camelCase(name) {
  if (!name) return name;
  return name[0].toLowerCase() + name.slice(1);
}

function buildErrorMap(idl) {
  const errors = Array.isArray(idl?.errors) ? idl.errors : [];
  return errors.map((entry) => ({
    anchorCode: entry.code,
    name: entry.name,
    restErrorCode: camelCase(entry.name),
  }));
}

function main() {
  const idl = JSON.parse(readFileSync(IDL_TARGET, "utf8"));
  mkdirSync(dirname(IDL_COMMITTED), { recursive: true });
  writeFileSync(IDL_COMMITTED, JSON.stringify(idl, null, 2) + "\n");
  writeFileSync(
    ERROR_MAP_COMMITTED,
    JSON.stringify(buildErrorMap(idl), null, 2) + "\n"
  );
  console.log(`wrote ${IDL_COMMITTED}`);
  console.log(`wrote ${ERROR_MAP_COMMITTED}`);
}

main();
