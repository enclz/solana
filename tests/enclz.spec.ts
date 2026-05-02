import * as anchor from "@coral-xyz/anchor";
import { expect } from "chai";

describe("enclz program scaffold", () => {
  const provider = anchor.AnchorProvider.env();
  anchor.setProvider(provider);

  const program = anchor.workspace.Enclz as anchor.Program;

  it("loads the deployed program from the local validator", async () => {
    const accountInfo = await provider.connection.getAccountInfo(
      program.programId
    );
    expect(accountInfo, "program not deployed to test validator").to.not.be
      .null;
    expect(accountInfo!.executable).to.equal(true);
  });

  it("derives the GroupConfig PDA with the documented seeds", () => {
    const owner = anchor.web3.Keypair.generate().publicKey;
    const [pda] = anchor.web3.PublicKey.findProgramAddressSync(
      [Buffer.from("group"), owner.toBuffer()],
      program.programId
    );
    expect(pda).to.be.instanceOf(anchor.web3.PublicKey);
  });
});
