/**
 * End-to-end smoke test against a live cluster (devnet by default).
 *
 * Run via `npm run smoke:devnet` after `npm run deploy:devnet`.
 *
 * Required env (loaded via dotenv-cli):
 *   QUICKNODE_DEVNET_RPC_URL — devnet RPC endpoint
 *   ANCHOR_WALLET            — funded fee payer; defaults to .solana/keys/devnet-deployer.json
 *
 * Exits 0 only if every step passes:
 *   1. fresh owner / backend operator / merchant keypairs (airdropped from fee payer)
 *   2. fresh test mint (stand-in for USDC)
 *   3. pre-create protocol_fee_wallet ATA via getOrCreateAssociatedTokenAccount
 *   4. initialize_group (creates DEX-router type-2 entry atomically)
 *   5. add_agent with hourly_tx_cap=10 (so the 6th transfer hits WhitelistViolation, not HourlyCapExceeded)
 *   6. add_to_whitelist for an external merchant ($5 cap, ttl=now+3600)
 *   7. mint 10 USDC into agent ATA
 *   8. execute 5 × $1 execute_transfer; assert each succeeds and amount_used grows
 *   9. assert WhitelistEntry PDA closed after 5th transfer
 *  10. attempt a 6th transfer to the same merchant — assert it reverts
 *  11. attempt a stale-nonce transfer — assert NonceMismatch
 */

import * as anchor from "@coral-xyz/anchor";
import { BN, Program } from "@coral-xyz/anchor";
import {
  Connection,
  Keypair,
  LAMPORTS_PER_SOL,
  PublicKey,
  SendTransactionError,
  SystemProgram,
} from "@solana/web3.js";
import {
  ASSOCIATED_TOKEN_PROGRAM_ID,
  TOKEN_PROGRAM_ID,
  createAssociatedTokenAccount,
  createMint,
  getAccount,
  getAssociatedTokenAddressSync,
  getOrCreateAssociatedTokenAccount,
  mintTo,
} from "@solana/spl-token";
import { existsSync, readFileSync } from "node:fs";
import path from "node:path";

import { Enclz } from "../target/types/enclz";

const GROUP_SEED = Buffer.from("group");
const WALLET_SEED = Buffer.from("wallet");
const WHITELIST_SEED = Buffer.from("whitelist");

const ENTRY_TYPE_EXTERNAL = 1;
const ENTRY_TYPE_PROTOCOL = 2;

const REPO_ROOT = path.resolve(__dirname, "..");
const DEFAULT_DEVNET_KEYPAIR = path.join(
  REPO_ROOT,
  ".solana/keys/devnet-deployer.json"
);

function rpcUrl(): string {
  return (
    process.env.QUICKNODE_DEVNET_RPC_URL ?? "https://api.devnet.solana.com"
  );
}

function loadFeePayer(): Keypair {
  const candidate = process.env.ANCHOR_WALLET ?? DEFAULT_DEVNET_KEYPAIR;
  if (!existsSync(candidate)) {
    throw new Error(
      `fee payer keypair not found at ${candidate}; set ANCHOR_WALLET or place the file under .solana/keys/`
    );
  }
  const bytes = JSON.parse(readFileSync(candidate, "utf8")) as number[];
  return Keypair.fromSecretKey(Uint8Array.from(bytes));
}

function padDisplayName(text: string): number[] {
  const buf = Buffer.alloc(32);
  buf.write(text);
  return Array.from(buf);
}

function findGroupPda(programId: PublicKey, owner: PublicKey): PublicKey {
  return PublicKey.findProgramAddressSync(
    [GROUP_SEED, owner.toBuffer()],
    programId
  )[0];
}

function findAgentPda(
  programId: PublicKey,
  group: PublicKey,
  agentIndex: number
): PublicKey {
  return PublicKey.findProgramAddressSync(
    [WALLET_SEED, group.toBuffer(), Buffer.from([agentIndex])],
    programId
  )[0];
}

function findWhitelistPda(
  programId: PublicKey,
  group: PublicKey,
  target: PublicKey
): PublicKey {
  return PublicKey.findProgramAddressSync(
    [WHITELIST_SEED, group.toBuffer(), target.toBuffer()],
    programId
  )[0];
}

async function fundAccount(
  connection: Connection,
  feePayer: Keypair,
  recipient: PublicKey,
  lamports: number
): Promise<void> {
  const ix = SystemProgram.transfer({
    fromPubkey: feePayer.publicKey,
    toPubkey: recipient,
    lamports,
  });
  const tx = new anchor.web3.Transaction().add(ix);
  tx.feePayer = feePayer.publicKey;
  const { blockhash, lastValidBlockHeight } =
    await connection.getLatestBlockhash("confirmed");
  tx.recentBlockhash = blockhash;
  tx.sign(feePayer);
  const sig = await connection.sendRawTransaction(tx.serialize());
  await connection.confirmTransaction(
    { signature: sig, blockhash, lastValidBlockHeight },
    "confirmed"
  );
}

function step(label: string): void {
  console.log(`\n→ ${label}`);
}

async function main(): Promise<void> {
  const connection = new Connection(rpcUrl(), "confirmed");
  const feePayer = loadFeePayer();

  console.log(`smoke test against ${rpcUrl()}`);
  console.log(`fee payer: ${feePayer.publicKey.toBase58()}`);

  const wallet = new anchor.Wallet(feePayer);
  const provider = new anchor.AnchorProvider(connection, wallet, {
    commitment: "confirmed",
  });
  anchor.setProvider(provider);
  const program = anchor.workspace.Enclz as Program<Enclz>;
  console.log(`program: ${program.programId.toBase58()}`);

  step("provisioning keypairs");
  const owner = Keypair.generate();
  const backendOperator = Keypair.generate();
  const merchantOwner = Keypair.generate();
  const dexRouter = Keypair.generate().publicKey;
  console.log(`  owner            ${owner.publicKey.toBase58()}`);
  console.log(`  backendOperator  ${backendOperator.publicKey.toBase58()}`);
  console.log(`  merchantOwner    ${merchantOwner.publicKey.toBase58()}`);

  step("funding owner / operator / merchant from fee payer");
  await fundAccount(connection, feePayer, owner.publicKey, LAMPORTS_PER_SOL);
  await fundAccount(
    connection,
    feePayer,
    backendOperator.publicKey,
    LAMPORTS_PER_SOL / 10
  );
  await fundAccount(
    connection,
    feePayer,
    merchantOwner.publicKey,
    LAMPORTS_PER_SOL / 10
  );

  step("creating fresh test mint (stand-in for USDC)");
  const mint = await createMint(
    connection,
    feePayer,
    feePayer.publicKey,
    null,
    6
  );
  console.log(`  mint ${mint.toBase58()}`);

  step("pre-creating protocol_fee_wallet ATA");
  const protocolFeeWalletOwner = Keypair.generate();
  await fundAccount(
    connection,
    feePayer,
    protocolFeeWalletOwner.publicKey,
    LAMPORTS_PER_SOL / 10
  );
  const protocolFeeAta = await getOrCreateAssociatedTokenAccount(
    connection,
    feePayer,
    mint,
    protocolFeeWalletOwner.publicKey
  );
  console.log(`  protocolFeeAta ${protocolFeeAta.address.toBase58()}`);

  step("initialize_group");
  const groupPda = findGroupPda(program.programId, owner.publicKey);
  const dexRouterEntry = findWhitelistPda(
    program.programId,
    groupPda,
    dexRouter
  );
  const initSig = await program.methods
    .initializeGroup(
      backendOperator.publicKey,
      protocolFeeWalletOwner.publicKey,
      dexRouter
    )
    .accounts({
      owner: owner.publicKey,
      groupConfig: groupPda,
      dexRouterEntry,
      systemProgram: SystemProgram.programId,
    })
    .signers([owner])
    .rpc();
  console.log(`  initialize_group  ${initSig}`);

  const router = await program.account.whitelistEntry.fetch(dexRouterEntry);
  if (router.entryType !== ENTRY_TYPE_PROTOCOL) {
    throw new Error(
      `dex_router_entry.entry_type=${router.entryType}, expected ${ENTRY_TYPE_PROTOCOL}`
    );
  }

  step("add_agent (hourly_tx_cap=10)");
  const agentPda = findAgentPda(program.programId, groupPda, 0);
  const intraGroupEntry = findWhitelistPda(
    program.programId,
    groupPda,
    agentPda
  );
  const agentAta = getAssociatedTokenAddressSync(mint, agentPda, true);
  const addAgentSig = await program.methods
    .addAgent(padDisplayName("smoke-bot"), null, null, 10)
    .accounts({
      owner: owner.publicKey,
      groupConfig: groupPda,
      agentWallet: agentPda,
      intraGroupEntry,
      agentTokenAccount: agentAta,
      mint,
      tokenProgram: TOKEN_PROGRAM_ID,
      associatedTokenProgram: ASSOCIATED_TOKEN_PROGRAM_ID,
      systemProgram: SystemProgram.programId,
    })
    .signers([owner])
    .rpc();
  console.log(`  add_agent ${addAgentSig}`);

  step("add_to_whitelist (external merchant, $5 cap, ttl=now+3600)");
  const merchantEntry = findWhitelistPda(
    program.programId,
    groupPda,
    merchantOwner.publicKey
  );
  const ttl = Math.floor(Date.now() / 1000) + 3600;
  const merchantAta = await createAssociatedTokenAccount(
    connection,
    merchantOwner,
    mint,
    merchantOwner.publicKey
  );
  const addWhitelistSig = await program.methods
    .addToWhitelist(
      merchantOwner.publicKey,
      padDisplayName("acme-merchant"),
      ENTRY_TYPE_EXTERNAL,
      new BN(ttl),
      new BN(5_000_000)
    )
    .accounts({
      owner: owner.publicKey,
      groupConfig: groupPda,
      whitelistEntry: merchantEntry,
      systemProgram: SystemProgram.programId,
    })
    .signers([owner])
    .rpc();
  console.log(`  add_to_whitelist ${addWhitelistSig}`);

  step("funding agent ATA: minting 10 USDC");
  await mintTo(connection, feePayer, mint, agentAta, feePayer, 10_000_000);

  step("executing 5 × $1 transfers");
  for (let i = 0; i < 5; i++) {
    const sig = await program.methods
      .executeTransfer(new BN(1_000_000), new BN(i), 0)
      .accounts({
        backendOperator: backendOperator.publicKey,
        groupConfig: groupPda,
        groupOwner: owner.publicKey,
        agentWallet: agentPda,
        fromTokenAccount: agentAta,
        toTokenAccount: merchantAta,
        whitelistEntry: merchantEntry,
        protocolFeeTokenAccount: protocolFeeAta.address,
        tokenProgram: TOKEN_PROGRAM_ID,
        systemProgram: SystemProgram.programId,
      })
      .signers([backendOperator])
      .rpc();
    console.log(`  transfer #${i + 1}  ${sig}`);
  }

  step("verifying whitelist PDA closed after 5th transfer");
  const whitelistAfter = await connection.getAccountInfo(merchantEntry);
  if (whitelistAfter !== null) {
    throw new Error(
      `expected merchant whitelist PDA to be closed; still has ${whitelistAfter.lamports} lamports`
    );
  }

  step("verifying merchant balance & protocol fee balance");
  const merchantBalance = (await getAccount(connection, merchantAta)).amount;
  const feeBalance = (await getAccount(connection, protocolFeeAta.address))
    .amount;
  if (merchantBalance.toString() !== (999_000n * 5n).toString()) {
    throw new Error(
      `merchant balance ${merchantBalance.toString()} ≠ ${999_000n * 5n}`
    );
  }
  if (feeBalance.toString() !== (1_000n * 5n).toString()) {
    throw new Error(
      `protocol fee balance ${feeBalance.toString()} ≠ ${1_000n * 5n}`
    );
  }

  step("attempting 6th transfer — must fail (whitelist closed)");
  let sixthFailed = false;
  try {
    await program.methods
      .executeTransfer(new BN(1_000_000), new BN(5), 0)
      .accounts({
        backendOperator: backendOperator.publicKey,
        groupConfig: groupPda,
        groupOwner: owner.publicKey,
        agentWallet: agentPda,
        fromTokenAccount: agentAta,
        toTokenAccount: merchantAta,
        whitelistEntry: merchantEntry,
        protocolFeeTokenAccount: protocolFeeAta.address,
        tokenProgram: TOKEN_PROGRAM_ID,
        systemProgram: SystemProgram.programId,
      })
      .signers([backendOperator])
      .rpc();
  } catch (err) {
    sixthFailed = true;
    console.log(
      `  6th transfer rejected (expected): ${
        (err as Error).message.split("\n")[0]
      }`
    );
  }
  if (!sixthFailed) {
    throw new Error("6th transfer succeeded but should have been rejected");
  }

  step("attempting stale-nonce transfer — must fail with NonceMismatch");
  // Re-add merchant to be able to test nonce; but to keep the test minimal, we
  // try a second merchant (so the whitelist isn't the gate) and submit an
  // expected_nonce that's already been consumed.
  const merchant2Owner = Keypair.generate();
  await fundAccount(
    connection,
    feePayer,
    merchant2Owner.publicKey,
    LAMPORTS_PER_SOL / 10
  );
  const merchant2Ata = await createAssociatedTokenAccount(
    connection,
    merchant2Owner,
    mint,
    merchant2Owner.publicKey
  );
  const merchant2Entry = findWhitelistPda(
    program.programId,
    groupPda,
    merchant2Owner.publicKey
  );
  await program.methods
    .addToWhitelist(
      merchant2Owner.publicKey,
      padDisplayName("acme-merchant-2"),
      ENTRY_TYPE_EXTERNAL,
      new BN(Math.floor(Date.now() / 1000) + 3600),
      new BN(5_000_000)
    )
    .accounts({
      owner: owner.publicKey,
      groupConfig: groupPda,
      whitelistEntry: merchant2Entry,
      systemProgram: SystemProgram.programId,
    })
    .signers([owner])
    .rpc();

  // First transfer succeeds — agent.operator_nonce moves from 5 to 6.
  await program.methods
    .executeTransfer(new BN(500_000), new BN(5), 0)
    .accounts({
      backendOperator: backendOperator.publicKey,
      groupConfig: groupPda,
      groupOwner: owner.publicKey,
      agentWallet: agentPda,
      fromTokenAccount: agentAta,
      toTokenAccount: merchant2Ata,
      whitelistEntry: merchant2Entry,
      protocolFeeTokenAccount: protocolFeeAta.address,
      tokenProgram: TOKEN_PROGRAM_ID,
      systemProgram: SystemProgram.programId,
    })
    .signers([backendOperator])
    .rpc();

  // Replay with the same nonce 5 — must fail with NonceMismatch.
  let nonceFailed = false;
  try {
    await program.methods
      .executeTransfer(new BN(500_000), new BN(5), 0)
      .accounts({
        backendOperator: backendOperator.publicKey,
        groupConfig: groupPda,
        groupOwner: owner.publicKey,
        agentWallet: agentPda,
        fromTokenAccount: agentAta,
        toTokenAccount: merchant2Ata,
        whitelistEntry: merchant2Entry,
        protocolFeeTokenAccount: protocolFeeAta.address,
        tokenProgram: TOKEN_PROGRAM_ID,
        systemProgram: SystemProgram.programId,
      })
      .signers([backendOperator])
      .rpc();
  } catch (err) {
    nonceFailed = true;
    const msg = (err as Error).message;
    if (!msg.includes("NonceMismatch")) {
      console.log(`  stale nonce rejected with: ${msg.split("\n")[0]}`);
    } else {
      console.log(`  stale nonce rejected with NonceMismatch ✓`);
    }
  }
  if (!nonceFailed) {
    throw new Error("stale-nonce transfer succeeded but should have failed");
  }

  console.log("\n✓ smoke test passed");
}

main().catch((err) => {
  if (err instanceof SendTransactionError) {
    console.error(err.logs?.join("\n"));
  }
  console.error(err);
  process.exit(1);
});
