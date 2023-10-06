import * as anchor from "@coral-xyz/anchor";
import {
  closeMerkleTreeUpdateState,
  executeUpdateMerkleTreeTransactions,
  MerkleTreeConfig,
  SolMerkleTree,
} from "../merkleTree/index";
import { confirmConfig, merkleTreeProgramId } from "../constants";
import { IDL_MERKLE_TREE_PROGRAM } from "../idls/index";
import { Connection, Keypair } from "@solana/web3.js";
import { sleep } from "../index";

export async function updateMerkleTreeForTest(payer: Keypair, url: string) {
  const connection = new Connection(url, confirmConfig);

  const anchorProvider = new anchor.AnchorProvider(
    connection,
    new anchor.Wallet(Keypair.generate()),
    confirmConfig,
  );

  const merkleTreeProgram = new anchor.Program(
    IDL_MERKLE_TREE_PROGRAM,
    merkleTreeProgramId,
    anchorProvider && anchorProvider,
  );

  try {
    const transactionMerkleTreePda =
      MerkleTreeConfig.getTransactionMerkleTreePda();

    let leavesPdas: any[] = [];
    let retries = 5;
    while (leavesPdas.length === 0 && retries > 0) {
      if (retries !== 5) await sleep(1000);
      leavesPdas = await SolMerkleTree.getUninsertedLeavesRelayer(
        transactionMerkleTreePda,
        anchorProvider && anchorProvider,
      );
      retries--;
    }

    await executeUpdateMerkleTreeTransactions({
      connection,
      signer: payer,
      merkleTreeProgram,
      leavesPdas,
      transactionMerkleTree: transactionMerkleTreePda,
    });
  } catch (err) {
    // TODO: Revisit recovery.
    // Rn, we're just blanked-closing the update state account on failure which
    // might not be desirable in some cases.
    console.error("failed at updateMerkleTreeForTest", err);
    try {
      console.log("closing update state account...");
      await closeMerkleTreeUpdateState(merkleTreeProgram, payer, connection);
      console.log("successfully closed update state account");
    } catch (e) {
      // TODO: append to error stack trace or solve differently
      console.log("failed to close update state account");
    }
    throw err;
  }
}
