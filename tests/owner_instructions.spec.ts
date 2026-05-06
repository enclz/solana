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

const DEFAULT_DAILY_LIMIT = 10_000_000n;
const DEFAULT_PER_TX_LIMIT = 1_000_000n;
const DEFAULT_HOURLY_CAP = 5;

const ENTRY_TYPE_INTRA_GROUP = 0;
const ENTRY_TYPE_EXTERNAL = 1;
const ENTRY_TYPE_PROTOCOL = 2;

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

async function provisionGroup(
  program: Program<Enclz>,
  provider: anchor.AnchorProvider,
  owner: Keypair,
  groupName: number[] = padDisplayName("acme-trading-desk")
): Promise<{
  group: PublicKey;
  backendOperator: PublicKey;
  protocolFeeWallet: PublicKey;
  dexRouter: PublicKey;
  groupName: number[];
}> {
  const backendOperator = Keypair.generate().publicKey;
  const protocolFeeWallet = Keypair.generate().publicKey;
  const dexRouter = Keypair.generate().publicKey;
  const [group] = findGroupPda(program.programId, owner.publicKey);
  const [dexRouterEntry] = findWhitelistPda(
    program.programId,
    group,
    dexRouter
  );
  await program.methods
    .initializeGroup(groupName, backendOperator, protocolFeeWallet, dexRouter)
    .accounts({
      owner: owner.publicKey,
      groupConfig: group,
      dexRouterEntry,
      systemProgram: SystemProgram.programId,
    })
    .signers([owner])
    .rpc();
  return { group, backendOperator, protocolFeeWallet, dexRouter, groupName };
}

async function addAgent(
  program: Program<Enclz>,
  owner: Keypair,
  group: PublicKey,
  agentIndex: number,
  mint: PublicKey,
  displayName: string,
  options: {
    dailyLimit?: BN | null;
    perTxLimit?: BN | null;
    hourlyTxCap?: number | null;
  } = {}
): Promise<{ agent: PublicKey; ata: PublicKey; intraEntry: PublicKey }> {
  const [agent] = findAgentPda(program.programId, group, agentIndex);
  const [intraEntry] = findWhitelistPda(program.programId, group, agent);
  const ata = getAssociatedTokenAddressSync(mint, agent, true);
  await program.methods
    .addAgent(
      padDisplayName(displayName),
      options.dailyLimit ?? null,
      options.perTxLimit ?? null,
      options.hourlyTxCap ?? null
    )
    .accounts({
      owner: owner.publicKey,
      groupConfig: group,
      agentWallet: agent,
      intraGroupEntry: intraEntry,
      agentTokenAccount: ata,
      mint,
      tokenProgram: TOKEN_PROGRAM_ID,
      associatedTokenProgram: ASSOCIATED_TOKEN_PROGRAM_ID,
      systemProgram: SystemProgram.programId,
    })
    .signers([owner])
    .rpc();
  return { agent, ata, intraEntry };
}

describe("enclz owner instructions (mocha + anchor)", function () {
  this.timeout(60_000);
  const provider = anchor.AnchorProvider.env();
  anchor.setProvider(provider);
  const program = anchor.workspace.Enclz as Program<Enclz>;

  let owner: Keypair;
  let mintAuthority: Keypair;

  beforeEach(async () => {
    owner = Keypair.generate();
    mintAuthority = Keypair.generate();
    await airdrop(provider, owner.publicKey, 5 * LAMPORTS_PER_SOL);
    await airdrop(provider, mintAuthority.publicKey, 2 * LAMPORTS_PER_SOL);
  });

  it("provisions a group, two agents, then renews and removes a merchant entry", async () => {
    const { group, backendOperator, protocolFeeWallet, dexRouter, groupName } =
      await provisionGroup(program, provider, owner);
    const groupAfterInit = await program.account.groupConfig.fetch(group);
    expect(groupAfterInit.owner.equals(owner.publicKey)).to.equal(true);
    expect(groupAfterInit.backendOperator.equals(backendOperator)).to.equal(
      true
    );
    expect(groupAfterInit.protocolFeeWallet.equals(protocolFeeWallet)).to.equal(
      true
    );
    expect(groupAfterInit.agentCount).to.equal(0);
    expect(Array.from(groupAfterInit.groupName)).to.deep.equal(groupName);

    const [routerEntryPda] = findWhitelistPda(
      program.programId,
      group,
      dexRouter
    );
    const routerEntry = await program.account.whitelistEntry.fetch(
      routerEntryPda
    );
    expect(routerEntry.entryType).to.equal(ENTRY_TYPE_PROTOCOL);
    expect(routerEntry.ttlExpiresAt.toString()).to.equal("0");
    expect(routerEntry.approvedAmount.toString()).to.equal("0");

    const mint = await createMint(
      provider.connection,
      mintAuthority,
      mintAuthority.publicKey,
      null,
      6
    );

    const first = await addAgent(
      program,
      owner,
      group,
      0,
      mint,
      "research-bot-1"
    );
    const firstAgent = await program.account.agentWallet.fetch(first.agent);
    expect(firstAgent.dailyLimit.toString()).to.equal(
      DEFAULT_DAILY_LIMIT.toString()
    );
    expect(firstAgent.perTxLimit.toString()).to.equal(
      DEFAULT_PER_TX_LIMIT.toString()
    );
    expect(firstAgent.hourlyTxCap).to.equal(DEFAULT_HOURLY_CAP);

    const intraEntry = await program.account.whitelistEntry.fetch(
      first.intraEntry
    );
    expect(intraEntry.entryType).to.equal(ENTRY_TYPE_INTRA_GROUP);
    expect(intraEntry.ttlExpiresAt.toString()).to.equal("0");
    expect(intraEntry.approvedAmount.toString()).to.equal("0");

    const firstAtaState = await getAccount(provider.connection, first.ata);
    expect(firstAtaState.owner.equals(first.agent)).to.equal(true);
    expect(firstAtaState.mint.equals(mint)).to.equal(true);

    const second = await addAgent(
      program,
      owner,
      group,
      1,
      mint,
      "research-bot-2",
      { dailyLimit: new BN(50_000_000) }
    );
    const secondAgent = await program.account.agentWallet.fetch(second.agent);
    expect(secondAgent.dailyLimit.toString()).to.equal("50000000");
    expect(secondAgent.perTxLimit.toString()).to.equal(
      DEFAULT_PER_TX_LIMIT.toString()
    );
    const groupAfterAgents = await program.account.groupConfig.fetch(group);
    expect(groupAfterAgents.agentCount).to.equal(2);

    const merchant = Keypair.generate().publicKey;
    const [merchantEntry] = findWhitelistPda(
      program.programId,
      group,
      merchant
    );
    const initialTtl = Math.floor(Date.now() / 1000) + 3_600;
    await program.methods
      .addToWhitelist(
        merchant,
        padDisplayName("acme-merchant"),
        ENTRY_TYPE_EXTERNAL,
        new BN(initialTtl),
        new BN(10_000_000)
      )
      .accounts({
        owner: owner.publicKey,
        groupConfig: group,
        whitelistEntry: merchantEntry,
        systemProgram: SystemProgram.programId,
      })
      .signers([owner])
      .rpc();
    const merchantStateInitial = await program.account.whitelistEntry.fetch(
      merchantEntry
    );
    expect(merchantStateInitial.entryType).to.equal(ENTRY_TYPE_EXTERNAL);
    expect(merchantStateInitial.ttlExpiresAt.toNumber()).to.equal(initialTtl);
    expect(merchantStateInitial.approvedAmount.toString()).to.equal("10000000");

    const renewedTtl = Math.floor(Date.now() / 1000) + 86_400;
    await program.methods
      .renewWhitelistEntry(merchant, new BN(renewedTtl), new BN(20_000_000))
      .accounts({
        owner: owner.publicKey,
        groupConfig: group,
        whitelistEntry: merchantEntry,
      })
      .signers([owner])
      .rpc();
    const merchantStateRenewed = await program.account.whitelistEntry.fetch(
      merchantEntry
    );
    expect(merchantStateRenewed.ttlExpiresAt.toNumber()).to.equal(renewedTtl);
    expect(merchantStateRenewed.approvedAmount.toString()).to.equal("20000000");
    const [merchantRederived] = findWhitelistPda(
      program.programId,
      group,
      merchant
    );
    expect(merchantRederived.equals(merchantEntry)).to.equal(true);

    await program.methods
      .removeFromWhitelist(merchant)
      .accounts({
        owner: owner.publicKey,
        groupConfig: group,
        whitelistEntry: merchantEntry,
      })
      .signers([owner])
      .rpc();
    const closed = await provider.connection.getAccountInfo(merchantEntry);
    expect(closed).to.equal(null);
  });

  it("emergency_withdraw sweeps the full agent ATA balance to a destination ATA", async () => {
    const { group } = await provisionGroup(program, provider, owner);
    const mint = await createMint(
      provider.connection,
      mintAuthority,
      mintAuthority.publicKey,
      null,
      6
    );
    const { agent, ata } = await addAgent(
      program,
      owner,
      group,
      0,
      mint,
      "fund-me-then-sweep"
    );

    const fundedAmount = 25_000_000;
    await mintTo(
      provider.connection,
      mintAuthority,
      mint,
      ata,
      mintAuthority,
      fundedAmount
    );
    const beforeSweep = await getAccount(provider.connection, ata);
    expect(beforeSweep.amount.toString()).to.equal(fundedAmount.toString());

    const destinationOwner = Keypair.generate();
    await airdrop(provider, destinationOwner.publicKey, LAMPORTS_PER_SOL);
    const destinationAta = await createAssociatedTokenAccount(
      provider.connection,
      destinationOwner,
      mint,
      destinationOwner.publicKey
    );

    await program.methods
      .emergencyWithdraw(0)
      .accounts({
        owner: owner.publicKey,
        groupConfig: group,
        agentWallet: agent,
        agentTokenAccount: ata,
        destinationTokenAccount: destinationAta,
        mint,
        tokenProgram: TOKEN_PROGRAM_ID,
      })
      .signers([owner])
      .rpc();

    const afterSweep = await getAccount(provider.connection, ata);
    expect(afterSweep.amount.toString()).to.equal("0");
    const destination = await getAccount(provider.connection, destinationAta);
    expect(destination.amount.toString()).to.equal(fundedAmount.toString());
  });
});
