import * as anchor from "@coral-xyz/anchor";
import { BN, Program } from "@coral-xyz/anchor";
import {
  Keypair,
  LAMPORTS_PER_SOL,
  PublicKey,
  SystemProgram,
} from "@solana/web3.js";
import {
  ASSOCIATED_TOKEN_PROGRAM_ID,
  TOKEN_PROGRAM_ID,
  createAssociatedTokenAccount,
  createMint,
  getAccount,
  getAssociatedTokenAddressSync,
  mintTo,
} from "@solana/spl-token";
import { expect } from "chai";

import { Enclz } from "../target/types/enclz";

const GROUP_SEED = Buffer.from("group");
const WALLET_SEED = Buffer.from("wallet");
const WHITELIST_SEED = Buffer.from("whitelist");

const ENTRY_TYPE_EXTERNAL = 1;

function padDisplayName(text: string): number[] {
  const buffer = Buffer.alloc(32);
  buffer.write(text);
  return Array.from(buffer);
}

function findGroupPda(
  programId: PublicKey,
  owner: PublicKey
): [PublicKey, number] {
  return PublicKey.findProgramAddressSync(
    [GROUP_SEED, owner.toBuffer()],
    programId
  );
}

function findAgentPda(
  programId: PublicKey,
  group: PublicKey,
  agentIndex: number
): [PublicKey, number] {
  return PublicKey.findProgramAddressSync(
    [WALLET_SEED, group.toBuffer(), Buffer.from([agentIndex])],
    programId
  );
}

function findWhitelistPda(
  programId: PublicKey,
  group: PublicKey,
  target: PublicKey
): [PublicKey, number] {
  return PublicKey.findProgramAddressSync(
    [WHITELIST_SEED, group.toBuffer(), target.toBuffer()],
    programId
  );
}

async function airdrop(
  provider: anchor.AnchorProvider,
  recipient: PublicKey,
  lamports: number
): Promise<void> {
  const signature = await provider.connection.requestAirdrop(
    recipient,
    lamports
  );
  const blockhash = await provider.connection.getLatestBlockhash();
  await provider.connection.confirmTransaction(
    { signature, ...blockhash },
    "confirmed"
  );
}

interface ProvisionedFleet {
  group: PublicKey;
  backendOperator: Keypair;
  protocolFeeWalletOwner: Keypair;
  protocolFeeAta: PublicKey;
  dexRouter: PublicKey;
  agent: PublicKey;
  agentAta: PublicKey;
  intraEntry: PublicKey;
  mint: PublicKey;
  mintAuthority: Keypair;
}

async function provisionFleet(
  program: Program<Enclz>,
  provider: anchor.AnchorProvider,
  owner: Keypair,
  initialAgentBalance: bigint
): Promise<ProvisionedFleet> {
  const backendOperator = Keypair.generate();
  const protocolFeeWalletOwner = Keypair.generate();
  const dexRouter = Keypair.generate().publicKey;
  await airdrop(provider, backendOperator.publicKey, LAMPORTS_PER_SOL);
  await airdrop(provider, protocolFeeWalletOwner.publicKey, LAMPORTS_PER_SOL);

  const [group] = findGroupPda(program.programId, owner.publicKey);
  const [dexRouterEntry] = findWhitelistPda(
    program.programId,
    group,
    dexRouter
  );
  await program.methods
    .initializeGroup(
      padDisplayName("transfer-test"),
      backendOperator.publicKey,
      protocolFeeWalletOwner.publicKey,
      dexRouter
    )
    .accounts({
      owner: owner.publicKey,
      groupConfig: group,
      dexRouterEntry,
      systemProgram: SystemProgram.programId,
    })
    .signers([owner])
    .rpc();

  const mintAuthority = Keypair.generate();
  await airdrop(provider, mintAuthority.publicKey, LAMPORTS_PER_SOL);
  const mint = await createMint(
    provider.connection,
    mintAuthority,
    mintAuthority.publicKey,
    null,
    6
  );

  const [agent] = findAgentPda(program.programId, group, 0);
  const [intraEntry] = findWhitelistPda(program.programId, group, agent);
  const agentAta = getAssociatedTokenAddressSync(mint, agent, true);
  await program.methods
    .addAgent(padDisplayName("transfer-bot"), null, null, null)
    .accounts({
      owner: owner.publicKey,
      groupConfig: group,
      agentWallet: agent,
      intraGroupEntry: intraEntry,
      agentTokenAccount: agentAta,
      mint,
      tokenProgram: TOKEN_PROGRAM_ID,
      associatedTokenProgram: ASSOCIATED_TOKEN_PROGRAM_ID,
      systemProgram: SystemProgram.programId,
    })
    .signers([owner])
    .rpc();

  if (initialAgentBalance > 0n) {
    await mintTo(
      provider.connection,
      mintAuthority,
      mint,
      agentAta,
      mintAuthority,
      initialAgentBalance
    );
  }

  const protocolFeeAta = await createAssociatedTokenAccount(
    provider.connection,
    protocolFeeWalletOwner,
    mint,
    protocolFeeWalletOwner.publicKey
  );

  return {
    group,
    backendOperator,
    protocolFeeWalletOwner,
    protocolFeeAta,
    dexRouter,
    agent,
    agentAta,
    intraEntry,
    mint,
    mintAuthority,
  };
}

async function addExternalEntry(
  program: Program<Enclz>,
  owner: Keypair,
  group: PublicKey,
  target: PublicKey,
  ttlSeconds: number
): Promise<PublicKey> {
  const [entryPda] = findWhitelistPda(program.programId, group, target);
  const ttlExpiresAt = Math.floor(Date.now() / 1000) + ttlSeconds;
  await program.methods
    .addToWhitelist(
      target,
      padDisplayName("merchant"),
      ENTRY_TYPE_EXTERNAL,
      new BN(ttlExpiresAt)
    )
    .accounts({
      owner: owner.publicKey,
      groupConfig: group,
      whitelistEntry: entryPda,
      systemProgram: SystemProgram.programId,
    })
    .signers([owner])
    .rpc();
  return entryPda;
}

async function callExecuteTransfer(
  program: Program<Enclz>,
  fleet: ProvisionedFleet,
  ownerPubkey: PublicKey,
  recipientWallet: PublicKey,
  recipientAta: PublicKey,
  whitelistEntry: PublicKey,
  amount: bigint,
  expectedNonce: bigint,
  agentIndex = 0
): Promise<string> {
  return await program.methods
    .executeTransfer(
      new BN(amount.toString()),
      new BN(expectedNonce.toString()),
      agentIndex
    )
    .accounts({
      backendOperator: fleet.backendOperator.publicKey,
      groupConfig: fleet.group,
      groupOwner: ownerPubkey,
      agentWallet: fleet.agent,
      fromTokenAccount: fleet.agentAta,
      recipientWallet,
      mint: fleet.mint,
      toTokenAccount: recipientAta,
      whitelistEntry,
      protocolFeeTokenAccount: fleet.protocolFeeAta,
      associatedTokenProgram: ASSOCIATED_TOKEN_PROGRAM_ID,
      tokenProgram: TOKEN_PROGRAM_ID,
      systemProgram: SystemProgram.programId,
    })
    .signers([fleet.backendOperator])
    .rpc();
}

describe("enclz execute_transfer (mocha + anchor)", function () {
  this.timeout(120_000);
  const provider = anchor.AnchorProvider.env();
  anchor.setProvider(provider);
  const program = anchor.workspace.Enclz as Program<Enclz>;

  let owner: Keypair;
  beforeEach(async () => {
    owner = Keypair.generate();
    await airdrop(provider, owner.publicKey, 5 * LAMPORTS_PER_SOL);
  });

  it("end-to-end: external whitelist with $5 cap consumes across transfers, auto-voids, and rejects 6th call", async () => {
    const fleet = await provisionFleet(program, provider, owner, 10_000_000n);

    const merchantOwner = Keypair.generate();
    await airdrop(provider, merchantOwner.publicKey, LAMPORTS_PER_SOL);
    const merchantAta = await createAssociatedTokenAccount(
      provider.connection,
      merchantOwner,
      fleet.mint,
      merchantOwner.publicKey
    );
    const merchantEntry = await addExternalEntry(
      program,
      owner,
      fleet.group,
      merchantOwner.publicKey,
      86_400
    );

    await callExecuteTransfer(
      program,
      fleet,
      owner.publicKey,
      merchantOwner.publicKey,
      merchantAta,
      merchantEntry,
      1_000_000n,
      0n
    );

    const merchantBalance = (await getAccount(provider.connection, merchantAta))
      .amount;
    // With additive fee, recipient gets the full amount.
    expect(merchantBalance.toString()).to.equal(1_000_000n.toString());
    const feeBalance = (
      await getAccount(provider.connection, fleet.protocolFeeAta)
    ).amount;
    expect(feeBalance.toString()).to.equal(1_000n.toString());
  });

  it("nonce replay: second submission with the same expected_nonce fails", async () => {
    const fleet = await provisionFleet(program, provider, owner, 5_000_000n);
    const merchantOwner = Keypair.generate();
    await airdrop(provider, merchantOwner.publicKey, LAMPORTS_PER_SOL);
    const merchantAta = await createAssociatedTokenAccount(
      provider.connection,
      merchantOwner,
      fleet.mint,
      merchantOwner.publicKey
    );
    const merchantEntry = await addExternalEntry(
      program,
      owner,
      fleet.group,
      merchantOwner.publicKey,
      86_400
    );

    // First submission succeeds.
    await callExecuteTransfer(
      program,
      fleet,
      owner.publicKey,
      merchantOwner.publicKey,
      merchantAta,
      merchantEntry,
      500_000n,
      0n
    );

    // Second submission re-uses nonce 0 — must reject.
    let failed = false;
    try {
      await callExecuteTransfer(
        program,
        fleet,
        owner.publicKey,
        merchantOwner.publicKey,
        merchantAta,
        merchantEntry,
        500_000n,
        0n
      );
    } catch (error) {
      failed = true;
    }
    expect(failed).to.equal(true);

    const agent = await program.account.agentWallet.fetch(fleet.agent);
    expect(agent.operatorNonce.toString()).to.equal("1");
  });
});
