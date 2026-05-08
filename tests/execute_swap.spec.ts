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

const ENTRY_TYPE_PROTOCOL = 2;

// Test stub program loaded by Anchor.toml [[test.genesis]] — stands in for
// Jupiter v6. Opcode 0 (or empty data) is a no-op.
const STUB_PROGRAM_ID = new PublicKey(
  "4PhEhEZuZbQTC7WpKS6yMoRV6ySmpXVXUvPHQ624XQDU"
);

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

describe("enclz execute_swap (mocha + anchor)", function () {
  this.timeout(120_000);
  const provider = anchor.AnchorProvider.env();
  anchor.setProvider(provider);
  const program = anchor.workspace.Enclz as Program<Enclz>;

  it("happy path: swap deducts 10bps fee, calls stub Jupiter, increments counters", async () => {
    const owner = Keypair.generate();
    await airdrop(provider, owner.publicKey, 5 * LAMPORTS_PER_SOL);
    const backendOperator = Keypair.generate();
    const protocolFeeOwner = Keypair.generate();
    await airdrop(provider, backendOperator.publicKey, LAMPORTS_PER_SOL);
    await airdrop(provider, protocolFeeOwner.publicKey, LAMPORTS_PER_SOL);

    // Use the stub itself as the dex_router so the auto-created type-2
    // entry from initialize_group already covers the Jupiter call.
    const group = findGroupPda(program.programId, owner.publicKey);
    const dexRouterEntry = findWhitelistPda(
      program.programId,
      group,
      STUB_PROGRAM_ID
    );
    await program.methods
      .initializeGroup(
        padDisplayName("swap-test"),
        backendOperator.publicKey,
        protocolFeeOwner.publicKey,
        STUB_PROGRAM_ID
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
      .addAgent(padDisplayName("swap-bot"), null, null, null)
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

    // Output ATA must be owned by the agent_wallet PDA (custody pin). Use a
    // fresh mint so we exercise the realistic "swap into a novel mint" shape.
    const outputMint = await createMint(
      provider.connection,
      mintAuthority,
      mintAuthority.publicKey,
      null,
      6
    );
    const outputAta = await createAssociatedTokenAccount(
      provider.connection,
      owner,
      outputMint,
      agent
    );

    await program.methods
      .executeSwap(
        new BN(1_000_000),
        new BN(0),
        new BN(0),
        0,
        Buffer.from([0]) // noop opcode
      )
      .accounts({
        backendOperator: backendOperator.publicKey,
        groupConfig: group,
        agentWallet: agent,
        fromTokenAccount: agentAta,
        toTokenAccount: outputAta,
        whitelistEntry: dexRouterEntry,
        inputMint: mint,
        protocolFeeTokenAccount: protocolFeeAta,
        protocolFeeWallet: protocolFeeOwner.publicKey,
        jupiterProgram: STUB_PROGRAM_ID,
        tokenProgram: TOKEN_PROGRAM_ID,
        associatedTokenProgram: ASSOCIATED_TOKEN_PROGRAM_ID,
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
    // spent_today is no longer touched on the swap path under the new policy.
    expect(agentState.spentToday.toString()).to.equal("0");
    expect(agentState.txCountThisHour).to.equal(1);
  });
});
