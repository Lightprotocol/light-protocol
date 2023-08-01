import * as anchor from "@coral-xyz/anchor";
import {
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

  try {
    const merkleTreeProgram = new anchor.Program(
      IDL_MERKLE_TREE_PROGRAM,
      merkleTreeProgramId,
      anchorProvider && anchorProvider,
    );

    const transactionMerkleTreePda =
      MerkleTreeConfig.getTransactionMerkleTreePda();

    let leavesPdas: any[] = [];
    let retries = 3;
    while (leavesPdas.length === 0 && retries > 0) {
      if (retries !== 3) await sleep(1000);
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
  } catch (error) {
    console.error("failed at updateMerkleTreeForTest", error.stack);
    throw error;
  }
}
