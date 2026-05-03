#!/usr/bin/env node
/**
 * Parses cargo-tarpaulin's cobertura XML output and enforces thresholds:
 *   - Overall instruction-code coverage SHALL be ≥ 85%.
 *   - Coverage of `programs/enclz/src/instructions/execute_transfer.rs`
 *     SHALL be ≥ 90% (it's the security boundary).
 *
 * Exits 0 if both pass, 1 otherwise.
 */

import { readFileSync } from "node:fs";
import { resolve } from "node:path";

const OVERALL_THRESHOLD = 0.85;
const EXECUTE_TRANSFER_THRESHOLD = 0.9;
const INSTRUCTIONS_DIR = "programs/enclz/src/instructions/";
const EXECUTE_TRANSFER_FILE = `${INSTRUCTIONS_DIR}execute_transfer.rs`;

const xmlPath = resolve(process.argv[2] ?? "target/tarpaulin/cobertura.xml");
const xml = readFileSync(xmlPath, "utf8");

function parseClasses() {
  const classes = [];
  const re = /<class[^>]*filename="([^"]+)"[^>]*line-rate="([^"]+)"/g;
  let match;
  while ((match = re.exec(xml)) !== null) {
    classes.push({ filename: match[1], lineRate: Number(match[2]) });
  }
  return classes;
}

function aggregate(classes, predicate) {
  const linesRe = /<lines>[\s\S]*?<\/lines>/;
  // Walk class blocks and tally lines covered / total only for classes that
  // pass the predicate.
  const blockRe = /<class[^>]*filename="([^"]+)"[\s\S]*?<\/class>/g;
  let covered = 0;
  let total = 0;
  let block;
  while ((block = blockRe.exec(xml)) !== null) {
    const filename = block[1];
    if (!predicate(filename)) continue;
    const linesMatch = block[0].match(linesRe);
    if (!linesMatch) continue;
    const lineRe = /<line[^>]*hits="(\d+)"/g;
    let lineMatch;
    while ((lineMatch = lineRe.exec(linesMatch[0])) !== null) {
      total += 1;
      if (Number(lineMatch[1]) > 0) covered += 1;
    }
  }
  return { covered, total };
}

const classes = parseClasses();
if (classes.length === 0) {
  console.error(`could not parse any <class> entries from ${xmlPath}`);
  process.exit(1);
}

const overall = aggregate(classes, (f) => f.startsWith(INSTRUCTIONS_DIR));
const executeTransfer = aggregate(
  classes,
  (f) => f === EXECUTE_TRANSFER_FILE
);

if (overall.total === 0) {
  console.error(
    `no covered lines found under ${INSTRUCTIONS_DIR} — verify tarpaulin ran against the right tree`
  );
  process.exit(1);
}

const overallRate = overall.covered / overall.total;
const executeRate =
  executeTransfer.total === 0
    ? 0
    : executeTransfer.covered / executeTransfer.total;

console.log(
  `instructions/ : ${overall.covered}/${overall.total} = ${(overallRate * 100).toFixed(2)}% (gate ${OVERALL_THRESHOLD * 100}%)`
);
console.log(
  `execute_transfer.rs : ${executeTransfer.covered}/${executeTransfer.total} = ${(executeRate * 100).toFixed(2)}% (gate ${EXECUTE_TRANSFER_THRESHOLD * 100}%)`
);

let failed = false;
if (overallRate < OVERALL_THRESHOLD) {
  console.error(
    `FAIL: instructions/ coverage ${(overallRate * 100).toFixed(2)}% below ${OVERALL_THRESHOLD * 100}%`
  );
  failed = true;
}
if (executeTransfer.total > 0 && executeRate < EXECUTE_TRANSFER_THRESHOLD) {
  console.error(
    `FAIL: execute_transfer.rs coverage ${(executeRate * 100).toFixed(2)}% below ${EXECUTE_TRANSFER_THRESHOLD * 100}%`
  );
  failed = true;
}
if (executeTransfer.total === 0) {
  console.error(
    `FAIL: execute_transfer.rs not present in coverage report — tarpaulin may have skipped the file`
  );
  failed = true;
}

process.exit(failed ? 1 : 0);
