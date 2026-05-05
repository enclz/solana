#!/usr/bin/env node
/**
 * Asserts that every `pub fn` declared inside the `#[program]` module of
 * `programs/enclz/src/lib.rs` is present in `target/idl/enclz.json`.
 *
 * Anchor 1.x silently drops handlers from the IDL when its macro can't parse
 * a signature (we hit this with `<'info>`-generic handlers staying in the
 * source IDL but missing from the published artefact). This gate fails fast
 * so a stale or incomplete IDL never ships.
 *
 * Exits 0 on parity, 1 otherwise.
 */

import { readFileSync } from "node:fs";
import { resolve } from "node:path";

const root = resolve(new URL("..", import.meta.url).pathname);
const libRsPath = resolve(root, "programs/enclz/src/lib.rs");
const idlPath = resolve(root, process.argv[2] ?? "target/idl/enclz.json");

const libRs = readFileSync(libRsPath, "utf8");

// Slice out the `#[program] pub mod ... { ... }` block by counting braces from
// the opening `{` after `pub mod`. A regex would mis-match nested braces.
const programModRe = /#\[program\]\s*pub mod\s+\w+\s*\{/;
const modMatch = libRs.match(programModRe);
if (!modMatch) {
  console.error(`FAIL: could not locate #[program] module in ${libRsPath}`);
  process.exit(1);
}
const start = modMatch.index + modMatch[0].length;
let depth = 1;
let cursor = start;
while (cursor < libRs.length && depth > 0) {
  const ch = libRs[cursor];
  if (ch === "{") depth += 1;
  else if (ch === "}") depth -= 1;
  cursor += 1;
}
if (depth !== 0) {
  console.error(`FAIL: unbalanced braces in #[program] module in ${libRsPath}`);
  process.exit(1);
}
const programBody = libRs.slice(start, cursor - 1);

// `pub fn name<...>(...)` or `pub fn name(...)`.
const handlerRe = /pub\s+fn\s+([a-z_][a-z0-9_]*)\s*[<(]/g;
const handlers = new Set();
let match;
while ((match = handlerRe.exec(programBody)) !== null) {
  handlers.add(match[1]);
}

if (handlers.size === 0) {
  console.error(
    `FAIL: parsed zero handlers from #[program] module in ${libRsPath}`
  );
  process.exit(1);
}

const idl = JSON.parse(readFileSync(idlPath, "utf8"));
if (!Array.isArray(idl.instructions)) {
  console.error(`FAIL: ${idlPath} has no .instructions[] array`);
  process.exit(1);
}
const idlInstructions = new Set(idl.instructions.map((ix) => ix.name));

const missingFromIdl = [...handlers].filter((h) => !idlInstructions.has(h));
const extraInIdl = [...idlInstructions].filter((i) => !handlers.has(i));

console.log(
  `lib.rs handlers: ${handlers.size} (${[...handlers].sort().join(", ")})`
);
console.log(
  `IDL instructions: ${idlInstructions.size} (${[...idlInstructions].sort().join(", ")})`
);

let failed = false;
if (missingFromIdl.length > 0) {
  console.error(
    `FAIL: handlers absent from IDL: ${missingFromIdl.sort().join(", ")}`
  );
  failed = true;
}
if (extraInIdl.length > 0) {
  console.error(
    `FAIL: IDL instructions absent from lib.rs: ${extraInIdl.sort().join(", ")}`
  );
  failed = true;
}

process.exit(failed ? 1 : 0);
