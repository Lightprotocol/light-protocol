import * as anchor from "@project-serum/anchor";
import { Program } from "@project-serum/anchor";
import { VerifierProgram } from "../target/types/verifier_program";
const { SystemProgram } = require('@solana/web3.js');
import { findProgramAddressSync } from "@project-serum/anchor/dist/cjs/utils/pubkey";

const newAccountWithLamports = async (connection, lamports = 1e10) => {
  const account = new anchor.web3.Account()

  let retries = 30
  await connection.requestAirdrop(account.publicKey, lamports)
  for (;;) {
    await sleep(500)
    // eslint-disable-next-line eqeqeq
    if (lamports == (await connection.getBalance(account.publicKey))) {
      return account
    }
    if (--retries <= 0) {
      break
    }
  }
  throw new Error(`Airdrop of ${lamports} failed`)
}
const sleep = (ms) => {
  return new Promise((resolve) => setTimeout(resolve, ms))
}


describe("verifier_program", () => {
  // Configure the client to use the local cluster.
  anchor.setProvider(anchor.AnchorProvider.env());
  const provider = anchor.AnchorProvider.local();

  const program = anchor.workspace.VerifierProgram as Program<VerifierProgram>;

  it("Is initialized!", async () => {
    const userAccount = await newAccountWithLamports(provider.connection) // new anchor.web3.Account()
    let [pda, bump] = findProgramAddressSync(
        [
          anchor.utils.bytes.utf8.encode("data_holder_v0"),
          userAccount.publicKey.toBuffer(),
        ],
        program.programId
      );
    // Add your test here.

    // console.log("program.rpc: ", await program.methods)
    // const tx = await program.methods.createBar(
    //       new anchor.BN(2)),
    //       // new Uint8Array(32).fill(1),
    //       {
    //           accounts: {
    //             userAccount: userAccount.publicKey,
    //             authority: provider.wallet.publicKey,
    //             systemProgram: SystemProgram.programId,
    //           },
    //           signers: [provider.wallet.publicKey],
    //         }).rpc();
    // const tx1 = await program.methods.updateBar(
    //       new anchor.BN(2),
    //       // new Uint8Array(32).fill(1),
    //       {
    //           accounts: {
    //             userAccount: userAccount.publicKey,
    //             user: provider.wallet.publicKey,
    //             // systemProgram: SystemProgram.programId,
    //           },
    //           signers: [provider.wallet.publicKey],
    //         }).rpc();
    const userAccountInfo1 = await provider.connection.getAccountInfo(
          userAccount.publicKey
        )
    console.log(provider.wallet.publicKey)
    const tx = await program.methods.createBar(
            // new anchor.BN(2)
            // new Uint8Array(32).fill(1)
            // "nice"
            // new Uint8Array(3).fill(1)
          ).accounts(
              {
                authority: userAccount.publicKey,
                bar: pda,
                systemProgram: SystemProgram.programId,
              }
            ).signers([userAccount])
            .rpc();
      const tx1 = await program.methods.updateBar(
              // new anchor.BN(2)
              new Uint8Array(32).fill(1)
              // "nice"
              // new Uint8Array(3).fill(1)
            ).accounts(
                {
                  authority: userAccount.publicKey,
                  bar: pda,
                  systemProgram: SystemProgram.programId,
                }
              ).signers([userAccount])
              .rpc();
    // console.log("Your transaction signature", tx);
    // const accountInfo = await program.getAccountInfo( new solana.PublicKey(storage_account_pkey) );
    const userAccountInfo = await provider.connection.getAccountInfo(
          pda
        )
    console.log(userAccountInfo.data.slice(40,72))

  });
});
