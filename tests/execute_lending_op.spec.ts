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
  AuthorityType,
  TOKEN_PROGRAM_ID,
  createAssociatedTokenAccount,
  createMint,
  getAccount,
  getAssociatedTokenAddressSync,
  mintTo,
  setAuthority,
} from "@solana/spl-token";
import { expect } from "chai";

import { Enclz } from "../target/types/enclz";

const GROUP_SEED = Buffer.from("group");
const WALLET_SEED = Buffer.from("wallet");
const WHITELIST_SEED = Buffer.from("whitelist");
const STUB_AUTH_SEED = Buffer.from("stub-auth");

const ENTRY_TYPE_PROTOCOL = 2;

const STUB_PROGRAM_ID = new PublicKey(
  "4PhEhEZuZbQTC7WpKS6yMoRV6ySmpXVXUvPHQ624XQDU"
);

const OP_DEPOSIT = 0;
const OP_WITHDRAW = 1;

function padDisplayName(text: string): number[] {
  const buffer = Buffer.alloc(32);
  buffer.write(text);
  return Array.from(buffer);
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

function findStubAuthority(): PublicKey {
  return PublicKey.findProgramAddressSync([STUB_AUTH_SEED], STUB_PROGRAM_ID)[0];
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

describe("enclz execute_lending_op (mocha + anchor)", function () {
  this.timeout(120_000);
  const provider = anchor.AnchorProvider.env();
  anchor.setProvider(provider);
  const program = anchor.workspace.Enclz as Program<Enclz>;

  it("deposit happy path: 10bps fee deducted before lending CPI, counters bump", async () => {
    const owner = Keypair.generate();
    await airdrop(provider, owner.publicKey, 5 * LAMPORTS_PER_SOL);
    const backendOperator = Keypair.generate();
    const protocolFeeOwner = Keypair.generate();
    await airdrop(provider, backendOperator.publicKey, LAMPORTS_PER_SOL);
    await airdrop(provider, protocolFeeOwner.publicKey, LAMPORTS_PER_SOL);

    const group = findGroupPda(program.programId, owner.publicKey);
    // Use any pubkey as dex_router; the lending program goes through a
    // separately-added type-2 whitelist entry below.
    const dexRouter = Keypair.generate().publicKey;
    const dexRouterEntry = findWhitelistPda(program.programId, group, dexRouter);
    await program.methods
      .initializeGroup(
        padDisplayName("lending-test"),
        backendOperator.publicKey,
        protocolFeeOwner.publicKey,
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

    const agent = findAgentPda(program.programId, group, 0);
    const intraEntry = findWhitelistPda(program.programId, group, agent);
    const agentAta = getAssociatedTokenAddressSync(mint, agent, true);
    await program.methods
      .addAgent(padDisplayName("yield-bot"), null, null, null)
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
    await mintTo(
      provider.connection,
      mintAuthority,
      mint,
      agentAta,
      mintAuthority,
      5_000_000n
    );

    const protocolFeeAta = await createAssociatedTokenAccount(
      provider.connection,
      protocolFeeOwner,
      mint,
      protocolFeeOwner.publicKey
    );

    // Whitelist the stub program as type-2 (PROTOCOL).
    const lendingEntry = findWhitelistPda(program.programId, group, STUB_PROGRAM_ID);
    await program.methods
      .addToWhitelist(
        STUB_PROGRAM_ID,
        padDisplayName("kamino"),
        ENTRY_TYPE_PROTOCOL,
        new BN(0),
        new BN(0)
      )
      .accounts({
        owner: owner.publicKey,
        groupConfig: group,
        whitelistEntry: lendingEntry,
        systemProgram: SystemProgram.programId,
      })
      .signers([owner])
      .rpc();

    await program.methods
      .executeLendingOp(
        OP_DEPOSIT,
        new BN(1_000_000),
        new BN(0),
        0,
        Buffer.from([0])
      )
      .accounts({
        backendOperator: backendOperator.publicKey,
        groupConfig: group,
        agentWallet: agent,
        agentTokenAccount: agentAta,
        whitelistEntry: lendingEntry,
        protocolFeeTokenAccount: protocolFeeAta,
        lendingProgram: STUB_PROGRAM_ID,
        tokenProgram: TOKEN_PROGRAM_ID,
        systemProgram: SystemProgram.programId,
      })
      .signers([backendOperator])
      .rpc();

    const feeBalance = (await getAccount(provider.connection, protocolFeeAta))
      .amount;
    expect(feeBalance.toString()).to.equal("1000");
    const agentBalance = (await getAccount(provider.connection, agentAta))
      .amount;
    expect(agentBalance.toString()).to.equal((5_000_000n - 1_000n).toString());
    const agentState = await program.account.agentWallet.fetch(agent);
    expect(agentState.operatorNonce.toString()).to.equal("1");
    expect(agentState.spentToday.toString()).to.equal("1000000");
  });

  it("withdraw happy path: stub mints redeemed tokens, fee taken from delta", async () => {
    const owner = Keypair.generate();
    await airdrop(provider, owner.publicKey, 5 * LAMPORTS_PER_SOL);
    const backendOperator = Keypair.generate();
    const protocolFeeOwner = Keypair.generate();
    await airdrop(provider, backendOperator.publicKey, LAMPORTS_PER_SOL);
    await airdrop(provider, protocolFeeOwner.publicKey, LAMPORTS_PER_SOL);

    const group = findGroupPda(program.programId, owner.publicKey);
    const dexRouter = Keypair.generate().publicKey;
    const dexRouterEntry = findWhitelistPda(program.programId, group, dexRouter);
    await program.methods
      .initializeGroup(
        padDisplayName("lending-test"),
        backendOperator.publicKey,
        protocolFeeOwner.publicKey,
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

    const tempAuthority = Keypair.generate();
    await airdrop(provider, tempAuthority.publicKey, LAMPORTS_PER_SOL);
    const mint = await createMint(
      provider.connection,
      tempAuthority,
      tempAuthority.publicKey,
      null,
      6
    );

    const agent = findAgentPda(program.programId, group, 0);
    const intraEntry = findWhitelistPda(program.programId, group, agent);
    const agentAta = getAssociatedTokenAddressSync(mint, agent, true);
    await program.methods
      .addAgent(padDisplayName("yield-bot"), null, null, null)
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

    // Hand off mint authority to the stub PDA so the stub can mint during
    // withdraw CPI.
    const stubAuth = findStubAuthority();
    await setAuthority(
      provider.connection,
      tempAuthority,
      mint,
      tempAuthority,
      AuthorityType.MintTokens,
      stubAuth
    );

    const protocolFeeAta = await createAssociatedTokenAccount(
      provider.connection,
      protocolFeeOwner,
      mint,
      protocolFeeOwner.publicKey
    );

    const lendingEntry = findWhitelistPda(program.programId, group, STUB_PROGRAM_ID);
    await program.methods
      .addToWhitelist(
        STUB_PROGRAM_ID,
        padDisplayName("kamino"),
        ENTRY_TYPE_PROTOCOL,
        new BN(0),
        new BN(0)
      )
      .accounts({
        owner: owner.publicKey,
        groupConfig: group,
        whitelistEntry: lendingEntry,
        systemProgram: SystemProgram.programId,
      })
      .signers([owner])
      .rpc();

    // Stub opcode 0x01 + u64 LE amount → mints to remaining_accounts[2].
    const redeemed = 1_000_000n;
    const cpiData = Buffer.alloc(9);
    cpiData.writeUInt8(1, 0);
    cpiData.writeBigUInt64LE(redeemed, 1);

    await program.methods
      .executeLendingOp(
        OP_WITHDRAW,
        new BN(redeemed.toString()),
        new BN(0),
        0,
        cpiData
      )
      .accounts({
        backendOperator: backendOperator.publicKey,
        groupConfig: group,
        agentWallet: agent,
        agentTokenAccount: agentAta,
        whitelistEntry: lendingEntry,
        protocolFeeTokenAccount: protocolFeeAta,
        lendingProgram: STUB_PROGRAM_ID,
        tokenProgram: TOKEN_PROGRAM_ID,
        systemProgram: SystemProgram.programId,
      })
      .remainingAccounts([
        { pubkey: TOKEN_PROGRAM_ID, isSigner: false, isWritable: false },
        { pubkey: mint, isSigner: false, isWritable: true },
        { pubkey: agentAta, isSigner: false, isWritable: true },
        { pubkey: stubAuth, isSigner: false, isWritable: false },
      ])
      .signers([backendOperator])
      .rpc();

    const feeBalance = (await getAccount(provider.connection, protocolFeeAta))
      .amount;
    expect(feeBalance.toString()).to.equal("1000");
    const agentBalance = (await getAccount(provider.connection, agentAta))
      .amount;
    expect(agentBalance.toString()).to.equal("999000");
  });
});
