/**
 * Devnet / mainnet deploy entrypoint.
 *
 * Run via `npm run deploy:devnet` (devnet) or `npm run deploy:mainnet`
 * (mainnet, requires `--force-mainnet` until the upgrade authority is a
 * Squads multisig).
 *
 * What it does:
 *   1. Reads the program-keypair pubkey at `target/deploy/enclz-keypair.json`
 *      and compares it to `declare_id!` in `programs/enclz/src/lib.rs`
 *      and `enclz = "..."` in `Anchor.toml`. If they drift, patches both
 *      files and rebuilds with `anchor build`.
 *   2. On mainnet, refuses to proceed unless `--force-mainnet` is passed.
 *   3. Best-effort idempotent check: dumps the deployed program binary via
 *      `solana program dump`, compares hashes with `target/deploy/enclz.so`.
 *      If identical, prints "no upgrade needed" and exits 0.
 *   4. Otherwise runs `anchor deploy --provider.cluster <cluster>`.
 */

import { execSync, spawnSync } from "node:child_process";
import { createHash } from "node:crypto";
import { existsSync, readFileSync, writeFileSync } from "node:fs";
import path from "node:path";

const REPO_ROOT = path.resolve(__dirname, "..");
const ANCHOR_TOML = path.join(REPO_ROOT, "Anchor.toml");
const LIB_RS = path.join(REPO_ROOT, "programs/enclz/src/lib.rs");
const PROGRAM_KEYPAIR = path.join(
  REPO_ROOT,
  "target/deploy/enclz-keypair.json"
);
const PROGRAM_SO = path.join(REPO_ROOT, "target/deploy/enclz.so");

type Cluster = "devnet" | "mainnet";

interface DeployArgs {
  cluster: Cluster;
  forceMainnet: boolean;
  skipIdempotenceCheck: boolean;
}

function parseArgs(): DeployArgs {
  const args = process.argv.slice(2);
  let cluster: Cluster = "devnet";
  let forceMainnet = false;
  let skipIdempotenceCheck = false;
  for (const arg of args) {
    if (arg === "--mainnet" || arg === "--cluster=mainnet") cluster = "mainnet";
    else if (arg === "--devnet" || arg === "--cluster=devnet")
      cluster = "devnet";
    else if (arg === "--force-mainnet") forceMainnet = true;
    else if (arg === "--skip-idempotence-check") skipIdempotenceCheck = true;
    else if (arg === "-h" || arg === "--help") {
      printHelpAndExit();
    } else {
      console.error(`unknown argument: ${arg}`);
      printHelpAndExit(1);
    }
  }
  return { cluster, forceMainnet, skipIdempotenceCheck };
}

function printHelpAndExit(code = 0): never {
  console.log(
    `Usage: ts-node migrations/deploy.ts [--devnet|--mainnet] [--force-mainnet] [--skip-idempotence-check]`
  );
  process.exit(code);
}

function rpcUrlFor(cluster: Cluster): string {
  if (cluster === "mainnet") {
    return process.env.MAINNET_RPC_URL ?? "https://api.mainnet-beta.solana.com";
  }
  return (
    process.env.QUICKNODE_DEVNET_RPC_URL ?? "https://api.devnet.solana.com"
  );
}

function readDeclaredId(): string {
  const lib = readFileSync(LIB_RS, "utf8");
  const match = lib.match(/declare_id!\("([^"]+)"\)/);
  if (!match) throw new Error(`failed to find declare_id! in ${LIB_RS}`);
  return match[1];
}

function patchDeclaredId(newId: string): void {
  const lib = readFileSync(LIB_RS, "utf8");
  const updated = lib.replace(
    /declare_id!\("[^"]+"\)/,
    `declare_id!("${newId}")`
  );
  writeFileSync(LIB_RS, updated);
  console.log(
    `patched declare_id!("${newId}") in ${path.relative(REPO_ROOT, LIB_RS)}`
  );
}

function patchAnchorToml(newId: string): void {
  const toml = readFileSync(ANCHOR_TOML, "utf8");
  const updated = toml.replace(/^(enclz\s*=\s*)"[^"]+"/gm, `$1"${newId}"`);
  writeFileSync(ANCHOR_TOML, updated);
  console.log(
    `patched [programs.*] enclz = "${newId}" in ${path.relative(
      REPO_ROOT,
      ANCHOR_TOML
    )}`
  );
}

function readKeypairPubkey(keypairPath: string): string {
  return execSync(`solana-keygen pubkey "${keypairPath}"`).toString().trim();
}

function hashFile(p: string): string {
  return createHash("sha256").update(readFileSync(p)).digest("hex");
}

function checkIdempotent(programId: string, cluster: Cluster): boolean {
  if (!existsSync(PROGRAM_SO)) return false;
  const url = rpcUrlFor(cluster);
  const dumpPath = path.join(REPO_ROOT, "target/deploy/enclz-deployed.so");
  const result = spawnSync(
    "solana",
    ["program", "dump", "-u", url, programId, dumpPath],
    { stdio: "pipe" }
  );
  if (result.status !== 0) return false;
  return hashFile(PROGRAM_SO) === hashFile(dumpPath);
}

function main(): void {
  const { cluster, forceMainnet, skipIdempotenceCheck } = parseArgs();

  if (!existsSync(PROGRAM_KEYPAIR)) {
    console.error(
      `missing program keypair at ${path.relative(
        REPO_ROOT,
        PROGRAM_KEYPAIR
      )} — run \`anchor build\` first`
    );
    process.exit(1);
  }

  const keypairPubkey = readKeypairPubkey(PROGRAM_KEYPAIR);
  const declaredId = readDeclaredId();

  if (declaredId !== keypairPubkey) {
    console.log(
      `program ID drift: declare_id!=${declaredId}, keypair=${keypairPubkey}`
    );
    patchDeclaredId(keypairPubkey);
    patchAnchorToml(keypairPubkey);
    console.log("rebuilding with corrected program ID...");
    execSync("anchor build", { stdio: "inherit", cwd: REPO_ROOT });
  }

  if (cluster === "mainnet" && !forceMainnet) {
    const wallet =
      process.env.ANCHOR_WALLET ?? "./.solana/keys/mainnet-deployer.json";
    console.error(
      `refusing mainnet deploy with single-sig wallet ${wallet}.\n` +
        `Pass --force-mainnet only after the upgrade authority is a Squads multisig.`
    );
    process.exit(2);
  }

  console.log(`Cluster:    ${cluster}`);
  console.log(`RPC URL:    ${rpcUrlFor(cluster)}`);
  console.log(`Program ID: ${keypairPubkey}`);

  if (!skipIdempotenceCheck && checkIdempotent(keypairPubkey, cluster)) {
    console.log(
      `no upgrade needed — deployed binary matches local target/deploy/enclz.so`
    );
    process.exit(0);
  }

  const cmd = `anchor deploy --provider.cluster ${cluster}`;
  console.log(`> ${cmd}`);
  execSync(cmd, { stdio: "inherit", cwd: REPO_ROOT });
  console.log(`deployed to ${cluster}: ${keypairPubkey}`);
}

main();
